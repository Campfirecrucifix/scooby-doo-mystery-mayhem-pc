//! P34: PC settings (`settings.toml`), shared by the launcher, the game and the in-game PC Settings
//! page. One file: `[graphics]`, `[audio]`, `[remaster]`, `[launcher]` + the controls.toml keys
//! (top-level deadzone/trigger_threshold/mouse_*/invert_y, `[keyboard]`, `[gamepad]`), so an old
//! controls.toml copied to settings.toml still works. Migration: when settings.toml is missing and
//! controls.toml exists, its controls are taken over (controls.toml itself is left alone).
//! The P22 top-level `global_ray_tracing` key (engine::gfx::rtgi) is kept. CLI flags / env vars
//! override the file at start-up (game.rs).
use super::input::host::Controls;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    Borderless,
    Fullscreen,
}

impl WindowMode {
    pub const ALL: [WindowMode; 3] = [WindowMode::Windowed, WindowMode::Borderless, WindowMode::Fullscreen];
    pub fn name(self) -> &'static str {
        match self {
            WindowMode::Windowed => "windowed",
            WindowMode::Borderless => "borderless",
            WindowMode::Fullscreen => "fullscreen",
        }
    }
    pub fn parse(s: &str) -> Option<WindowMode> {
        Self::ALL.into_iter().find(|m| m.name() == s)
    }
}

/// Window sizes offered by the launcher / in-game page (any `WxH` is accepted in the file).
pub const RESOLUTIONS: [(u32, u32); 8] = [(640, 480), (1024, 768), (1280, 720), (1280, 960), (1600, 900), (1920, 1080), (2560, 1440), (3840, 2160)];
/// FPS cap choices (0 = off).
pub const FPS_CAPS: [u32; 6] = [0, 30, 60, 120, 144, 240];
/// MSAA sample counts (1 = off).
pub const MSAA: [u32; 4] = [1, 2, 4, 8];

#[derive(Clone, Debug, PartialEq)]
pub struct Graphics {
    pub window_mode: WindowMode,
    /// window size (windowed / borderless start size); None = Bevy default (1280x720)
    pub resolution: Option<(u32, u32)>,
    pub vsync: bool,
    /// 0 = uncapped
    pub fps_cap: u32,
    /// MSAA samples of the game cameras (1 = off; the port's default is 4). Applies on restart.
    pub msaa: u32,
}

impl Default for Graphics {
    fn default() -> Self {
        Graphics { window_mode: WindowMode::Borderless, resolution: None, vsync: true, fps_cap: 0, msaa: 4 }
    }
}

/// Volumes 0..1, multiplied on top of the game's own Music/FX/Movies options.
#[derive(Clone, Debug, PartialEq)]
pub struct Audio {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
    /// streamed dialogue + movie sound
    pub voice: f32,
    pub mute: bool,
}

impl Default for Audio {
    fn default() -> Self {
        Audio { master: 1.0, music: 1.0, sfx: 1.0, voice: 1.0, mute: false }
    }
}

impl Audio {
    /// effective gain of a category (master * category, 0 when muted)
    pub fn gain(&self, cat: f32) -> f32 {
        if self.mute { 0.0 } else { (self.master * cat).clamp(0.0, 1.0) }
    }
}

/// Remaster toggles: only used when a remaster build is installed next to game.exe (launcher).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Remaster {
    pub lighting: bool,
    pub texture_pack: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Launcher {
    /// "Play Scooby-Doo.bat" / launcher.exe start the game at once
    pub skip: bool,
    /// P34 R7: the settings tab shown last (Settings opens it again)
    pub tab: u32,
    /// P34 R12: the launcher window's last top-left corner (screen pixels; clamped to a monitor on start)
    pub pos: Option<(i32, i32)>,
}

/// P34 R4 `[game]`: the disc image chosen in the launcher (the game's --iso flag overrides it)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    pub iso: Option<PathBuf>,
    /// P41 R3: losing the window focus during gameplay opens the game's pause menu (default off)
    pub pause_on_focus_loss: bool,
    /// P41 R3 prototype: on-screen button icons show the bound PC key after keyboard input (default
    /// off = the original PS2 icons)
    pub pc_key_prompts: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub graphics: Graphics,
    pub audio: Audio,
    pub remaster: Remaster,
    pub launcher: Launcher,
    pub game: Game,
    pub controls: Controls,
    /// P22 `global_ray_tracing` (solari builds), kept as is
    pub global_ray_tracing: Option<bool>,
}

const SECTIONS: [&str; 5] = ["graphics", "audio", "remaster", "launcher", "game"];

fn strip_comment(l: &str) -> &str {
    let mut q = false;
    for (i, ch) in l.char_indices() {
        match ch {
            '"' => q = !q,
            '#' if !q => return &l[..i],
            _ => {}
        }
    }
    l
}

fn unquote(v: &str) -> &str {
    v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(v)
}

pub fn parse_res(v: &str) -> Option<(u32, u32)> {
    let (w, h) = v.split_once(['x', 'X'])?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    (w >= 320 && h >= 240 && w <= 16384 && h <= 16384).then_some((w, h))
}

