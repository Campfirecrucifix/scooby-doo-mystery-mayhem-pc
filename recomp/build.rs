// P34: the launcher uses the comctl32 v6 (visual styles) controls: embed the manifest dependency.
// Icon + version resource (res/app.rc) for the launcher and game.exe, compiled with the Windows
// SDK rc.exe when it is found (RC env var, else the newest Windows Kits 10 x64 rc.exe); skipped
// with a warning otherwise.
use std::path::PathBuf;

fn find_rc() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RC") {
        return Some(p.into());
    }
    let kits = PathBuf::from(std::env::var_os("ProgramFiles(x86)")?).join("Windows Kits/10/bin");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&kits).ok()?.flatten().map(|e| e.path().join("x64/rc.exe")).filter(|p| p.is_file()).collect();
    v.sort();
    v.pop()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=res/app.rc");
    println!("cargo:rerun-if-changed=res/scooby.ico");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    println!("cargo:rustc-link-arg-bin=launcher=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bin=launcher=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe not found: no icon/version resource");
        return;
    };
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ver = std::env::var("CARGO_PKG_VERSION").unwrap();
    let n: Vec<&str> = ver.split(['.', '-']).collect();
    let inc = rc.parent().and_then(|b| b.parent()).map(|b| b.to_path_buf());
    for (bin, desc, file) in [("launcher", "Scooby-Doo! Mystery Mayhem launcher", "Scooby-Doo Mystery Mayhem.exe"), ("game", "Scooby-Doo! Mystery Mayhem", "game.exe")] {
        let res = out.join(format!("{bin}.res"));
        let mut c = std::process::Command::new(&rc);
        c.arg("/nologo").arg(format!("/dVER_MAJ={}", n[0])).arg(format!("/dVER_MIN={}", n.get(1).unwrap_or(&"0"))).arg(format!("/dVER_PAT={}", n.get(2).unwrap_or(&"0")));
        c.arg(format!("/dVER_STR=\"{ver}\"")).arg(format!("/dVER_DESC=\"{desc}\"")).arg(format!("/dVER_FILE=\"{file}\""));
        // rc.exe finds winver.h through INCLUDE; outside a VS prompt add the SDK um/shared dirs
        if let Some(inc) = inc.as_ref().and_then(|b| b.file_name().map(|v| b.parent().unwrap().parent().unwrap().join("Include").join(v))) {
            c.arg("/i").arg(inc.join("um")).arg("/i").arg(inc.join("shared"));
        }
        c.arg("/fo").arg(&res).arg("res/app.rc");
        match c.status() {
            Ok(s) if s.success() => println!("cargo:rustc-link-arg-bin={bin}={}", res.display()),
            r => println!("cargo:warning=rc.exe failed for {bin}: {r:?}"),
        }
    }
}
