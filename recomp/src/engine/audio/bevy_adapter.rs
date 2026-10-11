//! Bevy playback of `Voice` descriptors (feature `bevy_audio`; needs `bevy_audio`/`bevy_asset`
//! deps + `[features] bevy_audio = [...]` in Cargo.toml). PCM -> WAV bytes -> `AudioSource`.
//! REPLACEMENT for the SPU2: pan/ADSR are baked into the PCM, pitch -> `PlaybackSettings::speed`.
#![cfg(feature = "bevy_audio")]
use bevy_asset::Assets;
use bevy_audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy_ecs::prelude::*;
use super::adsr::apply_adsr;
use super::mgr::Voice;
pub use super::wav::pcm_to_wav;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceHandle(pub u32);

/// Spawn a Bevy audio entity for `v` from mono PCM.
pub fn spawn_voice(commands: &mut Commands, sources: &mut Assets<AudioSource>, v: &Voice, pcm: &[i16]) -> Entity {
    let env = apply_adsr(pcm, v.sample_rate, v.pitch, v.adsr, None);
    let bytes = pcm_to_wav(&env, v.sample_rate, v.left, v.right);
    let h = sources.add(AudioSource { bytes: bytes.into() });
    let s = if v.looped { PlaybackSettings::LOOP } else { PlaybackSettings::DESPAWN };
    commands.spawn((AudioPlayer(h), s.with_speed(v.pitch).with_volume(Volume::Linear(1.0)), VoiceHandle(v.handle))).id()
}

/// J7: which half of a split voice an audio entity plays (0 = left, 1 = right).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceSide(pub u8);

/// J7: spawns a voice as two entities, a left-only and a right-only stereo WAV of the same
/// enveloped PCM, at unit gain; the host sets each sink's volume to `Voice::left` / `right` every
/// frame ([`update_voice_gains`]). REPLACEMENT for the SPU2 per-voice VOLL/VOLR registers, which
/// SOUND_ driver rewrites while the voice plays (EFNewAudioMgr 3D update: ComputeVolumeAndPan /
/// AdjustSoundToDistance); a single baked stereo clip could not follow the 3D pan.
pub fn spawn_voice_lr(commands: &mut Commands, sources: &mut Assets<AudioSource>, v: &Voice, pcm: &[i16]) -> [Entity; 2] {
    let mut env = apply_adsr(pcm, v.sample_rate, v.pitch, v.adsr, None);
    // A2: SBH start delay (EFNewSound_SP::MustSoftDelay) as leading silence (one-shots only: a
    // looping clip would repeat it).
    if !v.looped && v.delay > 0.0 {
        let n = (v.delay * v.sample_rate as f32 * v.pitch) as usize;
        env.splice(0..0, std::iter::repeat(0).take(n));
    }
    let s = if v.looped { PlaybackSettings::LOOP } else { PlaybackSettings::DESPAWN };
    [(0u8, 1.0f32, 0.0f32, v.left), (1, 0.0, 1.0, v.right)].map(|(side, l, r, g)| {
        let h = sources.add(AudioSource { bytes: pcm_to_wav(&env, v.sample_rate, l, r).into() });
        commands.spawn((AudioPlayer(h), s.with_speed(v.pitch).with_volume(Volume::Linear(g.clamp(0.0, 1.0))), VoiceHandle(v.handle), VoiceSide(side))).id()
    })
}

/// J7: per-frame VOLL/VOLR update of the playing split voices: `gains(handle)` -> (left, right).
pub fn update_voice_gains(sinks: &mut Query<(&VoiceHandle, &VoiceSide, &mut bevy_audio::AudioSink)>, gains: &dyn Fn(u32) -> Option<(f32, f32)>) {
    for (h, side, mut sink) in sinks.iter_mut() {
        if let Some((l, r)) = gains(h.0) {
            let g = if side.0 == 0 { l } else { r };
            bevy_audio::AudioSinkPlayback::set_volume(&mut *sink, Volume::Linear(g.clamp(0.0, 1.0)));
        }
    }
}