impl Settings {
    pub fn to_toml(&self) -> String {
        let g = &self.graphics;
        let a = &self.audio;
        let mut s = String::from("# Scooby-Doo! Mystery Mayhem PC settings (launcher + in-game Options > PC Settings).\n");
        s += "# Controls keys: see CONTROLS.md. Command-line flags / GAME_* env vars override this file.\n\n";
        if let Some(on) = self.global_ray_tracing {
            s += &format!("global_ray_tracing = {on}\n");
        }
        // controls (top-level keys + [keyboard] / [gamepad]), minus its own header comment
        let c = self.controls.to_toml();
        let (top, rest) = c.split_once("\n[keyboard]").unwrap_or((&c, ""));
        s += top.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).map(|l| format!("{l}\n")).collect::<String>().as_str();
        s += "\n[graphics]\n";
        s += &format!("window_mode = \"{}\"   # windowed | borderless | fullscreen\n", g.window_mode.name());
        s += &format!("resolution = \"{}\"\n", g.resolution.map_or("default".into(), |(w, h)| format!("{w}x{h}")));
        s += &format!("vsync = {}\nfps_cap = {}   # 0 = off\nmsaa = {}   # 1 = off, 2, 4, 8 (restart)\n", g.vsync, g.fps_cap, g.msaa);
        s += &format!("\n[audio]\nmaster = {}\nmusic = {}\nsfx = {}\nvoice = {}\nmute = {}\n", a.master, a.music, a.sfx, a.voice, a.mute);
        s += &format!("\n[remaster]\nlighting = {}\ntexture_pack = {}\n", self.remaster.lighting, self.remaster.texture_pack);
        s += &format!("\n[launcher]\nskip = {}\ntab = {}\n", self.launcher.skip, self.launcher.tab);
        if let Some((x, y)) = self.launcher.pos {
            s += &format!("pos = \"{x},{y}\"   # the launcher window position\n");
        }
        s += &format!("\n[game]\niso = \"{}\"   # the disc image (launcher: Browse...)\n", self.game.iso.as_ref().map_or(String::new(), |p| p.display().to_string()));
        s += &format!("pause_on_focus_loss = {}   # pause when the window loses focus (in a level)\npc_key_prompts = {}   # prototype: PC key names instead of PS2 button icons\n", self.game.pause_on_focus_loss, self.game.pc_key_prompts);
        if !rest.is_empty() {
            s += "\n[keyboard]";
            s += rest;
        }
        s
    }

    /// Parse; missing keys keep the defaults, unknown keys / sections / actions (a file written by a
    /// newer or older build) are ignored with a warning on stderr. Bad values are errors.
    pub fn parse(text: &str) -> Result<Settings, String> {
        // P39: Notepad's "UTF-8 with BOM" must not turn the first line into garbage
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut st = Settings::default();
        let mut ctl = String::new();
        let mut sect = String::new();
        for (n, raw) in text.lines().enumerate() {
            let line = strip_comment(raw).trim();
            let err = |m: &str| format!("settings.toml line {}: {m}", n + 1);
            if let Some(h) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                sect = h.trim().to_string();
                if !SECTIONS.contains(&sect.as_str()) {
                    ctl += raw;
                }
                ctl.push('\n');
                continue;
            }
            let kv = line.split_once('=').map(|(k, v)| (k.trim(), unquote(v.trim())));
            // P22 rtgi::save_setting appends the key at the end of the file (any section)
            if let Some(("global_ray_tracing", v)) = kv {
                st.global_ray_tracing = Some(v == "true");
                ctl.push('\n');
                continue;
            }
            if !SECTIONS.contains(&sect.as_str()) || line.is_empty() {
                // controls part: same line numbers for Controls::parse errors
                ctl += raw;
                ctl.push('\n');
                continue;
            }
            ctl.push('\n');
            let (k, v) = kv.ok_or_else(|| err("expected key = value"))?;
            // P39: "nan" / "inf" parse as f32 but would reach the mixer / camera as NaN
            let num = || v.parse::<f32>().ok().filter(|x| x.is_finite()).ok_or_else(|| err("expected number"));
            let int = || v.parse::<u32>().map_err(|_| err("expected integer"));
            let flag = || v.parse::<bool>().map_err(|_| err("expected true/false"));
            let (g, a) = (&mut st.graphics, &mut st.audio);
            match (sect.as_str(), k) {
                ("graphics", "window_mode") => g.window_mode = WindowMode::parse(v).ok_or_else(|| err("expected windowed/borderless/fullscreen"))?,
                ("graphics", "resolution") => g.resolution = if v == "default" { None } else { Some(parse_res(v).ok_or_else(|| err("expected \"WxH\""))?) },
                ("graphics", "vsync") => g.vsync = flag()?,
                ("graphics", "fps_cap") => g.fps_cap = int()?.min(1000),
                ("graphics", "msaa") => g.msaa = MSAA.into_iter().find(|&m| m == int().unwrap_or(0)).ok_or_else(|| err("expected 1/2/4/8"))?,
                ("audio", "master") => a.master = num()?.clamp(0.0, 1.0),
                ("audio", "music") => a.music = num()?.clamp(0.0, 1.0),
                ("audio", "sfx") => a.sfx = num()?.clamp(0.0, 1.0),
                ("audio", "voice") => a.voice = num()?.clamp(0.0, 1.0),
                ("audio", "mute") => a.mute = flag()?,
                ("remaster", "lighting") => st.remaster.lighting = flag()?,
                ("remaster", "texture_pack") => st.remaster.texture_pack = flag()?,
                ("launcher", "skip") => st.launcher.skip = flag()?,
                ("launcher", "tab") => st.launcher.tab = int()?.min(3),
                ("launcher", "pos") => st.launcher.pos = Some(v.split_once(',').and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?))).ok_or_else(|| err("expected \"x,y\""))?),
                ("game", "pause_on_focus_loss") => st.game.pause_on_focus_loss = flag()?,
                ("game", "pc_key_prompts") => st.game.pc_key_prompts = flag()?,
                ("game", "iso") => st.game.iso = Some(PathBuf::from(v)).filter(|_| !v.is_empty()),
                _ => eprintln!("{} (ignored)", err(&format!("unknown key '{k}' in [{sect}]"))),
            }
        }
        // controls part: drop unknown keys / actions (same line numbers) and parse again
        let mut lines: Vec<String> = ctl.lines().map(String::from).collect();
        st.controls = loop {
            match Controls::parse(&lines.join("
")) {
                Ok(c) => break c,
                Err(e) => {
                    let n = e.strip_prefix("controls.toml line ").and_then(|r| r.split(':').next()).and_then(|n| n.parse::<usize>().ok());
                    match n.filter(|&n| e.contains(": unknown ") && n >= 1 && n <= lines.len() && !lines[n - 1].is_empty()) {
                        Some(n) => {
                            eprintln!("{} (ignored)", e.replace("controls.toml", "settings.toml"));
                            lines[n - 1].clear();
                        }
                        None => return Err(e.replace("controls.toml", "settings.toml")),
                    }
                }
            }
        };
        Ok(st)
    }

    /// Load `path`. Missing: migrate `legacy_controls` (controls.toml) when it exists, then write the
    /// file. Unreadable / garbage file (bad value, not UTF-8): it is kept as `settings.toml.bad`, the
    /// defaults are written + message (never a crash).
    pub fn load_or_create(path: &Path, legacy_controls: Option<&Path>) -> (Settings, Option<String>) {
        match std::fs::read(path) {
            Ok(b) => match decode_text(b).ok_or_else(|| "settings.toml: not a text file".to_string()).and_then(|t| Settings::parse(&t)) {
                Ok(s) => (s, None),
                Err(e) => {
                    let bak = path.with_extension("toml.bad");
                    let _ = std::fs::remove_file(&bak);
                    let mut msg = e;
                    match std::fs::rename(path, &bak) {
                        Ok(()) => msg += &format!("
The old file was saved as {}.", bak.display()),
                        Err(_) => return (Settings::default(), Some(msg)),
                    }
                    let s = Settings::default();
                    if let Err(e) = s.save(path) {
                        msg += &format!("
{e}");
                    }
                    (s, Some(msg))
                }
            },
            Err(_) if path.exists() => (Settings::default(), Some(format!("cannot read {}", path.display()))),
            Err(_) => {
                let mut s = Settings::default();
                let mut msg = None;
                if let Some(t) = legacy_controls.and_then(|p| std::fs::read_to_string(p).ok()) {
                    match Controls::parse(&t) {
                        Ok(c) => s.controls = c,
                        Err(e) => msg = Some(format!("{e} (controls not migrated)")),
                    }
                }
                if let Err(e) = s.save(path) {
                    msg = Some(e);
                }
                (s, msg)
            }
        }
    }

    /// P34 R9: like `merged` but per KEY (every `key = value` line of `to_toml`): the in-game save
    /// keeps a launcher change to another key of the same section (and vice versa)
    pub fn merged_keys(&self, base: &Settings, disk: &Settings) -> Settings {
        type Keyed = Vec<(String, Option<String>, String)>;
        fn keyed(t: &str) -> Keyed {
            let mut sect = String::new();
            t.lines()
                .map(|l| {
                    let s = strip_comment(l).trim();
                    if let Some(h) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                        sect = h.trim().to_string();
                        return (sect.clone(), None, l.to_string());
                    }
                    let k = s.split_once('=').map(|(k, _)| k.trim().to_string());
                    (sect.clone(), k, l.to_string())
                })
                .collect()
        }
        fn find(v: &Keyed, s: &str, k: &str) -> Option<String> {
            v.iter().find(|(s2, k2, _)| s2 == s && k2.as_deref() == Some(k)).map(|e| strip_comment(&e.2).trim().to_string())
        }
        let (m, b, d) = (keyed(&self.to_toml()), keyed(&base.to_toml()), keyed(&disk.to_toml()));
        let mut out: Vec<String> = Vec::new();
        for (s, k, line) in &m {
            match k {
                // unchanged here: take the file's value (if the file has the key)
                Some(k) if find(&b, s, k).as_deref() == Some(strip_comment(line).trim()) => out.push(find(&d, s, k).unwrap_or_else(|| line.clone())),
                _ => out.push(line.clone()),
            }
        }
        // keys only the file has (and the base did not): insert after their section header
        for (s, k, line) in &d {
            let Some(k) = k else { continue };
            if find(&m, s, k).is_none() && find(&b, s, k).is_none() {
                let at = if s.is_empty() { 0 } else { out.iter().position(|l| l.trim() == format!("[{s}]")).map_or(out.len(), |i| i + 1) };
                out.insert(at, line.clone());
            }
        }
        let mut r = Settings::parse(&out.join("\n")).unwrap_or_else(|_| self.merged(base, disk));
        r.global_ray_tracing = if self.global_ray_tracing != base.global_ray_tracing { self.global_ray_tracing } else { disk.global_ray_tracing };
        r
    }

    /// P34 R7: three-way merge for a writer that loaded `base` and now holds `self`, while `disk`
    /// is what another writer (the in-game menu / a second launcher) left in the file meanwhile:
    /// per section, keep ours when we changed it, else take the file's.
    pub fn merged(&self, base: &Settings, disk: &Settings) -> Settings {
        fn pick<T: Clone + PartialEq>(mine: &T, base: &T, disk: &T) -> T {
            if mine != base { mine.clone() } else { disk.clone() }
        }
        Settings {
            graphics: pick(&self.graphics, &base.graphics, &disk.graphics),
            audio: pick(&self.audio, &base.audio, &disk.audio),
            remaster: pick(&self.remaster, &base.remaster, &disk.remaster),
            launcher: pick(&self.launcher, &base.launcher, &disk.launcher),
            // per key here: the disc (launcher) and the pause flag (either side) are unrelated
            game: Game {
                iso: pick(&self.game.iso, &base.game.iso, &disk.game.iso),
                pause_on_focus_loss: pick(&self.game.pause_on_focus_loss, &base.game.pause_on_focus_loss, &disk.game.pause_on_focus_loss),
                pc_key_prompts: pick(&self.game.pc_key_prompts, &base.game.pc_key_prompts, &disk.game.pc_key_prompts),
            },
            controls: pick(&self.controls, &base.controls, &disk.controls),
            global_ray_tracing: pick(&self.global_ray_tracing, &base.global_ray_tracing, &disk.global_ray_tracing),
        }
    }

    /// P39: written to `settings.toml.tmp` first and renamed over the file, so a full disk / a
    /// crash mid-write keeps the old settings instead of leaving a cut file
    pub fn save(&self, path: &Path) -> Result<(), String> {
        write_atomic(path, self.to_toml().as_bytes()).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

/// P39: write `path` via `<path>.tmp` + rename (the tmp file is removed on failure).
/// P39 R6: the tmp name is unique per process + call (two game instances / the launcher and the game
/// saving the same file at once wrote into one shared `.tmp` and could rename a mix of both: now each
/// rename is whole, the last writer wins); the parent folder is created (first run, a new profile dir)
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(d);
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.{}.tmp", std::process::id(), SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let tmp = PathBuf::from(tmp);
    let r = std::fs::write(&tmp, data).and_then(|()| {
        #[cfg(test)]
        if FAIL_WRITE.with(|f| f.replace(false)) {
            return Err(std::io::Error::new(std::io::ErrorKind::StorageFull, "injected: disk full"));
        }
        std::fs::rename(&tmp, path)
    });
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

#[cfg(test)]
thread_local! {
    /// P39 R6 test seam: the next write_atomic on this thread fails after writing its tmp file (disk full)
    pub static FAIL_WRITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// P39: a hand-edited config file: UTF-8 (with or without BOM) or Notepad's "Unicode" (UTF-16 with
/// BOM); None = not text
pub fn decode_text(b: Vec<u8>) -> Option<String> {
    let utf16 = |be: bool| -> Option<String> {
        let u: Vec<u16> = b[2..].chunks_exact(2).map(|c| if be { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) }).collect();
        String::from_utf16(&u).ok()
    };
    match b.get(..2) {
        Some([0xff, 0xfe]) => utf16(false),
        Some([0xfe, 0xff]) => utf16(true),
        _ => String::from_utf8(b).ok().map(|s| s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s)),
    }
}

/// P34 R7: the crash records process `pid` appended to the crash log (`tail` = the part written
/// since the launcher started it): each record's `=== ...` line + its first detail line. Info
/// notes (SAVES MIGRATED, a closed console window) don't count; an UNCLEAN EXIT of an older run
/// names that run's pid, so it doesn't either.
pub fn crash_records(tail: &str, pid: u32) -> Vec<String> {
    let me = format!(" pid {pid}");
    let lines: Vec<&str> = tail.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let Some(h) = l.strip_prefix("=== ") else { continue };
        let own = h.find(&me).is_some_and(|k| !h[k + me.len()..].starts_with(|c: char| c.is_ascii_digit()));
        if !own || h.starts_with("SAVES MIGRATED") || h.starts_with("CONSOLE EXIT") {
            continue;
        }
        let mut r = h.trim().to_string();
        if let Some(d) = lines.get(i + 1).filter(|d| !d.starts_with("===") && !d.trim().is_empty()) {
            r += "
";
            r += d.trim();
        }
        out.push(r);
    }
    out
}

