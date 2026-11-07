//! P32 R73: always-on crash log. A Windows unhandled-exception filter (+ a chained panic hook) appends the
//! exception code, faulting address, module + offset, access kind/address, thread, a backtrace and the current
//! engine context (frame, level, game object being updated) to a crash log, so a rare native crash (the
//! intermittent 0xc0000005, R69-R72) pins itself on its next occurrence. Path: `$SCOOBY_CRASHLOG`, else
//! `%TEMP%/scooby_crash.log`. `P17_CRASHDBG=1` additionally logs first-chance faults (vectored handler).
//!
//! P37 R1: the game binary calls [`install_game`] instead: default path `%LOCALAPPDATA%\ScoobyMM\logs\crash.log`
//! (same folder as P34's `settings::log_dir`), a backtrace + version on every panic record, a fatal-failure
//! handler (heap corruption / fast-fail, when the OS still runs vectored handlers), console close / Ctrl+C
//! records, and a per-process session marker: a run that dies without any record (abort, __fastfail, kill,
//! power loss) is reported as `=== UNCLEAN EXIT` by the next start. Tests keep calling [`install`] (temp path,
//! no backtraces: they deliberately catch hundreds of panics).
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering::Relaxed};

/// version string for `game --version`, the window/launcher and every crash record (`SCOOBY_BUILD` = the git
/// revision baked in by tools/make_dist)
pub fn version() -> String {
    match option_env!("SCOOBY_BUILD") {
        Some(b) if !b.is_empty() => format!("{} ({b})", env!("CARGO_PKG_VERSION")),
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// P39 R11: the command line without the program name. `std::env::args()` panics on an argument that is not
/// valid Unicode (Windows allows unpaired UTF-16 surrogates in file names, so an ISO found by the launcher in
/// such a folder and handed over as `--iso`) -- before any message box can show. Lossy instead: the bad
/// characters become U+FFFD, the path then fails to open and the player gets the normal "disc image" message.
pub fn args() -> Vec<String> {
    lossy_args(std::env::args_os().skip(1))
}

pub fn lossy_args(it: impl Iterator<Item = std::ffi::OsString>) -> Vec<String> {
    it.map(|a| a.into_string().unwrap_or_else(|a| a.to_string_lossy().into_owned())).collect()
}

static GAME: AtomicBool = AtomicBool::new(false);
static DEFAULT: OnceLock<PathBuf> = OnceLock::new();
static MARKER: OnceLock<PathBuf> = OnceLock::new();

/// per-user logs folder: `%LOCALAPPDATA%\ScoobyMM\logs` (else the temp dir); = P34 `settings::log_dir`
pub fn log_dir() -> PathBuf {
    user_dir().map(|d| d.join("logs")).unwrap_or_else(std::env::temp_dir)
}

/// P39: the per-user `ScoobyMM` folder (logs, saves, cache): `%LOCALAPPDATA%\ScoobyMM`; an unset /
/// empty / relative LOCALAPPDATA (stripped environment, some launchers) falls back to
/// `%USERPROFILE%\AppData\Local`, else None. Never a cwd-relative path.
pub fn user_dir() -> Option<PathBuf> {
    user_dir_from(std::env::var_os("LOCALAPPDATA"), std::env::var_os("USERPROFILE"))
}

/// [`user_dir`] for the given LOCALAPPDATA / USERPROFILE values
pub fn user_dir_from(local: Option<std::ffi::OsString>, profile: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let abs = |v: Option<std::ffi::OsString>| v.map(PathBuf::from).filter(|p| p.is_absolute());
    abs(local).or_else(|| abs(profile).map(|h| h.join("AppData").join("Local"))).map(|d| d.join("ScoobyMM"))
}

/// id of the game object whose logic is running (EFGameLogicMgr::UpdateLogic), 0 outside
pub static OBJ: AtomicU32 = AtomicU32::new(0);
static LEVEL: [AtomicU8; 24] = [const { AtomicU8::new(0) }; 24];

/// record the level being loaded (lock-free: read from the exception filter)
pub fn set_level(name: &str) {
    let b = name.as_bytes();
    for (i, s) in LEVEL.iter().enumerate() {
        s.store(b.get(i).copied().unwrap_or(0), Relaxed);
    }
}

/// one-line engine context: frame / level / object
pub fn context() -> String {
    let lv: Vec<u8> = LEVEL.iter().map(|b| b.load(Relaxed)).take_while(|&b| b != 0).collect();
    format!(
        "frame {} level {:?} obj {:#x}",
        crate::engine::runtime::TRACE_FRAME.load(Relaxed),
        String::from_utf8_lossy(&lv),
        OBJ.load(Relaxed)
    )
}

pub fn path() -> PathBuf {
    std::env::var_os("SCOOBY_CRASHLOG")
        .map(Into::into)
        .or_else(|| DEFAULT.get().cloned())
        .unwrap_or_else(|| std::env::temp_dir().join("scooby_crash.log"))
}

fn append(s: &str) {
    use std::io::Write;
    // P39: not eprint!: it panics when stderr is gone (closed pipe), which inside the panic hook aborts
    let _ = std::io::stderr().write_all(s.as_bytes());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path()) {
        // P39 R8: no user name / machine name in a crash.log attached to a bug report
        let _ = f.write_all(redact(s).as_bytes());
    }
}

/// P37 R1: a non-panic fatal condition (e.g. a level that cannot be loaded) as a crash-log record
pub fn note(tag: &str, msg: &str) {
    append(&format!("=== {tag} pid {}: {msg}
  {} version {}
", std::process::id(), context(), version()));
}

/// install once per process (game binary main, test harness)
pub fn install() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |p| {
            let mut s = format!("=== PANIC pid {} thread {:?}: {p}\n  {}\n", std::process::id(), std::thread::current().name(), context());
            if GAME.load(Relaxed) {
                s += &format!("  version {}\n{}\n", version(), std::backtrace::Backtrace::force_capture());
            }
            append(&s);
            PANICKED.store(true, Relaxed);
            prev(p);
            // P39 R2: let the panic message reach game.log through the stdout/stderr pipe (ship build)
            flush_std();
        }));
        #[cfg(windows)]
        win::install();
    });
}

