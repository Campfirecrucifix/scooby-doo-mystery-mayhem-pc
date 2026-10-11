//! WP-P3: FMV playback — EFVideoPlayer_SP (1:1) over a replacement of Sony's "MPEG movie player
//! sample" (`MpegPlay`, EXE.1 0x1208e0, Sony SDK/MPEG movie player sample - main.c).
//!
//! Format (MOVIES\*.PSS, measured on the disc): an MPEG-2 program stream. Video = stream 0xe0, MPEG-2
//! MP@ML 720x480 4:2:0 at 29.97 fps (decoded on the PS2 by the IPU). Audio = private stream 1 (0xbd):
//! every payload starts with the 4 bytes `ff a1 00 NN` (NN = mono track number); the first packet of
//! each track carries a `SShd` header (size 0x18: type 0x10 = SPU ADPCM, rate 48000, 1 channel, ...)
//! and an `SSbd` header (body size), then raw 16-byte SPU ADPCM blocks. The EP movies have 6 mono
//! tracks = 3 stereo pairs (left = 2*track, right = 2*track+1, MpegPlay `_leftVoiceTrack`), the track
//! being ENGINE.INI `VideoAudioTrack_<lang>` (EN 0, FR 1).
//!
//! REPLACEMENT (not 1:1): the IPU/videodec/audiodec/IOP threads are replaced by one decoder thread
//! (pure-Rust `na_mpeg2_decoder` for the video, `audio::vag::decode_adpcm` for the SPU ADPCM) feeding
//! RGBA frames through a bounded channel; the blocking `readMpeg` loop becomes [`MpegPlay::Update`],
//! called once per frame by the host. Kept 1:1: skip rules (mask, rising edge, >60 decoded pictures,
//! LEGAL.PSS unskippable, MOVIES\DEMOS\ any button), the stream volume, the end condition (video
//! flushed and audio finished), the `_MpgSkip` result and EFVideoPlayer_SP::Play's DEMO.PSS quirk.
//! The original has no fades and no looping: the GS is cleared to black before and after.
#![allow(non_snake_case, non_camel_case_types)]
use std::collections::BTreeMap;
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError};

/// One decoded picture (RGBA8, BT.601 limited range like the IPU CSC).
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A demultiplexed PSS file.
#[derive(Debug, Default)]
pub struct PssFile {
    /// MPEG-2 video elementary stream (stream id 0xe0)
    pub video_es: Vec<u8>,
    /// private stream 1 tracks: track number -> raw SPU ADPCM (SShd/SSbd headers removed)
    pub tracks: BTreeMap<u8, Vec<u8>>,
    /// SShd per track: (type, rate)
    pub track_fmt: BTreeMap<u8, (u32, u32)>,
}

fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Replacement for sceMpegDemuxPss(Ring): splits the program stream into video ES + audio tracks.
pub fn demux_pss(d: &[u8]) -> PssFile {
    let mut out = PssFile::default();
    let mut i = 0;
    while i + 6 <= d.len() {
        if d[i..i + 3] != [0, 0, 1] {
            i += 1;
            continue;
        }
        let sid = d[i + 3];
        if sid == 0xba {
            // MPEG-2 pack header: 14 bytes + stuffing
            i += 14 + (d.get(i + 13).copied().unwrap_or(0) & 7) as usize;
            continue;
        }
        if sid == 0xb9 {
            break;
        }
        let len = u16::from_be_bytes([d[i + 4], d[i + 5]]) as usize;
        let end = (i + 6 + len).min(d.len());
        if (sid == 0xe0 || sid == 0xbd) && i + 9 <= end {
            let p = (i + 9 + d[i + 8] as usize).min(end);
            let pl = &d[p..end];
            if sid == 0xe0 {
                out.video_es.extend_from_slice(pl);
            } else if pl.len() >= 4 {
                let tr = pl[3];
                let mut a = &pl[4..];
                if a.len() >= 8 && &a[..4] == b"SShd" {
                    let hs = le32(a, 4) as usize;
                    if a.len() >= 8 + hs && hs >= 8 {
                        out.track_fmt.insert(tr, (le32(a, 8), le32(a, 12)));
                    }
                    a = &a[(8 + hs).min(a.len())..];
                    if a.len() >= 8 && &a[..4] == b"SSbd" {
                        a = &a[8..];
                    }
                }
                out.tracks.entry(tr).or_default().extend_from_slice(a);
            }
        }
        i = end;
    }
    out
}