/// `name` in the cwd, else next to the exe when only that one exists (the controls.toml rule).
pub fn config_path(name: &str) -> PathBuf {
    let cwd = PathBuf::from(name);
    match std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(name))) {
        Some(e) if !cwd.exists() && e.exists() => e,
        _ => cwd,
    }
}

/// settings.toml path + the legacy controls.toml next to it. `SCOOBY_SETTINGS` = another
/// settings.toml (tests, a second profile).
pub fn paths() -> (PathBuf, PathBuf) {
    if let Some(p) = std::env::var_os("SCOOBY_SETTINGS").filter(|p| !p.is_empty()).map(PathBuf::from) {
        let c = p.with_file_name("controls.toml");
        return (p, c);
    }
    (config_path("settings.toml"), config_path("controls.toml"))
}

/// P34 R4: exclusive fullscreen video mode. `modes` = (w, h, refresh mHz) of the monitor; the wanted
/// size (None = the desktop size) at the highest refresh, else the closest size. None = no modes
/// (the caller falls back to borderless).
pub fn pick_video_mode(modes: &[(u32, u32, u32)], want: Option<(u32, u32)>, desktop: (u32, u32)) -> Option<usize> {
    let (w, h) = want.unwrap_or(desktop);
    let cost = |&(mw, mh, r): &(u32, u32, u32)| ((mw as i64 - w as i64).abs() + (mh as i64 - h as i64).abs(), u32::MAX - r);
    (0..modes.len()).min_by_key(|&i| cost(&modes[i]))
}

