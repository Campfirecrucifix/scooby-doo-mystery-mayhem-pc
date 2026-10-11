//! WP-S3: APIgame (EFApi_Game.cpp 0x001ed280..), APIqa (EFApi_QA.cpp), APIconsole
//! (EFApi_Console.cpp) host side, plus the part of EFEngineController / EFRessourcesMgr /
//! EFIniFile those wrappers touch.
//!
//! `HostGame` lives in `ParticleShared::game` (the runtime's shared state reached by the
//! `RuntimeHost`). The binary reads the requests the logic made through it:
//! [`HostGame::take_level_request`] (EFEngineController::Run's `flags & 8` -> LoadLevel) and
//! [`HostGame::quit_requested`] (`flags & 4`).
#![allow(non_snake_case)]

use crate::engine::waypoint::DTWpGrpMgr;
use crate::engine::{Engine, Mem, Ptr};

/// EFIniFile replacement: ENGINE.INI (ISO LBA 6537, 382 bytes) key = value lines. Defaults are
/// the shipped file's values so the host answers like the PS2 without the ISO.
#[derive(Debug, Clone)]
pub struct IniFile(pub Vec<(String, String)>);

/// The shipped ENGINE.INI
pub const ENGINE_INI: &str = "MaxFPS = 30\nDefaultLanguage = EN\nStartingLevelPath = LOADER\nStartingLevelToLoad = LOADER.P2R\nProgressBarTopLeftX = 482\nProgressBarTopLeftY = 390\nProgressBarWidth = 128\nProgressBarHeight = 32\nProductCode = BASLUS-20701\nDisplayMode = NTSC\nBootUpPSSName = \\MOVIES\\BOOT.PSS\nUseConsoleLanguage = TRUE\nAvailableLanguages = EN-FR\nVideoAudioTrack_EN = 0\nVideoAudioTrack_FR = 1\n";

impl Default for IniFile {
    fn default() -> Self {
        IniFile::Load(ENGINE_INI.as_bytes())
    }
}

impl IniFile {
    /// parse `key = value` lines (replacement for EFIniFile's reader; case-insensitive keys)
    pub fn Load(data: &[u8]) -> Self {
        let s = String::from_utf8_lossy(data);
        IniFile(
            s.lines()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .collect(),
        )
    }
    /// EFIniFile::GetIniString (None = key missing: the C++ leaves the buffer untouched)
    pub fn GetIniString(&self, key: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
    /// EFIniFile::GetIniInteger (missing / unparsable key: `out` untouched)
    pub fn GetIniInteger(&self, key: &str, out: &mut i32) {
        if let Some(v) = self.GetIniString(key).and_then(|v| v.parse().ok()) {
            *out = v;
        }
    }
}

/// What APIgame::LoadLevel queued for EFEngineController::LoadLevel (run by the binary).
#[derive(Debug, Clone, PartialEq)]
pub struct LevelRequest {
    /// EFEngineController+0x4ce: level (directory) name
    pub level: String,
    /// EFRessourcesMgr+0x54: pack file name (with extension)
    pub pack: String,
    /// flags & 0x10: EFVideoPlayer+4 video to play first, +0x108 audio track
    pub video: Option<(String, i32)>,
    /// EFRessourcesMgr::RequestExternalFileLoad names (stleveltransitionparams flag 4)
    pub external_files: Vec<String>,
}

/// SOUND_ driver karaoke state (gfTVOutputVolume 0x343960 etc., values from EXE.1 .data).
/// Replaced: there is no IOP / USB microphone; the IOP command words the driver would send are
/// appended to `iop_commands` (StartCommand code, data...) so the behaviour is observable.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundKaraoke {
    pub tv_output_volume: f32,
    pub mic_recording_volume: f32,
    pub headset_playback_volume: f32,
    /// g_fAverageKaraokeVolume / g_fMaximumKaraokeVolume (updated from IOP mic data; none here)
    pub average: f32,
    pub maximum: f32,
    pub iop_commands: Vec<Vec<i32>>,
}