/// MPEG-2 sequence header frame_rate_code -> fps (29.97 default).
pub fn video_fps(es: &[u8]) -> f32 {
    let p = es.windows(4).position(|w| w == [0, 0, 1, 0xb3]);
    match p.and_then(|p| es.get(p + 7)).map(|b| b & 0xf) {
        Some(1) => 24000.0 / 1001.0,
        Some(2) => 24.0,
        Some(3) => 25.0,
        Some(5) => 30.0,
        Some(6) => 50.0,
        Some(7) => 60000.0 / 1001.0,
        Some(8) => 60.0,
        _ => 30000.0 / 1001.0,
    }
}

/// Stereo PCM of audio track `track` (left = mono track 2*track, right = 2*track+1), as
/// MpegPlay's `_leftVoiceTrack = param_6 * 2; rightVoiceTrack = _leftVoiceTrack + 1`.
/// Returns (rate, interleaved L/R i16). None when the file has no such ADPCM tracks.
pub fn decode_audio(pss: &PssFile, track: i32) -> Option<(u32, Vec<i16>)> {
    let l = (track * 2) as u8;
    let (ty, rate) = *pss.track_fmt.get(&l)?;
    if ty != 0x10 {
        log::warn!("PSS audio type {ty:#x} not SPU ADPCM: no audio");
        return None;
    }
    let dl = crate::engine::audio::vag::decode_adpcm(pss.tracks.get(&l)?, false).samples;
    let dr = pss.tracks.get(&(l + 1)).map(|r| crate::engine::audio::vag::decode_adpcm(r, false).samples).unwrap_or_else(|| dl.clone());
    let n = dl.len().max(dr.len());
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        out.push(dl.get(i).copied().unwrap_or(0));
        out.push(dr.get(i).copied().unwrap_or(0));
    }
    Some((rate, out))
}

/// Message from the decoder thread.
enum Msg {
    /// strFileOpen failed (`_bCantFindPSS`)
    CantFind,
    /// stream opened: fps, stereo audio
    Header(f32, Option<(u32, Vec<i16>)>),
    Frame(VideoFrame),
}

/// libpad button word as the engine stores it (b2 | b3 << 8) -> MpegPlay's
/// `CONCAT11(b2, b3)` = (b2 << 8 | b3): the skip masks are in that byte-swapped layout
/// (0x40 = CROSS, 0x800 = START).
pub fn pad_to_mpeg(buttons: u16) -> u16 {
    buttons.swap_bytes()
}

/// Replacement state of the blocking `MpegPlay` call (one per movie).
pub struct MpegPlay {
    /// Mutex only to make the state Sync (Bevy resource); never contended
    rx: std::sync::Mutex<Receiver<Msg>>,
    /// param_4 skip button mask (MpegPlay pad layout)
    pub skip_mask: u16,
    /// `_gfStreamVolume` (param_1): EFNewAudioMgr +0x1132c (video volume); 0 for the boot movie
    pub volume: f32,
    /// param_7 `pal`: skip allowed after 50 (PAL) / 60 (NTSC) decoded pictures
    pub pal: bool,
    /// `_MpgSkip`
    pub MpgSkip: bool,
    /// sceMpeg +8 frameCount: pictures taken from the decoder
    pub frame_count: u32,
    /// previous `_controller_val` (uVar7)
    prev_pad: u16,
    pub fps: f32,
    /// stereo PCM (rate, L/R interleaved), taken by the host to start the audio voice
    pub audio: Option<(u32, Vec<i16>)>,
    audio_secs: f32,
    /// display started (`startDisplay`: video buffer full + audio preset)
    pub started: bool,
    /// seconds since startDisplay
    pub t: f32,
    /// picture on screen (`new_frame` set when it changed since the last `take_new_frame`)
    pub current: Option<VideoFrame>,
    new_frame: bool,
    pending: Option<VideoFrame>,
    video_done: bool,
    pub done: bool,
    /// host option: wait for the decoder instead of dropping behind (deterministic headless runs)
    pub blocking: bool,
}