/// Logs / crash log folder: `%LOCALAPPDATA%\ScoobyMM\logs` (else the temp dir); one helper for the
/// launcher and the game (= [`crate::crashlog::log_dir`]).
pub fn log_dir() -> PathBuf {
    crate::crashlog::log_dir()
}

/// The disc this port runs: the PAL SCEE disc of Scooby-Doo! Mystery Mayhem (boots PBPX_955.06).
pub const DISC_ELF: &str = "PBPX_955.06";

/// P34 R6: the releases of the game, by the ELF their SYSTEM.CNF boots: (ELF, release, tested).
/// The port runs the game's data files only (the executable is recompiled; ENGINE.INI is built
/// in). The tested disc is the SCEE disc (PBPX-95506) whose English game is the NTSC-U build
/// (its ENGINE.INI says ProductCode BASLUS-20701, DisplayMode NTSC), so the US disc should be
/// the same data; the PAL retail disc (E/F/G) may be a later build and is unknown.
pub const RELEASES: [(&str, &str, bool); 3] = [
    (DISC_ELF, "SCEE disc PBPX-95506", true),
    ("SLUS_207.01", "NTSC-U disc SLUS-20701", false),
    ("SLES_518.83", "PAL disc SLES-51883", false),
];

/// P34 R4/R6: check a disc image the way the game opens it (engine::iso::IsoFs): a readable
/// ISO-9660 image (.iso/.img/.bin, 2048 or raw 2352-byte sectors, or a .cue naming the .bin),
/// a known release of the game, and the folders the game loads from. Ok = a display line; a
/// line starting with '!' = playable but a warning (untested release, network path).
pub fn validate_iso(path: &Path) -> Result<String, String> {
    validate_iso_with(path, allow_untested_disc())
}

