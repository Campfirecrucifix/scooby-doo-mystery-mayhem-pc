//! P32 R73: the always-on crash log (scooby_recomp::crashlog) pins a native crash: a child copy of this test exe
//! takes a real access violation (null write) inside a fake game-object update; the parent checks the crash log
//! holds the code, module+offset, the access kind and the frame/level/object context.

#[test]
fn p32_r73_crash_child() {
    // only acts as the child of p32_r73_crashlog_pins_access_violation
    if std::env::var_os("P32_R73_CHILD").is_none() {
        return;
    }
    scooby_recomp::crashlog::install();
    scooby_recomp::crashlog::set_level("EP3/L05");
    scooby_recomp::crashlog::OBJ.store(0x9d, std::sync::atomic::Ordering::Relaxed);
    let p = std::hint::black_box(std::ptr::null_mut::<u32>());
    // SAFETY: deliberately not safe - the crash under test
    unsafe { p.write_volatile(1) };
}

#[cfg(windows)]
#[test]
fn p32_r73_crashlog_pins_access_violation() {
    let log = std::env::temp_dir().join(format!("p32_r73_crash_{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log);
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["p32_r73::p32_r73_crash_child", "--exact", "--include-ignored", "--test-threads=1", "--nocapture"])
        .env("P32_R73_CHILD", "1")
        .env("SCOOBY_CRASHLOG", &log)
        .output()
        .unwrap();
    let s = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_file(&log);
    println!("child {:?}\n{s}", out.status);
    assert_eq!(out.status.code().map(|c| c as u32), Some(0xc000_0005), "child must die of the AV");
    assert!(s.contains("UNHANDLED EXCEPTION") && s.contains("code 0xc0000005"), "{s}");
    assert!(s.contains(" write addr 0x0 "), "{s}");
    assert!(s.contains(".exe+0x"), "module+offset: {s}");
    assert!(s.contains("level \"EP3/L05\" obj 0x9d"), "{s}");
}