impl MpegPlay {
    /// `MpegPlay(volume, dir, name, skip_mask, 0, audio_track, pal)`. `read` runs on the decoder
    /// thread and returns the file bytes (None = `Can't Open file`).
    pub fn new(volume: f32, name: &str, skip_mask: u16, audio_track: i32, pal: bool, read: impl FnOnce() -> Option<Vec<u8>> + Send + 'static) -> Self {
        // quirk: only the exact string "MOVIES\LEGAL.PSS" is unskippable (the boot path passes
        // "\MOVIES\..." with a leading backslash)
        let skip_mask = if name == "MOVIES\\LEGAL.PSS" { 0 } else { skip_mask };
        let (tx, rx) = sync_channel::<Msg>(6);
        std::thread::spawn(move || {
            let Some(d) = read() else {
                let _ = tx.send(Msg::CantFind);
                return;
            };
            let pss = demux_pss(&d);
            drop(d);
            let fps = video_fps(&pss.video_es);
            if tx.send(Msg::Header(fps, decode_audio(&pss, audio_track))).is_err() {
                return;
            }
            let mut dec = na_mpeg2_decoder::Decoder::new();
            let conv = |f: &na_mpeg2_decoder::Frame| {
                let mut rgba = vec![0u8; f.width * f.height * 4];
                na_mpeg2_decoder::frame_to_rgba_bt601_limited(f, &mut rgba);
                Msg::Frame(VideoFrame { width: f.width as u32, height: f.height as u32, rgba })
            };
            for ch in pss.video_es.chunks(0x8000) {
                let Ok(fr) = dec.decode(ch, None) else { continue };
                for f in &fr {
                    if tx.send(conv(f)).is_err() {
                        return; // aborted (videoDecAbort)
                    }
                }
            }
            if let Ok(fr) = dec.flush() {
                for f in &fr {
                    if tx.send(conv(f)).is_err() {
                        return;
                    }
                }
            }
        });
        MpegPlay {
            rx: std::sync::Mutex::new(rx),
            skip_mask,
            volume,
            pal,
            MpgSkip: false,
            frame_count: 0,
            prev_pad: 0,
            fps: 30000.0 / 1001.0,
            audio: None,
            audio_secs: 0.0,
            started: false,
            t: 0.0,
            current: None,
            new_frame: false,
            pending: None,
            video_done: false,
            done: false,
            blocking: false,
        }
    }

    fn pull(&mut self) -> Option<VideoFrame> {
        if let Some(f) = self.pending.take() {
            return Some(f);
        }
        let rx = self.rx.get_mut().unwrap();
        let r = if self.blocking { rx.recv().map_err(|_| TryRecvError::Disconnected) } else { rx.try_recv() };
        match r {
            Ok(Msg::Frame(f)) => {
                self.frame_count += 1;
                Some(f)
            }
            Ok(Msg::CantFind) => {
                self.done = true;
                None
            }
            Ok(Msg::Header(fps, a)) => {
                self.fps = fps;
                self.audio_secs = a.as_ref().map(|(r, s)| s.len() as f32 / 2.0 / *r as f32).unwrap_or(0.0);
                self.audio = a;
                self.pull()
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.video_done = true;
                None
            }
        }
    }

    /// One host frame of `readMpeg` (dt seconds, `buttons` = engine pad word of port 0).
    /// Returns false once MpegPlay has returned (`done`; result in `MpgSkip`).
    pub fn Update(&mut self, dt: f32, buttons: u16) -> bool {
        if self.done {
            return false;
        }
        // skip test: rising edge of a mask button, only after 60 (NTSC) / 50 (PAL) pictures
        let pad = pad_to_mpeg(buttons);
        let min = if self.pal { 0x32 } else { 0x3c };
        if self.skip_mask & pad & (self.prev_pad ^ pad) != 0 && self.frame_count > min {
            self.MpgSkip = true; // videoDecAbort; audio reset, endDisplay
            self.done = true;
            return false;
        }
        self.prev_pad = pad;
        if !self.started {
            // startDisplay once the first picture (and the audio header) is ready
            if let Some(f) = self.pull() {
                self.current = Some(f);
                self.new_frame = true;
                self.started = true;
            } else if self.video_done || self.done {
                self.done = true;
                return false;
            }
            return true;
        }
        self.t += dt;
        let target = (self.t * self.fps) as u32 + 1;
        while self.frame_count < target && !self.video_done && !self.done {
            match self.pull() {
                Some(f) => {
                    self.current = Some(f);
                    self.new_frame = true;
                }
                None => break,
            }
        }
        // end: video flushed and (`while (!audioDone && !_MpgSkip)`) the audio played out
        if self.video_done && self.t >= self.audio_secs {
            self.done = true;
        }
        !self.done
    }