/// P34 R12: [`validate_iso`] that gives up after `limit` (a removed USB drive / an offline network
/// share can block a file open for 20+ s; the launcher calls this on its UI thread). The probe
/// thread is left behind on a timeout and ends on its own when the OS gives up.
pub fn validate_iso_within(path: &Path, limit: std::time::Duration) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let p = path.to_path_buf();
    std::thread::spawn(move || {
        let _ = tx.send(validate_iso(&p));
    });
    rx.recv_timeout(limit).unwrap_or_else(|_| Err("the drive or network share does not respond (a removed USB drive, or an offline network folder?)".into()))
}

/// P39 R3: `SCOOBY_UNTESTED_DISC=1` lets an untested release start (testers checking a US / PAL disc);
/// otherwise the launcher and game.exe refuse it with [`unsupported_disc`] instead of a gamedata panic
pub fn allow_untested_disc() -> bool {
    std::env::var_os("SCOOBY_UNTESTED_DISC").is_some_and(|v| !v.is_empty() && v != "0")
}

/// P39 R3: the ELF a SYSTEM.CNF boots (`BOOT2 = cdrom0:\SLUS_207.01;1` -> "SLUS_207.01"; "" if none)
pub fn boot_elf(cnf: &[u8]) -> String {
    let cnf = String::from_utf8_lossy(cnf);
    let boot = cnf.lines().find_map(|l| l.trim().trim_start_matches('\u{feff}').strip_prefix("BOOT2")).map(|l| l.trim_start_matches([' ', '=']).trim().to_string()).unwrap_or_default();
    boot.rsplit(['\\', ':']).next().unwrap_or("").split(';').next().unwrap_or("").trim().to_ascii_uppercase()
}

/// P39 R3: "SLUS_207.01" -> "SLUS-20701"
pub fn serial(elf: &str) -> String {
    elf.replace('_', "-").replace('.', "")
}

/// P39 R3: the player message for a release whose game data tables are not verified
pub fn unsupported_disc(elf: &str) -> String {
    let what = RELEASES.iter().find(|r| r.0 == elf).map_or_else(|| serial(elf), |r| r.1.to_string());
    let ok: Vec<&str> = RELEASES.iter().filter(|r| r.2).map(|r| r.1).collect();
    format!("This disc version ({what}) isn't supported yet; supported: {}.", ok.join(", "))
}

/// P39 R3: Err(player message) when the image's SYSTEM.CNF names a known but untested release (game.exe
/// start-up, before a game data lookup can panic). Unreadable images / no SYSTEM.CNF / another game give Ok:
/// the normal open path reports those.
pub fn check_disc_supported(path: &Path) -> Result<(), String> {
    disc_release(path).map_or(Ok(()), |(elf, tested)| if tested || allow_untested_disc() { Ok(()) } else { Err(unsupported_disc(&elf)) })
}

/// P39 R3: (boot ELF, tested) of a known release on this image; None if unreadable / not this game
pub fn disc_release(path: &Path) -> Option<(String, bool)> {
    let cnf = super::iso::IsoFs::open(path).ok()?.read("SYSTEM.CNF").ok()?;
    let elf = boot_elf(&cnf);
    let tested = RELEASES.iter().find(|r| r.0 == elf)?.2;
    Some((elf, tested))
}