impl Default for SoundKaraoke {
    fn default() -> Self {
        SoundKaraoke { tv_output_volume: 1.0, mic_recording_volume: 0.01, headset_playback_volume: 0.5, average: 0.0, maximum: 0.0, iop_commands: Vec::new() }
    }
}

/// (float)(int) truncation as the EE does it for the SOUND_AddData shorts (values here are < 2^31)
fn f2s(f: f32) -> i32 {
    (f as i32) as i16 as i32
}

impl SoundKaraoke {
    fn clamp01(v: f32) -> f32 {
        if v < 0.0 {
            0.0
        } else if 1.0 < v {
            1.0
        } else {
            v
        }
    }
    /// SOUND_OpenUSBMicrophone 0x0025ee10: cmd 0x67 (port, mic vol*1000, headset vol*100)
    pub fn SOUND_OpenUSBMicrophone(&mut self, port: u16) {
        self.iop_commands.push(vec![0x67, port as i32, f2s(self.mic_recording_volume * 1000.0), f2s(self.headset_playback_volume * 100.0)]);
    }
    /// SOUND_CloseUSBMicrophone 0x0025ef10: cmd 0x68 (port)
    pub fn SOUND_CloseUSBMicrophone(&mut self, port: u16) {
        self.iop_commands.push(vec![0x68, port as i32]);
    }
    /// SOUND_StartRecording 0x0025ef50: cmd 0x69 (port, tv vol*32767)
    pub fn SOUND_StartRecording(&mut self, port: u16) {
        self.iop_commands.push(vec![0x69, port as i32, f2s(self.tv_output_volume * 32767.0)]);
    }
    /// SOUND_StopRecording 0x0025eff0: cmd 0x6a (port)
    pub fn SOUND_StopRecording(&mut self, port: u16) {
        self.iop_commands.push(vec![0x6a, port as i32]);
    }
    /// SOUND_SetTVOutputVolume 0x0025ecf0 (clamped to [0,1]; port ignored)
    pub fn SOUND_SetTVOutputVolume(&mut self, v: f32) {
        self.tv_output_volume = Self::clamp01(v);
    }
    /// SOUND_SetMicrophoneRecordingVolume 0x0025ed50
    pub fn SOUND_SetMicrophoneRecordingVolume(&mut self, v: f32) {
        self.mic_recording_volume = Self::clamp01(v);
    }
    /// SOUND_SetHeadsetPlaybackVolume 0x0025edb0
    pub fn SOUND_SetHeadsetPlaybackVolume(&mut self, v: f32) {
        self.headset_playback_volume = Self::clamp01(v);
    }
}

/// Host state behind the WP-S3 APIs.
#[derive(Debug, Clone)]
pub struct HostGame {
    /// EFEngineController+0x29c (ctor: 2). 4 = quit, 8 = load level pending, 0x10 = video first
    pub flags: u32,
    /// EFEngineController+0x2d0 game language (ctor: boot params +0x400 = ENGINE.INI DefaultLanguage)
    pub language: String,
    /// Mem copy of +0x2d0 that GetGameLanguage hands out (0 = not allocated yet)
    pub language_ptr: Ptr,
    /// EFEngineController+0x4ce level name
    pub level: String,
    /// EFRessourcesMgr+0x54 pack name (None = NULL)
    pub pack: Option<String>,
    /// EFRessourcesMgr+0x5c pack extension (the starting level is LOADER.P2R)
    pub pack_ext: String,
    pub video: Option<(String, i32)>,
    pub external_files: Vec<String>,
    /// EFEngineController+0xaac..0xab8 GetDemoLaunchData words (ctor: gto_inactive / gto_gameplay
    /// 0xffff -> 0xffffffff; gplaymode, glanguage = 0: no demo launch)
    pub demo_launch: [u32; 4],
    /// Mem block GetDemoLaunchData copies the words to (struct return, see GetDemoLaunchData)
    pub demo_launch_ptr: Ptr,
    pub ini: IniFile,
    /// DTWpGrpMgr 0x3441c0 as seen by APIextra (EngineRuntime::LoadLevel copies the level's groups)
    pub waypoints: DTWpGrpMgr,
    pub karaoke: SoundKaraoke,
    /// gui32EnableReset (APILoadSave::EnableReset)
    pub enable_reset: u32,
}