/// P39 R8: a panic was recorded in this process (any thread)
pub fn panicked() -> bool {
    PANICKED.load(Relaxed)
}

static PANICKED: AtomicBool = AtomicBool::new(false);

/// P39 R8: the player text after a panic that ended the game (no more specific message)
pub fn internal_error_message() -> String {
    let dir = path().parent().map(|p| redact(&p.display().to_string()).into_owned()).unwrap_or_default();
    format!("The game hit an internal error and had to close. The details are in crash.log in {dir}; please attach it (and game.log) to a bug report. Your saved progress on the memory card is kept.")
}

/// P37 R1: the game binary's crash log (see the module doc). Call first thing in main; pair with [`clean_exit`].
pub fn install_game() {
    let _ = DEFAULT.set(log_dir().join("crash.log"));
    GAME.store(true, Relaxed);
    install();
    let dir = path().parent().map(Path::to_path_buf).unwrap_or_else(log_dir);
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(windows)]
    win::redirect_detached(&dir);
    // P39 R2: only the shipped game (its output goes to game.log); dev / lane runs keep their raw handles
    #[cfg(all(windows, feature = "ship"))]
    win::guard_std();
    #[cfg(windows)]
    win::install_game();
    let args = args();
    let (stale, marker) = session_begin(&dir, std::process::id(), &format!("version {} args {:?}", version(), args), pid_alive);
    for s in stale {
        append(&s);
    }
    if let Some(m) = marker {
        let _ = MARKER.set(m);
    }
}

/// P37 R1: the game ended normally (AppExit): drop this run's session marker
pub fn clean_exit() {
    flush_std();
    if let Some(m) = MARKER.get() {
        let _ = std::fs::remove_file(m);
    }
}

static EXIT_MSG: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// P39 R5: a fatal runtime error that ends the game (disc unreadable mid-game, GPU device lost): a crash.log
/// record now, and `player` kept for [`take_exit_message`] so main shows it in a message box once the app loop
/// has returned (a box from inside a Bevy system would block the render thread mid-frame)
pub fn fatal(kind: &str, player: &str, detail: &str) {
    eprintln!("game: {kind}: {player} ({detail})");
    note(kind, &format!("{player}\n  {detail}"));
    let mut m = EXIT_MSG.lock().unwrap_or_else(|e| e.into_inner());
    if m.is_none() {
        *m = Some(player.to_string());
    }
}

/// P39 R5: the first [`fatal`] message of this run (main: show it, then exit with an error code); None when
/// an [`error_box`] was already shown this run (gamedata::missing before its loader-thread panic)
pub fn take_exit_message() -> Option<String> {
    let m = EXIT_MSG.lock().unwrap_or_else(|e| e.into_inner()).take();
    m.filter(|_| !BOX_SHOWN.load(Relaxed))
}

static BOX_SHOWN: AtomicBool = AtomicBool::new(false);

/// P39 R5: the player text for a lost / reset GPU device (wgpu DeviceLost, a driver update or TDR reset)
pub fn gpu_lost_message(detail: &str) -> String {
    let d = detail.trim();
    let d = if d.is_empty() { String::new() } else { format!(" ({})", d.chars().take(200).collect::<String>()) };
    format!("The graphics device stopped responding{d}, so the game had to close. This usually follows a graphics driver reset or update. Update the graphics driver and start the game again; your saved progress on the memory card is kept.")
}

/// P39 R12: the player text when no usable GPU / graphics driver was found at start (Bevy's renderer init
/// panics: no adapter, the surface cannot be created, or the device lacks the needed features / limits)
pub fn gpu_init_message(detail: &str) -> String {
    let d = detail.trim();
    let d = if d.is_empty() { String::new() } else { format!("\n\n({})", d.chars().take(300).collect::<String>()) };
    format!("The game could not start the graphics: no supported graphics card or driver was found. The game needs a DirectX 12 or Vulkan capable graphics card. Update the graphics driver and start the game again.{d}")
}

/// P39 R12: the text of a panic payload (`panic!` with a literal or a formatted message)
pub fn panic_payload_text(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default()
}

/// P39 R12: the player message for a panic that unwound out of the game's main (renderer init in
/// `add_plugins(DefaultPlugins)`, before the app loop: it ended the game without any message box)
pub fn main_panic_message(payload: &str) -> String {
    const GPU: [&str; 4] = ["Unable to find a GPU", "Failed to create wgpu surface", "RequestDeviceError", "get the window handle in time"];
    if GPU.iter().any(|k| payload.contains(k)) {
        gpu_init_message(payload.lines().next().unwrap_or(""))
    } else {
        internal_error_message()
    }
}

/// P39 R12: main's panic catch: crash.log already has the record (panic hook); show the message box once
/// (not when an [`error_box`] was already shown, not headless) and exit with an error code
pub fn main_panicked(p: &(dyn std::any::Any + Send), headless: bool) -> ! {
    let msg = main_panic_message(&panic_payload_text(p));
    eprintln!("game: {msg}");
    flush_std();
    if !headless && !BOX_SHOWN.load(Relaxed) {
        error_box(&msg);
    }
    std::process::exit(101)
}

/// P39 R3: a blocking error message box for a fatal player-facing error (unsupported disc); Windows only
pub fn error_box(msg: &str) {
    BOX_SHOWN.store(true, Relaxed);
    #[cfg(windows)]
    win::error_box(msg);
    #[cfg(not(windows))]
    let _ = msg;
}