/// [`validate_iso`] with the untested-release override given (tests)
pub fn validate_iso_with(path: &Path, allow_untested: bool) -> Result<String, String> {
    let img = super::iso::resolve_image(path).map_err(|e| format!("can't read the .cue sheet ({e})"))?;
    if !img.is_file() {
        return Err(format!("{} does not exist", img.display()));
    }
    let mut head = [0u8; 8];
    match std::fs::File::open(&img).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head)) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Err("can't read this file (access denied)".into()),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Err("not a disc image (the file is too small)".into()),
        Err(e) => return Err(format!("can't read this file ({e})")),
        Ok(()) => {}
    }
    let packed = [(&b"PK\x03\x04"[..], "a .zip archive"), (b"7z\xbc\xaf", "a .7z archive"), (b"Rar!", "a .rar archive")];
    if let Some((_, what)) = packed.iter().find(|(m, _)| head.starts_with(m)) {
        return Err(format!("this is {what}: extract the .iso from it first"));
    }
    let other = [(&b"CISO"[..], ".cso"), (b"ZISO", ".zso"), (b"MComprHD", ".chd")];
    if let Some((_, what)) = other.iter().find(|(m, _)| head.starts_with(m)) {
        return Err(format!("{what} images are not supported: convert it to a plain .iso"));
    }
    let fs = super::iso::IsoFs::open(path).map_err(|e| match e {
        super::error::EngineError::Format(_) => "not a disc image (no ISO-9660 file system): choose the .iso of your Scooby-Doo! Mystery Mayhem PS2 disc".to_string(),
        e => format!("can't read this disc image ({e:?})"),
    })?;
    let cnf = fs.read("SYSTEM.CNF").map_err(|_| "a disc image, but not a PlayStation 2 game (no SYSTEM.CNF)".to_string())?;
    let elf = boot_elf(&cnf);
    let Some(&(_, release, tested)) = RELEASES.iter().find(|r| r.0 == elf) else {
        return Err(format!("a PS2 disc, but not Scooby-Doo! Mystery Mayhem ({elf}); this port needs the SCEE disc PBPX-95506 (or try SLUS-20701 / SLES-51883)"));
    };
    if !tested && !allow_untested {
        return Err(unsupported_disc(&elf));
    }
    for need in ["LEVEL/LOADER/", "GLOBAL/", "STREAM/", "MOVIES/"] {
        if !fs.names().any(|n| n.starts_with(need)) {
            return Err(format!("{release}, but {need} is missing (damaged or incomplete image?)"));
        }
    }
    let network = img.to_string_lossy().replace('/', "\\").starts_with("\\\\");
    Ok(match (tested, network) {
        (true, false) => format!("Scooby-Doo! Mystery Mayhem ({release}) found"),
        (false, _) => format!("!{release} found: untested, please report problems"),
        (true, true) => format!("!{release} on a network share: may stutter, copy it locally"),
    })
}

/// P34 R4: the first valid disc image (*.iso / *.bin) in the usual places: `dir` (the exe folder)
/// and 4 parents, the cwd, Documents, Downloads, Desktop.
pub fn find_iso(dir: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = dir.ancestors().take(5).map(Path::to_path_buf).collect();
    dirs.extend(std::env::current_dir().ok());
    if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        dirs.extend(["Documents", "Downloads", "Desktop"].map(|d| home.join(d)));
    }
    let is_img = |p: &PathBuf| p.extension().is_some_and(|e| ["iso", "bin", "img", "cue"].iter().any(|x| e.eq_ignore_ascii_case(x)));
    dirs.iter().flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path())).filter(is_img).find(|p| validate_iso(p).is_ok())
}

/// Load (or create / migrate) the default settings.toml.
pub fn load() -> (Settings, PathBuf, Option<String>) {
    let (s, c) = paths();
    let (st, e) = Settings::load_or_create(&s, Some(&c));
    (st, s, e)
}

/// Remaster build installed next to game.exe (the remaster branches are never merged into main;
/// a player copies their `game_remaster.exe` + `texpack\` here).
pub const REMASTER_EXE: &str = "game_remaster.exe";

pub fn remaster_exe(dir: &Path) -> Option<PathBuf> {
    Some(dir.join(REMASTER_EXE)).filter(|p| p.is_file())
}

/// Texture pack folder for `GAME_TEXPACK`: `texpack\` next to the exe or in the recomp dir.
pub fn texpack_dir(dir: &Path) -> Option<PathBuf> {
    [dir.join("texpack"), dir.join("..").join("..").join("texpack")].into_iter().find(|p| p.is_dir())
}

/// Env vars for the remaster exe in `dir`. The remaster build turns its lighting on whenever
/// GAME_TEXPACK is set unless GAME_REMASTER_LIGHT=0, so the light flag is always explicit. It reads
/// `remaster/light_levels.tsv` relative to the cwd: GAME_RL_LEVELS points at the one next to the
/// exe (or in the recomp dir, like texpack). Empty value = remove the variable.
pub fn remaster_env(r: &Remaster, dir: &Path) -> Vec<(&'static str, String)> {
    let pack = texpack_dir(dir).filter(|_| r.texture_pack).map(|p| p.display().to_string()).unwrap_or_default();
    let levels = [dir.to_path_buf(), dir.join("..").join("..")].into_iter().map(|d| d.join("remaster").join("light_levels.tsv")).find(|p| p.is_file()).map(|p| p.display().to_string());
    let mut v = vec![("GAME_REMASTER_LIGHT", if r.lighting { "1".into() } else { "0".into() }), ("GAME_TEXPACK", pack)];
    if let Some(l) = levels.filter(|_| std::env::var_os("GAME_RL_LEVELS").is_none()) {
        v.push(("GAME_RL_LEVELS", l));
    }
    v
}

