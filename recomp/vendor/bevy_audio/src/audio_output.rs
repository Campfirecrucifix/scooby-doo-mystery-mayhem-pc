use crate::{
    AudioPlayer, Decodable, DefaultSpatialScale, GlobalVolume, PlaybackMode, PlaybackSettings,
    SpatialAudioSink, SpatialListener,
};
use bevy_asset::{Asset, Assets};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_math::Vec3;
use bevy_transform::prelude::GlobalTransform;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source, SpatialPlayer};
use tracing::warn;

use crate::{AudioSink, AudioSinkPlayback};

/// Used internally to play audio on the current "audio device"
///
/// scooby_recomp patch (P36 R5): public, with device-loss detection (cpal `DeviceNotAvailable` /
/// `StreamInvalidated` via the stream error callback) and [`AudioOutput::reopen`], so the game can
/// reconnect to the default output after the device was unplugged.
#[derive(Resource)]
pub struct AudioOutput {
    stream: Option<MixerDeviceSink>,
    lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AudioOutput {
    fn open(lost: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> Option<MixerDeviceSink> {
        lost.store(false, std::sync::atomic::Ordering::Relaxed);
        let flag = lost.clone();
        let cb = move |e: rodio::cpal::StreamError| {
            use rodio::cpal::StreamError as E;
            if matches!(e, E::DeviceNotAvailable | E::StreamInvalidated) {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            warn!("audio stream error: {e}");
        };
        DeviceSinkBuilder::from_default_device()
            .and_then(|b| b.with_error_callback(cb).open_sink_or_fallback())
            // upstream fallback (other devices / configs), without loss detection
            .or_else(|_| DeviceSinkBuilder::open_default_sink())
            .map(|mut s| {
                s.log_on_drop(false);
                let c = s.config();
                let buf = match c.buffer_size() {
                    rodio::cpal::BufferSize::Fixed(n) => *n as u64,
                    _ => c.sample_rate().get() as u64 / 20,
                };
                UNDERRUN.buffer_frames.store(buf, std::sync::atomic::Ordering::Relaxed);
                UNDERRUN.rate.store(c.sample_rate().get() as u64, std::sync::atomic::Ordering::Relaxed);
                let rate = c.sample_rate().get() as f64;
                s.mixer().add(UnderrunProbe::new(c.channel_count().get(), rate, buf as f64 / rate));
                s
            })
            .ok()
    }
    /// An output stream is open.
    pub fn is_available(&self) -> bool {
        self.stream.is_some()
    }
    /// The open stream reported its device gone (unplugged / invalidated).
    pub fn is_lost(&self) -> bool {
        self.lost.load(std::sync::atomic::Ordering::Relaxed)
    }
    /// Drop the current stream (frees its mixer and every source queued on it).
    pub fn close(&mut self) {
        self.stream = None;
    }
    /// Close and try to open the default output device again; true if a stream is open.
    pub fn reopen(&mut self) -> bool {
        self.stream = None;
        self.stream = Self::open(&self.lost);
        self.stream.is_some()
    }
}

impl Default for AudioOutput {
    fn default() -> Self {
        let lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stream = Self::open(&lost);
        if stream.is_none() {
            warn!("No audio device found.");
        }
        Self { stream, lost }
    }
}

/// scooby_recomp patch (P36 R8): output playout statistics of the current stream, see [`UnderrunProbe`].
pub struct UnderrunStats {
    /// Frames the device has pulled from the mixer.
    pub frames: std::sync::atomic::AtomicU64,
    /// Underruns: the device pulled more than 1.5 buffer periods later than real time (a glitch).
    pub late: std::sync::atomic::AtomicU64,
    /// Worst single lag step in microseconds.
    pub worst_us: std::sync::atomic::AtomicU64,
    /// Device buffer size (frames) and sample rate of the current stream.
    pub buffer_frames: std::sync::atomic::AtomicU64,
    pub rate: std::sync::atomic::AtomicU64,
}
pub static UNDERRUN: UnderrunStats = UnderrunStats {
    frames: std::sync::atomic::AtomicU64::new(0),
    late: std::sync::atomic::AtomicU64::new(0),
    worst_us: std::sync::atomic::AtomicU64::new(0),
    buffer_frames: std::sync::atomic::AtomicU64::new(0),
    rate: std::sync::atomic::AtomicU64::new(0),
};

/// Silent, endless mixer input: every 256 frames it compares the frames the device pulled with the
/// wall clock. A device that misses its deadline falls behind for good (the lag grows by the gap),
/// so a lag step of more than 1.5 buffer periods above the lowest lag seen counts one underrun.
pub struct UnderrunProbe {
    ch: u16,
    rate: f64,
    buf_s: f64,
    n: u64,
    start: Option<std::time::Instant>,
    base: f64,
}

impl UnderrunProbe {
    /// Feed one pull check: `elapsed` s of wall clock after `frames` pulled frames; true = underrun.
    pub fn check(&mut self, elapsed: f64, frames: u64) -> bool {
        let lag = elapsed - frames as f64 / self.rate;
        if lag < self.base {
            self.base = lag;
            return false;
        }
        let step = lag - self.base;
        if step > 1.5 * self.buf_s {
            use std::sync::atomic::Ordering::Relaxed;
            UNDERRUN.late.fetch_add(1, Relaxed);
            UNDERRUN.worst_us.fetch_max((step * 1e6) as u64, Relaxed);
            self.base = lag;
            return true;
        }
        false
    }
    /// `ch` channels at `rate` Hz, device buffer `buf_s` seconds.
    pub fn new(ch: u16, rate: f64, buf_s: f64) -> Self {
        Self { ch, rate, buf_s, n: 0, start: None, base: f64::MAX }
    }
}

impl Iterator for UnderrunProbe {
    type Item = rodio::Sample;
    fn next(&mut self) -> Option<rodio::Sample> {
        self.n += 1;
        if self.n % (256 * self.ch as u64) == 0 {
            let frames = self.n / self.ch as u64;
            UNDERRUN.frames.store(frames, std::sync::atomic::Ordering::Relaxed);
            match self.start {
                // the clock starts at the first check (stream warm-up excluded)
                None => self.start = Some(std::time::Instant::now() - std::time::Duration::from_secs_f64(256.0 / self.rate)),
                Some(t) => {
                    let e = t.elapsed().as_secs_f64();
                    self.check(e, frames);
                }
            }
        }
        Some(0.0)
    }
}

impl Source for UnderrunProbe {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        std::num::NonZero::new(self.ch).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        std::num::NonZero::new(self.rate as u32).unwrap()
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// Marker for internal use, to despawn entities when playback finishes.
#[derive(Component, Default)]
pub struct PlaybackDespawnMarker;

/// Marker for internal use, to remove audio components when playback finishes.
#[derive(Component, Default)]
pub struct PlaybackRemoveMarker;

#[derive(SystemParam)]
pub(crate) struct EarPositions<'w, 's> {
    pub(crate) query: Query<'w, 's, (Entity, &'static GlobalTransform, &'static SpatialListener)>,
}

impl<'w, 's> EarPositions<'w, 's> {
    /// Gets a set of transformed ear positions.
    ///
    /// If there are no listeners, use the default values. If a user has added multiple
    /// listeners for whatever reason, we will return the first value.
    pub(crate) fn get(&self) -> (Vec3, Vec3) {
        let (left_ear, right_ear) = self
            .query
            .iter()
            .next()
            .map(|(_, transform, settings)| {
                (
                    transform.transform_point(settings.left_ear_offset),
                    transform.transform_point(settings.right_ear_offset),
                )
            })
            .unwrap_or_else(|| {
                let settings = SpatialListener::default();
                (settings.left_ear_offset, settings.right_ear_offset)
            });

        (left_ear, right_ear)
    }

    pub(crate) fn multiple_listeners(&self) -> bool {
        self.query.iter().len() > 1
    }
}

/// Plays "queued" audio through the [`AudioOutput`] resource.
///
/// "Queued" audio is any audio entity (with an [`AudioPlayer`] component) that does not have an
/// [`AudioSink`]/[`SpatialAudioSink`] component.
///
/// This system detects such entities, checks if their source asset
/// data is available, and creates/inserts the sink.
pub(crate) fn play_queued_audio_system<Source: Asset + Decodable>(
    audio_output: Res<AudioOutput>,
    audio_sources: Res<Assets<Source>>,
    global_volume: Res<GlobalVolume>,
    query_nonplaying: Query<
        (
            Entity,
            &AudioPlayer<Source>,
            &PlaybackSettings,
            Option<&GlobalTransform>,
        ),
        (Without<AudioSink>, Without<SpatialAudioSink>),
    >,
    ear_positions: EarPositions,
    default_spatial_scale: Res<DefaultSpatialScale>,
    mut commands: Commands,
) where
    f32: rodio::cpal::FromSample<rodio::Sample>,
{
    let Some(stream) = audio_output.stream.as_ref() else {
        // audio output unavailable; cannot play sound
        return;
    };
    let mixer = stream.mixer();

    for (entity, source_handle, settings, maybe_emitter_transform) in &query_nonplaying {
        let Some(audio_source) = audio_sources.get(&source_handle.0) else {
            continue;
        };
        // audio data is available (has loaded), begin playback and insert sink component
        if settings.spatial {
            let (left_ear, right_ear) = ear_positions.get();

            // We can only use one `SpatialListener`. If there are more than that, then
            // the user may have made a mistake.
            if ear_positions.multiple_listeners() {
                warn!(
                    "Multiple SpatialListeners found. Using {}.",
                    ear_positions.query.iter().next().unwrap().0
                );
            }

            let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

            let emitter_translation = if let Some(emitter_transform) = maybe_emitter_transform {
                (emitter_transform.translation() * scale).into()
            } else {
                warn!("Spatial AudioPlayer with no GlobalTransform component. Using zero.");
                Vec3::ZERO.into()
            };

            let sink = SpatialPlayer::connect_new(
                mixer,
                emitter_translation,
                (left_ear * scale).into(),
                (right_ear * scale).into(),
            );

            let decoder = audio_source.decoder();

            match settings.mode {
                PlaybackMode::Loop => match (settings.start_position, settings.duration) {
                    // custom start position and duration
                    (Some(start_position), Some(duration)) => sink.append(
                        decoder
                            .skip_duration(start_position)
                            .take_duration(duration)
                            .repeat_infinite(),
                    ),

                    // custom start position
                    (Some(start_position), None) => {
                        sink.append(decoder.skip_duration(start_position).repeat_infinite());
                    }

                    // custom duration
                    (None, Some(duration)) => {
                        sink.append(decoder.take_duration(duration).repeat_infinite());
                    }

                    // full clip
                    (None, None) => sink.append(decoder.repeat_infinite()),
                },
                PlaybackMode::Once | PlaybackMode::Despawn | PlaybackMode::Remove => {
                    match (settings.start_position, settings.duration) {
                        (Some(start_position), Some(duration)) => sink.append(
                            decoder
                                .skip_duration(start_position)
                                .take_duration(duration),
                        ),

                        (Some(start_position), None) => {
                            sink.append(decoder.skip_duration(start_position));
                        }

                        (None, Some(duration)) => sink.append(decoder.take_duration(duration)),

                        (None, None) => sink.append(decoder),
                    }
                }
            }

            let mut sink = SpatialAudioSink::new(sink);

            if settings.muted {
                sink.mute();
            }

            sink.set_speed(settings.speed);
            sink.set_volume(settings.volume * global_volume.volume);

            if settings.paused {
                sink.pause();
            }

            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => commands.entity(entity).insert(sink),
                PlaybackMode::Despawn => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackDespawnMarker)),
                PlaybackMode::Remove => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackRemoveMarker)),
            };
        } else {
            let sink = Player::connect_new(mixer);
            let decoder = audio_source.decoder();

            match settings.mode {
                PlaybackMode::Loop => match (settings.start_position, settings.duration) {
                    // custom start position and duration
                    (Some(start_position), Some(duration)) => sink.append(
                        decoder
                            .skip_duration(start_position)
                            .take_duration(duration)
                            .repeat_infinite(),
                    ),

                    // custom start position
                    (Some(start_position), None) => {
                        sink.append(decoder.skip_duration(start_position).repeat_infinite());
                    }

                    // custom duration
                    (None, Some(duration)) => {
                        sink.append(decoder.take_duration(duration).repeat_infinite());
                    }

                    // full clip
                    (None, None) => sink.append(decoder.repeat_infinite()),
                },
                PlaybackMode::Once | PlaybackMode::Despawn | PlaybackMode::Remove => {
                    match (settings.start_position, settings.duration) {
                        (Some(start_position), Some(duration)) => sink.append(
                            decoder
                                .skip_duration(start_position)
                                .take_duration(duration),
                        ),

                        (Some(start_position), None) => {
                            sink.append(decoder.skip_duration(start_position));
                        }

                        (None, Some(duration)) => sink.append(decoder.take_duration(duration)),

                        (None, None) => sink.append(decoder),
                    }
                }
            }

            let mut sink = AudioSink::new(sink);

            if settings.muted {
                sink.mute();
            }

            sink.set_speed(settings.speed);
            sink.set_volume(settings.volume * global_volume.volume);

            if settings.paused {
                sink.pause();
            }

            match settings.mode {
                PlaybackMode::Loop | PlaybackMode::Once => commands.entity(entity).insert(sink),
                PlaybackMode::Despawn => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackDespawnMarker)),
                PlaybackMode::Remove => commands
                    .entity(entity)
                    // PERF: insert as bundle to reduce archetype moves
                    .insert((sink, PlaybackRemoveMarker)),
            };
        }
    }
}