/// P39 R2: copy `r` into `w` until `r` ends, ignoring every write error: println!/eprintln! panic
/// ("failed printing to stdout") when their file write fails (disk full, a removed USB stick, a network
/// folder gone), so the shipped game prints into a pipe and this loop feeds game.log
pub fn drain(mut r: impl std::io::Read, mut w: impl std::io::Write) {
    let mut b = [0u8; 4096];
    loop {
        match r.read(&mut b) {
            Ok(0) => break,
            Ok(n) => {
                let _ = w.write_all(&b[..n]).and_then(|()| w.flush());
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
}

/// P39 R2: give the [`drain`] pipes (ship build) up to 0.5 s to reach game.log (clean exit, panic)
pub fn flush_std() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    #[cfg(windows)]
    win::wait_drained();
}

/// session markers in `dir`: report every `scooby-session-<pid>.txt` whose process is gone (no clean exit, no
/// record) and remove it, then write this run's marker. Returns (records to append, own marker path).
pub fn session_begin(dir: &Path, pid: u32, info: &str, alive: impl Fn(u32) -> bool) -> (Vec<String>, Option<PathBuf>) {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(p) = name.strip_prefix("scooby-session-").and_then(|n| n.strip_suffix(".txt")).and_then(|n| n.parse::<u32>().ok()) else {
            continue;
        };
        if p == pid || alive(p) {
            continue;
        }
        let body = std::fs::read_to_string(e.path()).unwrap_or_default();
        out.push(format!(
            "=== UNCLEAN EXIT pid {p}: the run ended without a clean exit and without a panic/exception record (abort, fast-fail, heap corruption, killed, or a PC crash/power loss)\n  {}\n",
            body.trim()
        ));
        let _ = std::fs::remove_file(e.path());
    }
    let m = dir.join(format!("scooby-session-{pid}.txt"));
    let started = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let ok = std::fs::write(&m, format!("started unix {started} {info}\n")).is_ok();
    (out, ok.then_some(m))
}

/// is process `pid` still running (Windows: OpenProcess + GetExitCodeProcess; elsewhere: assume yes)
pub fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        win::pid_alive(pid)
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        true
    }
}