/// What Play starts: the exe, its cwd (inherited = None) and env changes (None = remove).
/// game.exe unless the remaster build is installed in `dir` and a remaster option is on.
pub fn launch_plan(st: &Settings, dir: &Path) -> (PathBuf, Vec<(&'static str, Option<String>)>) {
    match remaster_exe(dir).filter(|_| st.remaster.lighting || st.remaster.texture_pack) {
        Some(exe) => (exe, remaster_env(&st.remaster, dir).into_iter().map(|(k, v)| (k, Some(v).filter(|v| !v.is_empty()))).collect()),
        None => (dir.join("game.exe"), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::iso::tests::{build_iso, tmp};

    #[test]
    fn p39_corrupt_settings_fall_back() {
        let d = tmp("settings");
        let p = d.join("settings.toml");
        let garbage: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2654435761) >> 7) as u8).collect();
        let huge = "x".repeat(8 << 20).into_bytes();
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("binary", garbage),
            ("huge one line", huge),
            ("cut mid-line", b"[graphics]\nwindow_mo".to_vec()),
            ("bad value", b"[graphics]\nvsync = maybe\n".to_vec()),
            ("nan volume", b"[audio]\nmaster = nan\n".to_vec()),
            ("inf deadzone", b"deadzone = inf\n".to_vec()),
            ("bad utf-16", vec![0xff, 0xfe, 0x00, 0xd8]),
            ("nul bytes", vec![0u8; 1000]),
        ];
        for (what, bytes) in cases {
            std::fs::write(&p, &bytes).unwrap();
            let (s, msg) = Settings::load_or_create(&p, None);
            assert_eq!(s, Settings::default(), "{what}");
            assert!(msg.is_some(), "{what}: no message");
            assert_eq!(std::fs::read(d.join("settings.toml.bad")).unwrap(), bytes, "{what}: original not kept");
            assert_eq!(Settings::load_or_create(&p, None), (Settings::default(), None), "{what}: rewritten file");
        }
        assert!(no_tmp(&d));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// no leftover `*.tmp` of write_atomic in `d`
    fn no_tmp(d: &Path) -> bool {
        std::fs::read_dir(d).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".tmp"))
    }

    #[test]
    fn p39_settings_encodings() {
        let text = "[graphics]\r\nvsync = false\r\n[audio]\r\nmusic = 0.5\r\n[game]\r\niso = \"C:/Users/Jürgen Ñ 测试/OneDrive/Scooby.iso\"\r\n";
        let want = Settings::parse(text).unwrap();
        assert!(!want.graphics.vsync && want.audio.music == 0.5);
        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend(text.as_bytes());
        let mut le = vec![0xff, 0xfe];
        le.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        let mut be = vec![0xfe, 0xff];
        be.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
        for b in [bom, le, be] {
            assert_eq!(Settings::parse(&decode_text(b).unwrap()).unwrap(), want);
        }
        assert_eq!(Settings::parse(&format!("\u{feff}{text}")).unwrap(), want);
        // round trip of a non-ASCII disc path
        assert_eq!(Settings::parse(&want.to_toml()).unwrap(), want);
    }

    #[test]
    fn p39_settings_unwritable_and_long_paths() {
        let d = tmp("settings_paths");
        // P39 R6 first run: a missing folder is created and the defaults written, no message
        let (s, msg) = Settings::load_or_create(&d.join("gone").join("new dir").join("settings.toml"), None);
        assert_eq!((s, msg), (Settings::default(), None));
        assert!(d.join("gone").join("new dir").join("settings.toml").is_file());
        // a file where the folder should be: defaults + message, no panic
        std::fs::write(d.join("afile"), b"").unwrap();
        let (_, msg) = Settings::load_or_create(&d.join("afile").join("settings.toml"), None);
        assert!(msg.is_some());
        // failed atomic write (target is a folder): error, no .tmp left behind
        std::fs::create_dir_all(d.join("isdir.toml")).unwrap();
        assert!(Settings::default().save(&d.join("isdir.toml")).is_err());
        assert!(no_tmp(&d));
        // read-only settings.toml with a bad value: still falls back
        let ro = d.join("ro.toml");
        std::fs::write(&ro, b"[graphics]\nvsync = 7\n").unwrap();
        let mut perm = std::fs::metadata(&ro).unwrap().permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&ro, perm).unwrap();
        let (s, msg) = Settings::load_or_create(&ro, None);
        assert_eq!((s, msg.is_some()), (Settings::default(), true));
        for f in [ro.clone(), ro.with_extension("toml.bad")] {
            if let Ok(m) = std::fs::metadata(&f) {
                let mut p = m.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                p.set_readonly(false);
                let _ = std::fs::set_permissions(&f, p);
            }
        }
        // a non-ASCII profile folder past MAX_PATH
        let mut dir = d.join("Jürgen Ñ 测试").join("OneDrive - Contoso");
        while dir.as_os_str().len() < 250 {
            dir = dir.join("ScoobyMM Ünïcødé");
        }
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.toml");
        let (s, msg) = Settings::load_or_create(&p, None);
        assert_eq!((s, msg), (Settings::default(), None));
        let mut st = Settings::default();
        st.game.iso = Some(dir.join("Scooby Ñ.iso"));
        st.save(&p).unwrap();
        assert_eq!(Settings::load_or_create(&p, None).0, st);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// P39 R3: SYSTEM.CNF serial check (launcher + game.exe start-up) and the gamedata error text
    #[test]
    fn p39_unsupported_disc() {
        assert_eq!(boot_elf(b"BOOT2 = cdrom0:\\SLUS_207.01;1\r\nVER = 1.00\r\n"), "SLUS_207.01");
        assert_eq!(boot_elf(b"\xef\xbb\xbfBOOT2=cdrom0:\\sles_518.83;1"), "SLES_518.83");
        assert_eq!(boot_elf(b"VER = 1.00"), "");
        assert_eq!(boot_elf(b"BOOT2 ="), "");
        assert_eq!(boot_elf(&[0xff; 64]), "");
        assert_eq!(serial("SLUS_207.01"), "SLUS-20701");
        let m = unsupported_disc("SLUS_207.01");
        assert_eq!(m, "This disc version (NTSC-U disc SLUS-20701) isn't supported yet; supported: SCEE disc PBPX-95506.");
        assert!(unsupported_disc("SLUS_999.99").starts_with("This disc version (SLUS-99999) "));
        let d = tmp("unsupported");
        let dirs: [(&str, &[u8]); 4] = [("LEVEL/LOADER/A", b"a"), ("GLOBAL/B", b"b"), ("STREAM/C", b"c"), ("MOVIES/D", b"d")];
        let iso = |name: &str, cnf: &[u8]| {
            let mut f = vec![("SYSTEM.CNF", cnf)];
            f.extend(dirs);
            std::fs::write(d.join(name), build_iso(&f)).unwrap();
            d.join(name)
        };
        let us = iso("us.iso", b"BOOT2 = cdrom0:\\SLUS_207.01;1\r\n");
        let pal = iso("pal.iso", b"BOOT2 = cdrom0:\\SLES_518.83;1\r\n");
        let ok = iso("ok.iso", b"BOOT2 = cdrom0:\\PBPX_955.06;1\r\n");
        let other = iso("other.iso", b"BOOT2 = cdrom0:\\SLUS_999.99;1\r\n");
        assert_eq!(validate_iso_with(&us, false).unwrap_err(), m);
        assert!(validate_iso_with(&us, true).unwrap().starts_with('!'));
        assert!(validate_iso_with(&pal, false).unwrap_err().contains("(PAL disc SLES-51883) isn't supported yet"));
        assert!(!validate_iso_with(&ok, false).unwrap().starts_with('!'));
        assert_eq!(disc_release(&us), Some(("SLUS_207.01".into(), false)));
        assert_eq!(disc_release(&ok), Some(("PBPX_955.06".into(), true)));
        assert_eq!(disc_release(&other), None);
        assert_eq!(disc_release(&d.join("nothere.iso")), None);
        // game.exe start-up: only a known untested release is refused (the normal open path reports the rest)
        if !allow_untested_disc() {
            assert_eq!(check_disc_supported(&us), Err(m.clone()));
        }
        for p in [&ok, &other, &d.join("nothere.iso")] {
            assert_eq!(check_disc_supported(p), Ok(()));
        }
        // gamedata's single error path: the message names the reason
        use crate::engine::gamedata::missing_hint;
        assert_eq!(missing_hint(Some(&us)), m);
        assert!(missing_hint(Some(&ok)).contains("damaged or modified"));
        assert!(missing_hint(Some(&other)).contains("--iso"));
        assert!(missing_hint(None).contains("no disc image opened"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn p39_validate_iso_messages() {
        let d = tmp("validate");
        let cnf: &[u8] = b"BOOT2 = cdrom0:\\PBPX_955.06;1\r\n";
        let full = [("SYSTEM.CNF", cnf), ("LEVEL/LOADER/A", b"a"), ("GLOBAL/B", b"b"), ("STREAM/C", b"c"), ("MOVIES/D", b"d")];
        std::fs::write(d.join("ok.iso"), build_iso(&full)).unwrap();
        assert!(validate_iso(&d.join("ok.iso")).unwrap().contains("found"));
        // the game folder (LEVEL/LOADER, MOVIES...) missing from the disc
        std::fs::write(d.join("nolevel.iso"), build_iso(&[full[0], full[2], full[3], full[4]])).unwrap();
        assert!(validate_iso(&d.join("nolevel.iso")).unwrap_err().contains("LEVEL/LOADER/ is missing"));
        std::fs::write(d.join("nomovies.iso"), build_iso(&full[..4])).unwrap();
        assert!(validate_iso(&d.join("nomovies.iso")).unwrap_err().contains("MOVIES/ is missing"));
        std::fs::write(d.join("other.iso"), build_iso(&[("SYSTEM.CNF", b"BOOT2 = cdrom0:\\SLUS_999.99;1")])).unwrap();
        assert!(validate_iso(&d.join("other.iso")).unwrap_err().contains("not Scooby-Doo"));
        std::fs::write(d.join("nocnf.iso"), build_iso(&[("README.TXT", b"x")])).unwrap();
        assert!(validate_iso(&d.join("nocnf.iso")).unwrap_err().contains("SYSTEM.CNF"));
        let img = build_iso(&full);
        std::fs::write(d.join("cut.iso"), &img[..18 * 2048 + 10]).unwrap();
        assert!(validate_iso(&d.join("cut.iso")).is_err());
        std::fs::write(d.join("tiny.iso"), b"abc").unwrap();
        assert!(validate_iso(&d.join("tiny.iso")).unwrap_err().contains("too small"));
        assert!(validate_iso(&d.join("missing.iso")).unwrap_err().contains("does not exist"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
