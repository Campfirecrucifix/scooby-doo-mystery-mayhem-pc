//! P36 R5: audio output device loss / reconnect policy (pure; the host owns the stream).
//! Not in the original (the PS2 SPU2 cannot disappear): on PC the default output device can be
//! unplugged or invalidated mid-game. The host closes the dead stream (rodio queues new sources on
//! its mixer forever otherwise: per-key-on leak), keeps the game running silently, retries the
//! default device every [`RETRY_SECS`] and, once reopened, re-keys the live voices.

/// seconds between reopen attempts while no stream is open
pub const RETRY_SECS: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    None,
    /// drop the stream and every voice playing on it
    Close,
    /// try to open the default output device
    Reopen,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceWatch {
    retry_at: f64,
    /// reopen attempts / successful reconnects (logs + tests)
    pub attempts: u32,
    pub reconnects: u32,
}

impl DeviceWatch {
    /// `available`: a stream is open; `lost`: its error callback reported the device gone
    /// (or the `GAME_AUDIO_FAKE_LOSS` test hook fired); `now`: wall-clock seconds.
    pub fn step(&mut self, now: f64, available: bool, lost: bool) -> DeviceAction {
        if available {
            if lost {
                self.retry_at = now + RETRY_SECS / 2.0;
                return DeviceAction::Close;
            }
            return DeviceAction::None;
        }
        if now >= self.retry_at {
            self.retry_at = now + RETRY_SECS;
            self.attempts += 1;
            return DeviceAction::Reopen;
        }
        DeviceAction::None
    }
    /// result of a [`DeviceAction::Reopen`]
    pub fn reopened(&mut self, ok: bool) {
        if ok {
            self.reconnects += 1;
        }
    }
}

/// P34/P36 PC volume category of a voice (0 = Music, 1 = Sound effects, 2 = Voice / movies):
/// original Options "movies" group 0x300 (CS*_*.INT cut-scene streams) = Voice, other interleaved
/// stream = Music, other stream = Voice, bank = SFX. `group` = TSE word 0x28 of the voice's event.
pub fn pc_category(group: Option<u32>, stream: bool, interleaved: bool) -> usize {
    match () {
        _ if group.is_some_and(|g| g & 0x300 != 0) => 2,
        _ if stream && interleaved => 0,
        _ if stream => 2,
        _ => 1,
    }
}

/// P36 R6: FMV soundtrack after an output (re)open. `spawned` = reconnect generation the movie's audio
/// was started on (None = not yet), `cur` = current generation, `elapsed` = seconds since the movie's
/// audio clock started (the A/V master), `len` = soundtrack length. Some(offset) = (re)start the
/// soundtrack now at `offset` s (~0 on the first start; the wall-clock position after a reconnect so the
/// sound stays in sync with the picture); None = nothing to do (already playing on this stream, or
/// the soundtrack has already ended).
pub fn movie_audio_restart(spawned: Option<u32>, cur: u32, available: bool, elapsed: f64, len: f64) -> Option<f64> {
    match spawned {
        Some(g) if g == cur => None,
        _ if !available || elapsed >= len => None,
        _ => Some(elapsed.max(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P36 R8: playout underrun counter (vendored bevy_audio UnderrunProbe, fed by the device pulls):
    /// jitter up to one buffer is not an underrun; a 100 ms stall counts once.
    #[cfg(feature = "bevy_audio")]
    #[test]
    fn p36_underrun_probe() {
        let mut p = bevy_audio::UnderrunProbe::new(2, 48000.0, 0.04);
        for k in 0..2000u64 {
            let f = k * 480;
            let jitter = if k % 7 == 0 { 0.035 } else { 0.0 };
            assert!(!p.check(f as f64 / 48000.0 + jitter, f));
        }
        let f = 2000 * 480;
        assert!(p.check(f as f64 / 48000.0 + 0.1, f));
        for k in 2001..2100u64 {
            let f = k * 480;
            assert!(!p.check(f as f64 / 48000.0 + 0.1, f));
        }
    }

    /// P36 R7: the PC volumes (settings.toml [audio], P34) multiply: master x category, mute = 0. The host
    /// (game.rs play_audio / FMV) multiplies this by the voice's own level, which already carries the
    /// original Options slider of its group (0xa8 music, 0x17 FX, 0x300 + video volume = movies;
    /// p36_option_slider_groups), so no setting is ignored.
    #[test]
    fn p36_pc_gain_multiplies() {
        use crate::engine::settings::Audio;
        let mut a = Audio { master: 0.5, music: 0.8, sfx: 0.4, voice: 0.0, mute: false };
        assert!((a.gain(a.music) - 0.4).abs() < 1e-6);
        assert!((a.gain(a.sfx) - 0.2).abs() < 1e-6);
        assert_eq!(a.gain(a.voice), 0.0);
        assert!((0.5 * a.gain(a.music) - 0.2).abs() < 1e-6); // game slider half x PC gain
        a.master = 1.0;
        assert_eq!(a.gain(1.0), 1.0);
        a.mute = true;
        assert_eq!(a.gain(a.music), 0.0);
        assert_eq!(Audio::default().gain(1.0), 1.0);
    }

    #[test]
    fn p36_movie_audio_restart() {
        // first start on a healthy stream: from 0
        assert_eq!(movie_audio_restart(None, 0, true, 0.0, 30.0), Some(0.0));
        // movie starts while no device: wait (no queued source that would start late at 0)
        assert_eq!(movie_audio_restart(None, 0, false, 3.0, 30.0), None);
        // ... device appears at 5 s: start at the picture's position
        assert_eq!(movie_audio_restart(None, 1, true, 5.0, 30.0), Some(5.0));
        // playing on the current stream: nothing
        assert_eq!(movie_audio_restart(Some(1), 1, true, 12.0, 30.0), None);
        // lost (stream closed) then reopened at 12.5 s: resume at 12.5 s
        assert_eq!(movie_audio_restart(Some(1), 1, false, 12.0, 30.0), None);
        assert_eq!(movie_audio_restart(Some(1), 2, true, 12.5, 30.0), Some(12.5));
        // reopened after the soundtrack's end: nothing to replay
        assert_eq!(movie_audio_restart(Some(1), 2, true, 31.0, 30.0), None);
    }

    #[test]
    fn p36_device_loss_reconnect() {
        let mut w = DeviceWatch::default();
        // healthy stream: nothing to do
        assert_eq!(w.step(0.0, true, false), DeviceAction::None);
        // device lost at t=10: close at once, first retry after RETRY/2
        assert_eq!(w.step(10.0, true, true), DeviceAction::Close);
        assert_eq!(w.step(10.5, false, false), DeviceAction::None);
        assert_eq!(w.step(11.0, false, false), DeviceAction::Reopen);
        w.reopened(false);
        // still unplugged: retries every RETRY_SECS, never a busy loop
        let reopens = (0..600).filter(|i| w.step(11.0 + *i as f64 / 60.0, false, false) == DeviceAction::Reopen).count();
        assert_eq!(reopens, 4, "10 s more without a device = 4 attempts (13, 15, 17, 19 s)");
        // plugged back in
        assert_eq!(w.step(30.0, false, false), DeviceAction::Reopen);
        w.reopened(true);
        assert_eq!(w.step(30.1, true, false), DeviceAction::None);
        assert_eq!((w.attempts, w.reconnects), (6, 1));
    }
}