#[cfg(windows)]
mod win {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(h: *mut u8, text: *const u16, caption: *const u16, ty: u32) -> i32;
    }

    /// P39 R3: see [`super::error_box`]
    pub fn error_box(msg: &str) {
        let w = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        // SAFETY: two NUL-terminated UTF-16 strings; MB_OK | MB_ICONERROR (0x10) | MB_SETFOREGROUND (0x10000)
        unsafe { MessageBoxW(std::ptr::null_mut(), w(msg).as_ptr(), w("Scooby-Doo! Mystery Mayhem").as_ptr(), 0x10 | 0x10000) };
    }

    #[repr(C)]
    struct ExceptionRecord { code: u32, flags: u32, rec: *mut ExceptionRecord, addr: *mut u8, nparams: u32, info: [usize; 15] }
    #[repr(C)]
    struct ExceptionPointers { rec: *mut ExceptionRecord, ctx: *mut u8 }
    unsafe extern "system" {
        fn SetUnhandledExceptionFilter(h: Option<unsafe extern "system" fn(*mut ExceptionPointers) -> i32>) -> usize;
        fn AddVectoredExceptionHandler(first: u32, h: unsafe extern "system" fn(*mut ExceptionPointers) -> i32) -> *mut u8;
        fn GetModuleHandleExW(flags: u32, addr: *const u8, m: *mut *mut u8) -> i32;
        fn GetModuleFileNameW(m: *mut u8, buf: *mut u16, n: u32) -> u32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut u8;
        fn GetExitCodeProcess(h: *mut u8, code: *mut u32) -> i32;
        fn CloseHandle(h: *mut u8) -> i32;
        fn SetConsoleCtrlHandler(h: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
        fn GetStdHandle(id: u32) -> *mut u8;
        fn SetStdHandle(id: u32, h: *mut u8) -> i32;
        fn CreatePipe(r: *mut *mut u8, w: *mut *mut u8, sa: *const u8, size: u32) -> i32;
        fn GetFileType(h: *mut u8) -> u32;
        fn GetFileInformationByHandle(h: *mut u8, info: *mut [u32; 13]) -> i32;
    }

    /// P37 R6: drain state of the [`guard_std`] pipes (for [`wait_drained`]): (blocked in a read, reads started).
    /// It used to PeekNamedPipe the read end, but CreatePipe handles are synchronous: the peek queues behind
    /// the drain thread's pending ReadFile and never returns while the pipe stays empty, so every clean exit /
    /// panic of a shipped game.exe with a disk-file stdout or stderr (the launcher's game.log) hung for good.
    pub(crate) struct DrainState {
        idle: std::sync::atomic::AtomicBool,
        reads: std::sync::atomic::AtomicU64,
    }
    static PIPES: std::sync::Mutex<Vec<std::sync::Arc<DrainState>>> = std::sync::Mutex::new(Vec::new());
    struct Tracked(std::fs::File, std::sync::Arc<DrainState>);
    impl std::io::Read for Tracked {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            use std::sync::atomic::Ordering::SeqCst;
            self.1.reads.fetch_add(1, SeqCst);
            self.1.idle.store(true, SeqCst);
            let r = self.0.read(b);
            self.1.idle.store(false, SeqCst);
            r
        }
    }

    /// P39 R2: a pipe whose read end a thread copies into `out` (see [`super::drain`]); returns the write
    /// end. A pipe write never fails while the thread lives, so the print macros can no longer panic on a
    /// full disk.
    pub fn spawn_drain(out: impl std::io::Write + Send + 'static, track: bool) -> Option<std::fs::File> {
        use std::os::windows::io::FromRawHandle;
        let (mut r, mut w) = (std::ptr::null_mut(), std::ptr::null_mut());
        // SAFETY: out-pointers to two handles; default security, 64 KB buffer
        if unsafe { CreatePipe(&mut r, &mut w, std::ptr::null(), 1 << 16) } == 0 {
            return None;
        }
        // SAFETY: r / w are fresh pipe handles owned by these Files (the read end lives in the thread forever)
        let (rf, wf) = unsafe { (std::fs::File::from_raw_handle(r as _), std::fs::File::from_raw_handle(w as _)) };
        let st = std::sync::Arc::new(DrainState { idle: false.into(), reads: 0.into() });
        if track {
            PIPES.lock().unwrap_or_else(|e| e.into_inner()).push(st.clone());
        }
        let ok = std::thread::Builder::new().name("stdio-drain".into()).spawn(move || super::drain(Tracked(rf, st), out)).is_ok();
        ok.then_some(wf)
    }

    /// until every guarded drain thread has sat in one read for 10 ms (= its pipe is empty: a read with
    /// bytes waiting returns at once; max 0.5 s), then a moment for the last file write
    pub fn wait_drained() {
        use std::sync::atomic::Ordering::SeqCst;
        let pipes = PIPES.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if pipes.is_empty() {
            return;
        }
        for _ in 0..50 {
            let snap: Vec<u64> = pipes.iter().map(|p| p.reads.load(SeqCst)).collect();
            std::thread::sleep(std::time::Duration::from_millis(10));
            if pipes.iter().zip(&snap).all(|(p, &n)| p.idle.load(SeqCst) && p.reads.load(SeqCst) == n) {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    /// P39 R2 (ship build): stdout / stderr that are disk files (game.log from the launcher or
    /// [`redirect_detached`]) are replaced by pipes drained into those files, ignoring write errors. One pipe
    /// when both are the same file (keeps the line order). Consoles / pipes are left alone.
    #[allow(dead_code)]
    pub fn guard_std() {
        use std::os::windows::io::{FromRawHandle, IntoRawHandle};
        const STD_OUTPUT: u32 = -11i32 as u32;
        const STD_ERROR: u32 = -12i32 as u32;
        // SAFETY: plain handle queries; FILE_TYPE_DISK = 1
        let disk = |id: u32| {
            let h = unsafe { GetStdHandle(id) };
            (!h.is_null() && h as isize != -1 && unsafe { GetFileType(h) } == 1).then_some(h)
        };
        // (volume serial, file index): two handles of one file (the launcher's try_clone)
        let id = |h: *mut u8| {
            let mut i = [0u32; 13];
            // SAFETY: valid handle + a 52-byte BY_HANDLE_FILE_INFORMATION
            (unsafe { GetFileInformationByHandle(h, &mut i) } != 0).then(|| (i[7], i[11], i[12]))
        };
        let (o, e) = (disk(STD_OUTPUT), disk(STD_ERROR));
        let same = matches!((o, e), (Some(a), Some(b)) if a == b || id(a).is_some_and(|x| Some(x) == id(b)));
        let guard = |h: *mut u8, ids: &[u32]| {
            // SAFETY: the original handle now belongs to the drain thread (std looks the std handle up per
            // write, so nothing else writes through it once it is replaced)
            let f = unsafe { std::fs::File::from_raw_handle(h as _) };
            if let Some(w) = spawn_drain(super::RedactLines::new(f), true) {
                let w = w.into_raw_handle() as *mut u8; // the process-lifetime write end
                for &i in ids {
                    // SAFETY: a valid pipe handle that is never closed
                    unsafe { SetStdHandle(i, w) };
                }
            }
        };
        match (o, e) {
            (Some(a), Some(_)) if same => guard(a, &[STD_OUTPUT, STD_ERROR]),
            _ => {
                if let Some(a) = o {
                    guard(a, &[STD_OUTPUT]);
                }
                if let Some(b) = e {
                    guard(b, &[STD_ERROR]);
                }
            }
        }
    }

    /// P37 R2: a windows-subsystem game.exe (dist, feature `ship`) started from Explorer / the launcher has no
    /// stdout/stderr: send both into `<log dir>\game.log` (the previous run's kept as game.prev.log) so the
    /// log lines survive. A console or redirected start keeps its handles (nothing changes).
    pub fn redirect_detached(dir: &std::path::Path) {
        const STD_OUTPUT: u32 = -11i32 as u32;
        const STD_ERROR: u32 = -12i32 as u32;
        // SAFETY: plain handle queries
        let (o, e) = unsafe { (GetStdHandle(STD_OUTPUT), GetStdHandle(STD_ERROR)) };
        let none = |h: *mut u8| h.is_null() || h as isize == -1;
        // P39 R10: GAME_LOG_FILE=path (QA / headless runs that have a console) forces the same redacted,
        // error-dropping game.log path into that file, so a headless run's log can be checked like a player's
        let forced = std::env::var_os("GAME_LOG_FILE").filter(|p| !p.is_empty());
        if forced.is_none() && (!none(o) || !none(e)) {
            return;
        }
        let f = match &forced {
            Some(p) => super::open_log_exclusive(std::path::Path::new(p)).ok(),
            None => open_game_log(dir),
        };
        let Some(f) = f else { return };
        let f = super::RedactLines::new(f);
        // P37 R5 (BUGS P39 R1): println!/eprintln! panic on a failed write (ERROR_DISK_FULL on game.log's disk).
        // stdout/stderr go into a pipe instead; a drain thread copies it to game.log and drops write errors,
        // so a full disk only loses log lines (a pipe write never fails while the reader lives).
        // P39 R12: a tracked pipe (spawn_drain), so flush_std / wait_drained let the drain thread write the last
        // lines before a process::exit (they were lost: e.g. the message before exit(1/3/101))
        let Some(w) = spawn_drain(f, true) else { return };
        use std::os::windows::io::IntoRawHandle;
        let h = w.into_raw_handle() as *mut u8; // leaked on purpose: the process-lifetime log pipe
        // SAFETY: a valid pipe handle that is never closed; std's stdout/stderr look the handle up per write
        unsafe {
            SetStdHandle(STD_OUTPUT, h);
            SetStdHandle(STD_ERROR, h);
        }
    }

    /// P39 R8: this run's `dir\game.log` (the previous run's renamed to game.prev.log), opened without write /
    /// delete sharing ([`open_log_exclusive`]): while a game runs, a second game (refused by the card lock, it
    /// closes at once) can neither rotate nor truncate the running game's log; it gets None (no log) instead.
    pub fn open_game_log(dir: &std::path::Path) -> Option<std::fs::File> {
        let log = dir.join("game.log");
        if let Err(e) = std::fs::rename(&log, dir.join("game.prev.log")) {
            if super::log_in_use(&e) {
                return None;
            }
        }
        super::open_log_exclusive(&log).ok()
    }

    pub fn pid_alive(pid: u32) -> bool {
        // SAFETY: PROCESS_QUERY_LIMITED_INFORMATION (0x1000); handle closed below
        let h = unsafe { OpenProcess(0x1000, 0, pid) };
        if h.is_null() {
            return false; // gone (a marker is only ever written by our own game runs)
        }
        let mut code = 0u32;
        // SAFETY: valid handle + out-pointer
        let ok = unsafe { GetExitCodeProcess(h, &mut code) } != 0;
        // SAFETY: handle from OpenProcess
        unsafe { CloseHandle(h) };
        ok && code == 259 // STILL_ACTIVE
    }

    /// fatal failures that skip the unhandled-exception filter (STATUS_HEAP_CORRUPTION, STACK_BUFFER_OVERRUN /
    /// fast-fail, FAIL_FAST_EXCEPTION). No allocation (the heap may be corrupt): fixed text, pre-opened file.
    static FATAL_FILE: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();

    unsafe extern "system" fn fatal(p: *mut ExceptionPointers) -> i32 {
        // SAFETY: valid EXCEPTION_POINTERS from the OS
        let code = unsafe { (*(*p).rec).code };
        if matches!(code, 0xc000_0374 | 0xc000_0409 | 0xc000_0602)
            && let Some(mut f) = FATAL_FILE.get()
        {
            use std::io::Write;
            let mut buf = [0u8; 192];
            let mut c = std::io::Cursor::new(&mut buf[..]);
            let _ = write!(
                c,
                "=== FATAL pid {} code {code:#x} (heap corruption / fast-fail)\n  frame {}\n",
                std::process::id(),
                crate::engine::runtime::TRACE_FRAME.load(std::sync::atomic::Ordering::Relaxed)
            );
            let n = c.position() as usize;
            let _ = f.write_all(&buf[..n]);
        }
        0 // EXCEPTION_CONTINUE_SEARCH
    }

    unsafe extern "system" fn console_ctrl(kind: u32) -> i32 {
        let what = match kind {
            0 => "Ctrl+C",
            1 => "Ctrl+Break",
            2 => "console window closed",
            5 => "user logoff",
            6 => "system shutdown",
            _ => "console event",
        };
        super::append(&format!("=== CONSOLE EXIT pid {}: {what}\n  {}\n", std::process::id(), super::context()));
        super::clean_exit();
        0 // not handled: default action (the process ends)
    }

    pub fn install_game() {
        if let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(super::path()) {
            let _ = FATAL_FILE.set(f);
        }
        // SAFETY: registering plain function pointers
        unsafe {
            AddVectoredExceptionHandler(1, fatal);
            SetConsoleCtrlHandler(Some(console_ctrl), 1);
        }
    }

    /// "module.dll+0xoff" for a code address
    pub fn module_off(a: usize) -> String {
        let mut m = std::ptr::null_mut();
        // SAFETY: FROM_ADDRESS (4) | UNCHANGED_REFCOUNT (2); out-pointer valid
        if unsafe { GetModuleHandleExW(6, a as *const u8, &mut m) } == 0 {
            return format!("{a:#x} (no module)");
        }
        let mut buf = [0u16; 260];
        // SAFETY: buffer length passed
        let n = unsafe { GetModuleFileNameW(m, buf.as_mut_ptr(), 260) } as usize;
        let name = String::from_utf16_lossy(&buf[..n]);
        let name = name.rsplit(['\\', '/']).next().unwrap_or("").to_string();
        format!("{name}+{:#x}", a - m as usize)
    }

    fn report(tag: &str, p: *mut ExceptionPointers) {
        // SAFETY: called by the OS with a valid EXCEPTION_POINTERS
        let r = unsafe { &*(*p).rec };
        let kind = match (r.code, r.info[0]) {
            (0xc000_0005, 0) => "read",
            (0xc000_0005, 1) => "write",
            (0xc000_0005, _) => "exec",
            _ => "-",
        };
        let mut s = format!(
            "=== {tag} pid {} code {:#x} at {:p} = {} {kind} addr {:#x} thread {:?}\n  {}\n",
            std::process::id(),
            r.code,
            r.addr,
            module_off(r.addr as usize),
            r.info[1],
            std::thread::current().name(),
            super::context()
        );
        // P32 R98: the GPRs (x64 CONTEXT +0x78 Rax .. +0xf0 R15, +0xf8 Rip), so a fault in safe code
        // (the mat_mul null read) shows which register held the bad pointer and what the others held
        #[cfg(target_arch = "x86_64")]
        // SAFETY: the OS-provided CONTEXT record is >= 0x100 bytes on x64; unaligned reads
        unsafe {
            let c = (*p).ctx;
            if !c.is_null() {
                const N: [&str; 17] = ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15", "rip"];
                s += "  regs";
                for (i, n) in N.iter().enumerate() {
                    s += &format!(" {n}={:#x}", (c.add(0x78 + i * 8) as *const u64).read_unaligned());
                }
                s += "\n";
            }
        }
        // no backtrace on a stack overflow (no stack left to symbolize on)
        if r.code != 0xc000_00fd {
            s += &format!("{}\n", std::backtrace::Backtrace::force_capture());
        }
        super::append(&s);
    }

    unsafe extern "system" fn unhandled(p: *mut ExceptionPointers) -> i32 {
        report("UNHANDLED EXCEPTION", p);
        0 // EXCEPTION_CONTINUE_SEARCH: default handling (process dies as before)
    }

    unsafe extern "system" fn first_chance(p: *mut ExceptionPointers) -> i32 {
        // SAFETY: as above
        let c = unsafe { (*(*p).rec).code };
        if matches!(c, 0xc000_0005 | 0xc000_00fd | 0xc000_001d | 0xc000_0096) {
            report("FIRST-CHANCE", p);
        }
        0
    }

    pub fn install() {
        // SAFETY: registering plain function pointers
        unsafe { SetUnhandledExceptionFilter(Some(unhandled)) };
        if std::env::var_os("P17_CRASHDBG").is_some() {
            // SAFETY: as above
            unsafe { AddVectoredExceptionHandler(1, first_chance) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P39 R12: Bevy renderer-init panics (no GPU, surface, device features) get the GPU text, others the
    /// internal-error text; payloads of both panic! forms are read
    #[test]
    fn p39_main_panic_gpu_message() {
        let no_gpu = "Unable to find a GPU! Make sure you have installed required drivers!";
        let m = main_panic_message(no_gpu);
        assert!(m.contains("could not start the graphics") && m.contains(no_gpu), "{m}");
        let dev = "called `Result::unwrap()` on an `Err` value: RequestDeviceError { inner: Core(UnsupportedFeature) }";
        assert!(main_panic_message(dev).contains("graphics card"));
        assert!(main_panic_message("index out of bounds").contains("internal error"));
        assert!(gpu_init_message(&"x".repeat(5000)).len() < 700);
        let p: Box<dyn std::any::Any + Send> = Box::new("lit");
        assert_eq!(panic_payload_text(&*p), "lit");
        let p: Box<dyn std::any::Any + Send> = Box::new(format!("f{}", 1));
        assert_eq!(panic_payload_text(&*p), "f1");
        let p: Box<dyn std::any::Any + Send> = Box::new(5u8);
        assert_eq!(panic_payload_text(&*p), "");
    }

    /// P39 R11: a non-Unicode argument (unpaired surrogate in a Windows path) is kept lossy, never a panic
    #[cfg(windows)]
    #[test]
    fn p39_non_unicode_args_no_panic() {
        use std::os::windows::ffi::OsStringExt;
        let bad = std::ffi::OsString::from_wide(&[0x43, 0x3a, 0x5c, 0xd800, 0x2e, 0x69, 0x73, 0x6f]);
        assert!(bad.clone().into_string().is_err());
        let v = lossy_args(["--iso".into(), bad, "Jürgen Ñ 测试".into()].into_iter());
        assert_eq!(v, ["--iso", "C:\\\u{fffd}.iso", "Jürgen Ñ 测试"]);
    }

    /// P37 R1: a run that vanished without a record is reported once by the next start; live runs and our
    /// own marker are left alone; clean_exit-style removal leaves nothing to report.
    #[test]
    fn p37_session_marker_reports_unclean_exit_once() {
        let dir = std::env::temp_dir().join(format!("p37_session_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // run 100 dies silently, run 200 is still running
        let (r, m100) = session_begin(&dir, 100, "version t args [\"EP1/L01\"]", |_| false);
        assert!(r.is_empty());
        let (r, m200) = session_begin(&dir, 200, "b", |p| p == 100);
        assert!(r.is_empty(), "100 is alive here: {r:?}");
        let (r, m300) = session_begin(&dir, 300, "c", |p| p == 200);
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].starts_with("=== UNCLEAN EXIT pid 100:") && r[0].contains("EP1/L01"), "{}", r[0]);
        assert!(!m100.unwrap().exists() && m200.as_ref().unwrap().exists());
        // 300 exits cleanly, 200 ended silently
        std::fs::remove_file(m300.unwrap()).unwrap();
        let (r, _) = session_begin(&dir, 400, "d", |_| false);
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].contains("pid 200"));
        let (r, _) = session_begin(&dir, 400, "d", |_| false);
        assert!(r.is_empty(), "own marker never reported");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(version().starts_with(env!("CARGO_PKG_VERSION")));
    }

    /// P39 R5: fatal runtime errors keep the FIRST player message for main's message box; the GPU text is capped
    #[test]
    fn p39_fatal_exit_message() {
        fatal("P39 TEST", "first", "d1");
        fatal("P39 TEST", "second", "d2");
        assert_eq!(take_exit_message().as_deref(), Some("first"));
        assert_eq!(take_exit_message(), None);
        let m = gpu_lost_message(&"z".repeat(9000));
        assert!(m.contains("graphics device stopped responding") && m.contains("driver") && m.len() < 700, "{m}");
        assert!(!gpu_lost_message("  ").contains("()"));
    }

    /// P39 R2: the stdout/stderr guard: pipe writes never fail even when every write to the log file
    /// fails (disk full); with a writable log the bytes arrive complete and in order
    #[cfg(windows)]
    #[test]
    fn p39_print_guard_ignores_write_errors() {
        use std::io::Write;
        struct Full;
        impl Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("There is not enough space on the disk. (os error 112)"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("disk full"))
            }
        }
        let mut w = win::spawn_drain(Full, false).expect("pipe");
        for i in 0..2000 {
            writeln!(w, "line {i} {}", "x".repeat(100)).expect("a pipe write must not fail");
        }
        #[derive(Clone)]
        struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for Shared {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let got = Shared(Default::default());
        let mut w2 = win::spawn_drain(got.clone(), false).expect("pipe");
        let want: String = (0..500).map(|i| format!("ünïcødé line {i}\n")).collect();
        w2.write_all(want.as_bytes()).unwrap();
        for _ in 0..200 {
            if got.0.lock().unwrap().len() >= want.len() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(String::from_utf8(got.0.lock().unwrap().clone()).unwrap(), want);
        // drain() itself on a failing writer: consumes everything and returns at the end of the input
        drain(std::io::Cursor::new(vec![7u8; 100_000]), Full);
    }

    /// P39 R9: the ISO folder / saves / settings outside the profile are redacted with their label, longest
    /// prefix first; a drive root is kept; the file name stays
    #[test]
    fn p39_redact_game_paths() {
        redact_also(std::path::Path::new(r"Q:\P39 Jürgen 测试\Games"), "<ISO>");
        redact_also(std::path::Path::new(r"Q:\P39 Jürgen 测试\Games\saves"), "<SAVES>");
        redact_also(std::path::Path::new("Q:\\"), "<ROOT>");
        let r = redact(r"iso q:\p39 Jürgen 测试\Games\Scooby.iso card Q:/P39 Jürgen 测试/Games/saves/memcard1 other Q:\x.iso");
        assert_eq!(r, r"iso <ISO>\Scooby.iso card <SAVES>/memcard1 other Q:\x.iso");
    }

    /// P39 R8: the profile prefix (any case, `\` `/` and Debug `\\` spelling, non-ASCII + spaces) and the machine
    /// name are redacted; a longer user name sharing the prefix and the rest of the path are kept
    #[test]
    fn p39_redact_profile_paths() {
        let p = vec![
            (r"C:\Users\Jürgen Ñ 测试".to_string(), "%USERPROFILE%".to_string()),
            (r"\\JÜRGEN-PC".to_string(), r"\\%COMPUTERNAME%".to_string()),
        ];
        let s = concat!(
            r"iso C:\Users\Jürgen Ñ 测试\OneDrive\Games\Scooby.iso; ",
            r#"args ["--iso", "c:\\users\\Jürgen Ñ 测试\\Desktop\\a.iso"] "#,
            r"C:/Users/Jürgen Ñ 测试/x ",
            r"other C:\Users\Jürgen Ñ 测试2\y ",
            r"unc \\JÜRGEN-PC\share\b.iso end C:\Users\Jürgen Ñ 测试",
        );
        let r = redact_with(s, &p);
        assert_eq!(
            r,
            concat!(
                r"iso %USERPROFILE%\OneDrive\Games\Scooby.iso; ",
                r#"args ["--iso", "%USERPROFILE%\\Desktop\\a.iso"] "#,
                r"%USERPROFILE%/x ",
                r"other C:\Users\Jürgen Ñ 测试2\y ",
                r"unc \\%COMPUTERNAME%\share\b.iso end %USERPROFILE%",
            )
        );
        assert!(matches!(redact_with("frame 12 level \"EP1/L01\"", &p), std::borrow::Cow::Borrowed(_)));
        // the line writer: split writes, whole lines through redact, a newline-free 5 KB blob passes as is
        use std::io::Write;
        let mut w = RedactLines::new(Vec::new());
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let line = format!("panicked at {home}\\x.rs\n");
        let (a, b) = line.split_at(line.len() / 2);
        w.write_all(a.as_bytes()).unwrap();
        w.write_all(b.as_bytes()).unwrap();
        w.write_all(&[b'z'; 5000]).unwrap();
        let out = String::from_utf8(w.w).unwrap();
        assert!(out.starts_with("panicked at ") && out.len() >= 5000 && w.buf.is_empty(), "{}", &out[..60.min(out.len())]);
        if home.len() > 3 {
            assert!(out.starts_with("panicked at %USERPROFILE%\\x.rs\n"), "{}", &out[..60]);
        }
    }

    /// P39 R8: a running game's game.log (open_game_log) cannot be rotated or truncated by a second game
    /// (gets None, the log keeps its lines) or a second launcher Play (log_in_use); after the game ends the
    /// next run rotates it to game.prev.log as before
    #[cfg(windows)]
    #[test]
    fn p39_second_game_keeps_first_log() {
        use std::io::Write;
        let d = std::env::temp_dir().join(format!("p39_gamelog_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("game.log"), "older run\n").unwrap();
        let mut first = win::open_game_log(&d).expect("first game");
        assert_eq!(std::fs::read_to_string(d.join("game.prev.log")).unwrap(), "older run\n");
        writeln!(first, "first game line").unwrap();
        assert!(win::open_game_log(&d).is_none(), "second game must not get the log");
        let e = open_log_exclusive(&d.join("game.log")).unwrap_err();
        assert!(log_in_use(&e), "{e:?}");
        assert_eq!(std::fs::read_to_string(d.join("game.log")).unwrap(), "first game line\n", "readable + intact");
        assert_eq!(std::fs::read_to_string(d.join("game.prev.log")).unwrap(), "older run\n");
        drop(first);
        let _next = win::open_game_log(&d).expect("after the first game ended");
        assert_eq!(std::fs::read_to_string(d.join("game.prev.log")).unwrap(), "first game line\n");
        drop(_next);
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// P39 R8: create/truncate a log file (game.log) shared for reading only: a running game's log can be read
/// (Notepad, the launcher's exit report) but not truncated, renamed or deleted by a second game / launcher
/// ([`log_in_use`] on their error). Elsewhere a plain create.
pub fn open_log_exclusive(p: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(windows)]
    std::os::windows::fs::OpenOptionsExt::share_mode(&mut o, 1); // FILE_SHARE_READ
    o.open(p)
}

/// P39 R8: the error of opening / renaming a log that a running game holds ([`open_log_exclusive`])
pub fn log_in_use(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(32) // ERROR_SHARING_VIOLATION
}

/// P39 R8: privacy for bug reports (README: attach game.log + crash.log to an issue): the user's profile
/// folder (`C:\Users\<name>`, in `\`, `/` and Debug-escaped `\\` spelling, any letter case), a
/// LOCALAPPDATA outside it and the machine name as a UNC host are written as `%USERPROFILE%`,
/// `%LOCALAPPDATA%`, `\\%COMPUTERNAME%`. The rest of the path (ISO name, OneDrive, folders) stays for the report.
pub fn redact(s: &str) -> std::borrow::Cow<'_, str> {
    static P: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    let p = P.get_or_init(|| {
        let v = |k: &str| std::env::var(k).ok().map(|s| s.trim_end_matches(['\\', '/']).to_string()).filter(|s| s.len() > 3);
        let mut p = Vec::new();
        let home = v("USERPROFILE");
        if let Some(la) = v("LOCALAPPDATA").filter(|la| home.as_ref().is_none_or(|h| !la.to_ascii_lowercase().starts_with(&h.to_ascii_lowercase()))) {
            p.push((la, "%LOCALAPPDATA%".to_string()));
        }
        if let Some(h) = home {
            p.push((h, "%USERPROFILE%".to_string()));
        }
        if let Some(c) = std::env::var("COMPUTERNAME").ok().filter(|c| c.len() > 1) {
            p.push((format!("\\\\{c}"), "\\\\%COMPUTERNAME%".to_string()));
        }
        p
    });
    let extra = EXTRA.lock().map(|e| e.clone()).unwrap_or_default();
    if extra.is_empty() {
        return redact_with(s, p);
    }
    // P39 R9: the longest prefix first (a saves dir inside the profile becomes `<SAVES>`, not `%LOCALAPPDATA%\..`)
    let mut all: Vec<(String, String)> = extra.into_iter().chain(p.iter().cloned()).collect();
    all.sort_by_key(|(pre, _)| std::cmp::Reverse(pre.len()));
    redact_with(s, &all).into_owned().into()
}

static EXTRA: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

/// P39 R9: also redact the folder `dir` (as given and absolute) as `label` in game.log / crash.log: the ISO's
/// folder, the memory-card dir, the settings folder can sit outside the profile under a personal name
/// (`D:\Jürgen\Games`). A drive root or a cwd-relative bare file name is not redacted.
pub fn redact_also(dir: &std::path::Path, label: &str) {
    let mut forms = vec![dir.to_path_buf()];
    if let Ok(a) = std::path::absolute(dir) {
        forms.push(a);
    }
    let Ok(mut e) = EXTRA.lock() else { return };
    for f in forms {
        let s = f.display().to_string().trim_end_matches(['\\', '/']).to_string();
        // `D:` / `.` / `` are no personal names; keep them readable
        if s.len() <= 3 || s == ".." || e.iter().any(|(p, _)| p.eq_ignore_ascii_case(&s)) {
            continue;
        }
        e.push((s, label.to_string()));
    }
}

/// P39 R9: [`redact_also`] for the paths a game run uses: the ISO's folder `<ISO>`, GAME_SAVEDIR `<SAVES>`,
/// the settings.toml folder `<SETTINGS>`
pub fn redact_game_paths(iso: &std::path::Path) {
    if let Some(d) = iso.parent() {
        redact_also(d, "<ISO>");
    }
    if let Some(d) = std::env::var_os("GAME_SAVEDIR").filter(|d| !d.is_empty()) {
        redact_also(std::path::Path::new(&d), "<SAVES>");
    }
    if let Some(d) = crate::engine::settings::paths().0.parent() {
        redact_also(d, "<SETTINGS>");
    }
}

/// [`redact`] with explicit (prefix, replacement) pairs; a prefix only matches as a whole path component
pub fn redact_with<'a>(s: &'a str, prefixes: &[(String, String)]) -> std::borrow::Cow<'a, str> {
    let mut out = std::borrow::Cow::Borrowed(s);
    for (pre, rep) in prefixes {
        for (pre, rep) in [(pre.clone(), rep.clone()), (pre.replace('\\', "/"), rep.replace('\\', "/")), (pre.replace('\\', "\\\\"), rep.replace('\\', "\\\\"))] {
            let lower = out.to_ascii_lowercase();
            let needle = pre.to_ascii_lowercase();
            if needle.is_empty() || !lower.contains(&needle) {
                continue;
            }
            let mut r = String::with_capacity(out.len());
            let mut last = 0;
            for (i, _) in lower.match_indices(&needle) {
                let end = i + needle.len();
                let next = lower[end..].chars().next();
                if i < last || next.is_some_and(|c| c.is_alphanumeric() || "._-$~".contains(c)) {
                    continue;
                }
                r.push_str(&out[last..i]);
                r.push_str(&rep);
                last = end;
            }
            r.push_str(&out[last..]);
            out = std::borrow::Cow::Owned(r);
        }
    }
    out
}

/// P39 R8: a writer in front of game.log that writes whole lines through [`redact`] (a line longer than 4 KB
/// goes out as is once the buffer is full)
pub struct RedactLines<W> {
    w: W,
    buf: Vec<u8>,
}

impl<W> RedactLines<W> {
    pub fn new(w: W) -> Self {
        Self { w, buf: Vec::new() }
    }
}

impl<W: std::io::Write> std::io::Write for RedactLines<W> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(b);
        let end = match self.buf.iter().rposition(|&c| c == b'\n') {
            Some(i) => i + 1,
            None if self.buf.len() >= 4096 => self.buf.len(),
            None => return Ok(b.len()),
        };
        let chunk: Vec<u8> = self.buf.drain(..end).collect();
        // the bytes are taken either way: a failed write (disk full) only loses these lines
        let _ = match std::str::from_utf8(&chunk) {
            Ok(s) => self.w.write_all(redact(s).as_bytes()),
            Err(_) => self.w.write_all(&chunk),
        };
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.w.flush()
    }
}

/// P37 R5: copy `r` into `w` until EOF / a read error; write errors (disk full) are dropped, never fatal.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn drain_ignoring_errors(r: &mut impl std::io::Read, w: &mut impl std::io::Write) {
    let mut buf = [0u8; 8192];
    while let Ok(n @ 1..) = r.read(&mut buf) {
        let _ = w.write_all(&buf[..n]);
    }
}