pub(crate) fn cleanup_finished_audio<T: Decodable + Asset>(
    mut commands: Commands,
    query_nonspatial_despawn: Query<
        (Entity, &AudioSink),
        (With<PlaybackDespawnMarker>, With<AudioPlayer<T>>),
    >,
    query_spatial_despawn: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackDespawnMarker>, With<AudioPlayer<T>>),
    >,
    query_nonspatial_remove: Query<
        (Entity, &AudioSink),
        (With<PlaybackRemoveMarker>, With<AudioPlayer<T>>),
    >,
    query_spatial_remove: Query<
        (Entity, &SpatialAudioSink),
        (With<PlaybackRemoveMarker>, With<AudioPlayer<T>>),
    >,
) {
    for (entity, sink) in &query_nonspatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_spatial_despawn {
        if sink.sink.empty() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, sink) in &query_nonspatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                AudioPlayer<T>,
                AudioSink,
                PlaybackSettings,
                PlaybackRemoveMarker,
            )>();
        }
    }
    for (entity, sink) in &query_spatial_remove {
        if sink.sink.empty() {
            commands.entity(entity).remove::<(
                AudioPlayer<T>,
                SpatialAudioSink,
                PlaybackSettings,
                PlaybackRemoveMarker,
            )>();
        }
    }
}