    /// The picture changed since the last call (upload it).
    pub fn take_new_frame(&mut self) -> Option<&VideoFrame> {
        if std::mem::take(&mut self.new_frame) { self.current.as_ref() } else { None }
    }
}

/// EFVideoPlayer_SP (singleton EFVideoPlayer::GetInstance).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFVideoPlayer_SP {
    /// this+4: movie name without extension (APIgame::LoadLevel strips one leading '\')
    pub name: String,
    /// this+0x108: audio track (ENGINE.INI VideoAudioTrack_<lang>)
    pub audio_track: i32,
}

/// ENGINE.INI `VideoSkipButton` index -> MpegPlay pad bit (EFVideoPlayer_SP::Play switch).
pub fn skip_button_bit(v: i32) -> u16 {
    match v {
        0 => 0x40,
        1 => 0x20,
        2 => 0x80,
        3 => 0x10,
        4 => 4,
        5 => 8,
        6 => 1,
        7 => 2,
        10 => 0x100,
        11 => 0x800,
        12 => 0x1000,
        13 => 0x4000,
        14 => 0x8000,
        15 => 0x2000,
        _ => 0,
    }
}

impl EFVideoPlayer_SP {
    /// EFVideoPlayer_SP::Play, first half: builds the MpegPlay call. (The 0.5 s VSync wait with
    /// EFGraphicsMgr paused before the call is a black-screen pause; the host shows black while
    /// the decoder pre-rolls.) `video_skip_button` = ENGINE.INI VideoSkipButton (default 0),
    /// `volume` = EFNewAudioMgr +0x1132c, `pal` = EFGraphicsMgr +0x44 == 1.
    pub fn Play(&self, video_skip_button: i32, volume: f32, pal: bool, read: impl FnOnce(&str) -> Option<Vec<u8>> + Send + 'static) -> MpegPlay {
        let file = format!("{}.PSS", self.name);
        let mut mask = skip_button_bit(video_skip_button) | 0x800; // + START always
        if self.name.starts_with("MOVIES\\DEMOS\\") {
            mask = 0xffff; // demo movies: any button
        }
        let f2 = file.clone();
        MpegPlay::new(volume, &file, mask, self.audio_track, pal, move || read(&f2))
    }

    /// EFVideoPlayer_SP::Play, after MpegPlay returned `skipped`: when the movie was "\DEMO.PSS"
    /// and was skipped, the pending pack (EFRessourcesMgr +0x54) becomes "\LEVEL\TITLE.P2R".
    /// quirk: `this+4` never starts with '\' (LoadLevel strips it), so this never fires.
    pub fn PlayEnd(&self, skipped: bool, pack: &mut String) {
        if self.name == "\\DEMO.PSS" && skipped && pack != "\\LEVEL\\TITLE.P2R" {
            *pack = "\\LEVEL\\TITLE.P2R".to_string();
        }
    }
}

/// main.cpp boot movie (not demo mode): ENGINE.INI BootUpPSSName (default "\MOVIES\LEGAL.PSS"),
/// upper-cased, played with volume 0 (quirk: `MpegPlay(0, ...)` -> silent), no skip button,
/// track 0, `pal` = DisplayMode bool.
pub fn BootMovie(boot_pss_name: Option<&str>, pal: bool, read: impl FnOnce(&str) -> Option<Vec<u8>> + Send + 'static) -> MpegPlay {
    let name = boot_pss_name.unwrap_or("\\MOVIES\\LEGAL.PSS").to_ascii_uppercase();
    let n2 = name.clone();
    MpegPlay::new(0.0, &name, 0, 0, pal, move || read(&n2))
}

/// "MOVIES\EP01\EP01A.PSS" / "\MOVIES\BOOT.PSS" -> ISO path "MOVIES/EP01/EP01A.PSS".
pub fn iso_path(name: &str) -> String {
    name.replace('\\', "/").trim_start_matches('/').to_string()
}