impl Default for HostGame {
    fn default() -> Self {
        let ini = IniFile::default();
        HostGame {
            flags: 2,
            language: ini.GetIniString("DefaultLanguage").unwrap_or("EN").to_string(),
            language_ptr: 0,
            level: String::new(),
            pack: None,
            pack_ext: ".P2R".into(),
            video: None,
            external_files: Vec::new(),
            demo_launch: [0xffff_ffff, 0xffff_ffff, 0, 0],
            demo_launch_ptr: 0,
            ini,
            waypoints: DTWpGrpMgr::default(),
            karaoke: SoundKaraoke::default(),
            enable_reset: 0,
        }
    }
}

/// strcpy into Mem
fn wstr(m: &mut Mem, p: Ptr, s: &str) {
    for (i, b) in s.bytes().chain(std::iter::once(0)).enumerate() {
        m.w8(p + i as u32, b);
    }
}

impl HostGame {
    /// the game language the binary should load packs / sound banks with
    pub fn set_language(&mut self, m: Option<&mut Mem>, lang: &str) {
        self.language = lang.to_string();
        if let Some(m) = m {
            if self.language_ptr != 0 {
                wstr(m, self.language_ptr, lang);
            }
        }
    }
    /// EFEngineController::Run `flags & 8` -> EFEngineController::LoadLevel: hands the pending
    /// request to the binary and clears 8 (and 0x10 once the video is taken).
    pub fn take_level_request(&mut self) -> Option<LevelRequest> {
        if self.flags & 8 == 0 {
            return None;
        }
        self.flags &= !0x18;
        Some(LevelRequest {
            level: self.level.clone(),
            pack: self.pack.clone().unwrap_or_default(),
            video: self.video.take(),
            external_files: std::mem::take(&mut self.external_files),
        })
    }
    /// EFEngineController::Run loop condition (`flags & 4` ends the game)
    pub fn quit_requested(&self) -> bool {
        self.flags & 4 != 0
    }

    /// LoadLevel 0x001ed400 (char *level, char *pack, stleveltransitionparams *)
    /// Returns the C++ return register (last strcpy/... result); callers ignore it -> 0 here
    /// (the +0x4ce buffer has no Mem address).
    pub fn LoadLevel(&mut self, m: &Mem, level: Ptr, pack: Ptr, params: Ptr) -> Ptr {
        log::debug!("LOAD LEVEL : {}", m.cstr_at(level));
        let l = m.cstr_at(level);
        if !l.is_empty() {
            self.level = l;
        }
        self.flags |= 8;
        // quirk: the C++ compares the stored pack pointer with `pack` (never equal to a module
        // string), so the name is always re-copied
        let p = m.cstr_at(pack);
        let has_ext = match p.rfind('.') {
            Some(i) => {
                self.pack_ext = p[i..].to_string();
                true
            }
            None => false,
        };
        self.pack = Some(if has_ext { p } else { format!("{p}{}", self.pack_ext) });
        if params != 0 && m.r32(params) & 2 != 0 {
            self.flags |= 0x10;
            let v = m.cstr_at(m.r32(params + 4));
            let v = v.strip_prefix('\\').map(str::to_string).unwrap_or(v);
            let mut track = 0;
            self.ini.GetIniInteger(&format!("VideoAudioTrack_{}", self.language), &mut track);
            self.video = Some((v, track));
        }
        if params != 0 && m.r32(params) & 4 != 0 {
            for i in 0..m.r32(params + 8) {
                self.external_files.push(m.cstr_at(params + 0xc + i * 200));
            }
        }
        0
    }
    /// SetGameLanguage 0x001ed6d0: strcpy(+0x2d0) unless empty; returns the copy (NULL if empty)
    pub fn SetGameLanguage(&mut self, m: &mut Mem, s: Ptr) -> Ptr {
        let l = m.cstr_at(s);
        if l.is_empty() {
            return 0;
        }
        self.language = l;
        self.GetGameLanguage(m)
    }
    /// GetGameLanguage 0x001ed710: &EFEngineController+0x2d0 (0x1fe-byte buffer)
    pub fn GetGameLanguage(&mut self, m: &mut Mem) -> Ptr {
        if self.language_ptr == 0 {
            self.language_ptr = m.malloc(0x1fe);
        }
        wstr(m, self.language_ptr, &self.language);
        self.language_ptr
    }
    /// QuitGame 0x001ed800 / APIqa CloseGame 0x00222590: flags |= 4
    pub fn QuitGame(&mut self) -> u32 {
        self.flags |= 4;
        0
    }
    /// GetDemoLaunchData 0x001ed820 (struct return): the 4 words in a Mem block (see the
    /// engine-side `GetDemoLaunchData`)
    pub fn GetDemoLaunchData(&mut self, m: &mut Mem) -> Ptr {
        if self.demo_launch_ptr == 0 {
            self.demo_launch_ptr = m.malloc(0x10);
        }
        for (i, w) in self.demo_launch.iter().enumerate() {
            m.w32(self.demo_launch_ptr + 4 * i as u32, *w);
        }
        self.demo_launch_ptr
    }
    /// GetSystemRegion 0x001ed850: ini DisplayMode NTSC -> 1, PAL -> 4, else 0
    pub fn GetSystemRegion(&self) -> u32 {
        match self.ini.GetIniString("DisplayMode") {
            Some("NTSC") => 1,
            Some("PAL") => 4,
            _ => 0,
        }
    }
}