/// Run Condition to only play audio if the audio output is available
pub(crate) fn audio_output_available(audio_output: Res<AudioOutput>) -> bool {
    audio_output.stream.is_some()
}

/// Updates spatial audio sinks when emitter positions change.
pub(crate) fn update_emitter_positions(
    mut emitters: Query<
        (&GlobalTransform, &SpatialAudioSink, &PlaybackSettings),
        Or<(Changed<GlobalTransform>, Changed<PlaybackSettings>)>,
    >,
    default_spatial_scale: Res<DefaultSpatialScale>,
) {
    for (transform, sink, settings) in emitters.iter_mut() {
        let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

        let translation = transform.translation() * scale;
        sink.set_emitter_position(translation);
    }
}

/// Updates spatial audio sink ear positions when spatial listeners change.
pub(crate) fn update_listener_positions(
    mut emitters: Query<(&SpatialAudioSink, &PlaybackSettings)>,
    changed_listener: Query<
        (),
        (
            Or<(
                Changed<SpatialListener>,
                Changed<GlobalTransform>,
                Changed<PlaybackSettings>,
            )>,
            With<SpatialListener>,
        ),
    >,
    ear_positions: EarPositions,
    default_spatial_scale: Res<DefaultSpatialScale>,
) {
    if !default_spatial_scale.is_changed() && changed_listener.is_empty() {
        return;
    }

    let (left_ear, right_ear) = ear_positions.get();

    for (sink, settings) in emitters.iter_mut() {
        let scale = settings.spatial_scale.unwrap_or(default_spatial_scale.0).0;

        sink.set_ears_position(left_ear * scale, right_ear * scale);
    }
}