#[cfg(test)]
mod p37_drain_tests {
    /// a full disk: every write fails; the pipe writer side must still accept everything (no print panic)
    #[test]
    fn p37_log_pipe_survives_failing_disk() {
        struct DiskFull;
        impl std::io::Write for DiskFull {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("disk full"))
            }
        }
        let (mut r, mut w) = std::io::pipe().unwrap();
        let t = std::thread::spawn(move || super::drain_ignoring_errors(&mut r, &mut DiskFull));
        use std::io::Write;
        for _ in 0..1000 {
            w.write_all(&[b'x'; 1000]).expect("pipe write must not fail while the drain lives"); // 1 MB > pipe buffer
        }
        drop(w);
        t.join().unwrap();
    }

    /// P37 R6: wait_drained (every clean exit / panic of the shipped game) must return with an idle, empty
    /// guarded pipe (it hung forever: PeekNamedPipe queued behind the drain thread's blocked ReadFile) and
    /// still deliver what was written before it
    #[cfg(windows)]
    #[test]
    fn p37_wait_drained_returns_on_idle_pipe() {
        #[derive(Clone)]
        struct Sink(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl std::io::Write for Sink {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let sink = Sink(Default::default());
        let mut w = super::win::spawn_drain(sink.clone(), true).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50)); // the drain thread is now blocked in its read
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            super::win::wait_drained();
            let _ = tx.send(());
        });
        rx.recv_timeout(std::time::Duration::from_secs(3)).expect("wait_drained hung on an idle pipe");
        use std::io::Write;
        w.write_all(b"last line
").unwrap();
        super::win::wait_drained();
        assert_eq!(&sink.0.lock().unwrap()[..], b"last line
");
    }
}