/// GetDemoLaunchData as the modules call it: the C++ returns a 16-byte struct through the hidden
/// pointer `out` (a0), which the generated ABI drops; democontrol passes its frame here.
pub fn GetDemoLaunchData(e: &mut Engine, out: Ptr) {
    let src = e.host.APIgame_GetDemoLaunchData(&mut e.mem) as Ptr;
    if src != 0 {
        e.mem.memcpy(out, src, 0x10);
    }
}

/// Pause 0x001ed3c0: EFWorld::Pause(world, p != 0) + EFCutScenePlayer::Pause. The host part
/// (EFWorld+0x14 flag, cut-scene flag 2) returns the previous paused state; the object loop of
/// EFWorld::Pause (0x0026..) runs here because it needs the logic:
/// for every world object: logic flag 2 -> SetLogicFlags(f | 0x78) / (f & ~0x78); logic flag 1 ->
/// SendMessage(0x10000005, p). Deviation: iteration is in object-table order (the C++ walks its
/// hash map). N2: SetLogicFlags goes through the host's EFGameObject::SetLogicFlags (anim 0x8 =
/// EFAnimationMgr +0x50 bit0, physic 0x10, ...): only these pausable objects stop animating, the
/// rest (pause-menu panel) keeps its anims running.
pub fn Pause(e: &mut Engine, p: u32) -> u32 {
    let on = (p != 0) as u32;
    let was = e.host.APIgame_Pause(&mut e.mem, on);
    if was == on {
        return 0;
    }
    for h in 0..e.rt.objects.len() {
        let Some((f, id)) = e.rt.objects[h].as_ref().map(|o| (o.logic_flags, o.id)) else { continue };
        if f & 2 != 0 {
            // SetLogicFlags(f | 0x78) / SetLogicFlags(f & ~0x78)
            if on != 0 {
                e.host.APIgameobj_SetLogicFlag(&mut e.mem, id, 0x78);
            } else {
                e.host.APIgameobj_ResetLogicFlag(&mut e.mem, id, 0x78);
            }
        }
        if e.rt.objects[h].as_ref().is_some_and(|o| o.logic_flags & 1 != 0) {
            e.SendMessage(h as crate::logic::ObjH, 0x1000_0005, on, 0);
        }
    }
    0
}
