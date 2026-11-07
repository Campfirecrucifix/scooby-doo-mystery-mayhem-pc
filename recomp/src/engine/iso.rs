//! Game file system: replaces the PS2 DTFopen/DTFsize/DTFread layer (cdvd / host0:) with an
//! ISO-9660 reader over the original disc image (rule 2: platform replacement, not 1:1).
//!
//! The engine's file names are DOS-style ("LEVEL\EP1\L01\GEN\EP1L01.P2R"); lookup is
//! case-insensitive, accepts '\\' or '/', and ignores the ";1" version suffix, like the PS2 cdvd
//! driver. Whole files are read in one call (`read`), which is the only access pattern the
//! engine's loaders use; this is done at level-load time, never inside a per-frame system.

use super::error::EngineError;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const SECTOR: u64 = 2048;
/// P39: no real directory is anywhere near this (the game disc's biggest is a few sectors)
const MAX_DIR: u32 = 16 << 20;

/// Directory of a disc image: upper-case path -> (lba, size).
#[derive(Debug)]
pub struct IsoFs {
    path: PathBuf,
    files: HashMap<String, (u32, u32)>,
    /// P34 R6: sector layout of the image file: (bytes per sector, offset of the 2048 user bytes).
    /// 2048/0 = .iso; 2352/24 = raw MODE2/2352 .bin/.img; 2352/16 = raw MODE1/2352.
    layout: (u64, u64),
}

/// P34 R6: a .cue sheet names its data track file (`FILE "x.bin" BINARY`, relative to the sheet);
/// any other path is the image itself.
pub fn resolve_image(path: &Path) -> std::io::Result<PathBuf> {
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("cue")) {
        return Ok(path.to_path_buf());
    }
    let cue = std::fs::read_to_string(path)?;
    let file = cue.lines().find_map(|l| {
        let l = l.trim();
        l.get(..5).filter(|k| k.eq_ignore_ascii_case("FILE ")).map(|_| l[5..].trim())
    });
    let name = file.map(|f| match f.strip_prefix('"') {
        Some(q) => q.split('"').next().unwrap_or("").to_string(),
        None => f.split_whitespace().next().unwrap_or("").to_string(),
    });
    match name.filter(|n| !n.is_empty()) {
        Some(n) => Ok(path.parent().unwrap_or(Path::new(".")).join(n)),
        None => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "the .cue sheet names no FILE")),
    }
}

fn norm(p: &str) -> String {
    let p = p.replace('\\', "/").to_ascii_uppercase();
    let p = p.trim_start_matches('/');
    match p.rfind(';') {
        Some(i) => p[..i].to_string(),
        None => p.to_string(),
    }
}

impl IsoFs {
    /// Reads the primary volume descriptor (sector 16) and walks the whole directory tree.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let io = |e: std::io::Error| EngineError::Io(e.to_string());
        let path = resolve_image(path.as_ref()).map_err(io)?;
        let mut f = File::open(&path).map_err(io)?;
        let layout = [(SECTOR, 0), (2352, 24), (2352, 16)]
            .into_iter()
            .find(|&l| read_at(&mut f, l, 16, 6).is_ok_and(|v| &v[1..6] == b"CD001"))
            .ok_or_else(|| EngineError::Format("not an ISO-9660 image".into()))?;
        let pvd = read_at(&mut f, layout, 16, SECTOR as usize).map_err(io)?;
        let root = &pvd[156..156 + 34];
        let bad = |m: &str| EngineError::Format(format!("damaged disc image: {m}"));
        let mut files = HashMap::new();
        // P39: a damaged / foreign image must give an error, never a panic, an endless walk or a
        // multi-GB allocation (the launcher scans every *.iso / *.bin it finds)
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![(String::new(), le32(root, 2), le32(root, 10))];
        while let Some((prefix, lba, size)) = stack.pop() {
            if !seen.insert(lba) || seen.len() > 100_000 {
                continue;
            }
            if size > MAX_DIR {
                return Err(bad(&format!("directory '{prefix}' is {size} bytes")));
            }
            let dir = read_at(&mut f, layout, lba as u64, size as usize).map_err(io)?;
            let mut off = 0usize;
            while off < dir.len() {
                let len = dir[off] as usize;
                if len == 0 {
                    // records never cross a sector: skip to the next one
                    off = (off / SECTOR as usize + 1) * SECTOR as usize;
                    continue;
                }
                let rec = &dir[off..(off + len).min(dir.len())];
                off += len;
                let Some(&nlen) = rec.get(32) else { continue };
                let Some(name) = rec.get(33..33 + nlen as usize) else { continue };
                if !(nlen == 1 && (name[0] == 0 || name[0] == 1)) {
                    let name = norm(&String::from_utf8_lossy(name));
                    let full = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
                    let (elba, esize) = (le32(rec, 2), le32(rec, 10));
                    if rec[25] & 2 != 0 {
                        stack.push((full, elba, esize));
                    } else {
                        files.insert(full, (elba, esize));
                    }
                }
            }
        }
        // J18: this is the SCEE demo disc: the launcher (SCEE_DD/CONFIG.DD2) runs EXE.1 for RUSSIAN and
        // EXE.2 for ENGLISH. EXE.2 is EXE.1 with only its root dir literals changed (33 bytes:
        // "LEVE_", "GLOBA_", "STREA_"); the packs are identical, the TEXT.TX/audio differ. We run
        // the English executable: its root dirs answer for the engine's LEVEL/GLOBAL/STREAM names.
        // GAME_LANG=RU keeps the EXE.1 (Russian) roots.
        if std::env::var("GAME_LANG").map_or(true, |v| !v.eq_ignore_ascii_case("RU")) {
            for (en, ru) in [("LEVE_/", "LEVEL/"), ("GLOBA_/", "GLOBAL/"), ("STREA_/", "STREAM/")] {
                let alt: Vec<(String, (u32, u32))> = files.iter().filter_map(|(k, v)| k.strip_prefix(en).map(|r| (format!("{ru}{r}"), *v))).collect();
                files.extend(alt);
            }
        }
        // P33 R10: not in lib unit tests: the first opened image wins, and the p39 tests open throw-away images
        // (then deleted), which made lit_strings_present fail (module 014a381f not found) whenever they ran first.
        if !cfg!(test) {
            super::gamedata::set_iso_path(&path);
        }
        Ok(Self { path, files, layout })
    }

    /// DTFsize: file size, or -1 when missing.
    pub fn size(&self, name: &str) -> i32 {
        self.files.get(&norm(name)).map_or(-1, |e| e.1 as i32)
    }

    /// DTFopen + DTFread(whole) + DTFclose.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, EngineError> {
        let &(lba, size) = self.files.get(&norm(name)).ok_or_else(|| EngineError::FileNotFound(name.to_string()))?;
        let once = || {
            #[cfg(test)]
            if FAIL_READS.with(|n| n.get() > 0 && { n.set(n.get() - 1); true }) {
                return Err(std::io::Error::other("injected: the device is not ready (os error 21)"));
            }
            let mut f = File::open(&self.path)?;
            read_at(&mut f, self.layout, lba as u64, size as usize)
        };
        // P39 R7: a USB / network disc can fail one read transiently (drive spin-up, a Wi-Fi blip): one retry
        // after a short delay before the read error ends the game with the disc-gone message
        once()
            .or_else(|e| {
                log::warn!("disc read {name}: {e}; retrying once");
                std::thread::sleep(std::time::Duration::from_millis(READ_RETRY_MS));
                once()
            })
            .map_err(|e| EngineError::Io(e.to_string()))
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|s| s.as_str())
    }
}

/// P39 R7: delay before the one retry of a failed disc read
pub const READ_RETRY_MS: u64 = 300;

#[cfg(test)]
thread_local! {
    /// P39 R7: error-injection seam: the next N reads of [`IsoFs::read`] on this thread fail
    pub static FAIL_READS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// P39 R5: the player text when the disc image stops being readable mid-game (a USB drive / network share
/// removed, the file moved or locked): the level read fails, the game closes with this message box
pub fn disc_read_message(iso: &Path, detail: &str) -> String {
    let what = if iso.exists() { "can't be read any more" } else { "is gone" };
    format!(
        "The game disc image {what}:\n{}\n\nIf it is on a USB drive or network folder, reconnect it and start the game again. Your saved progress on the memory card is kept.\n\n({})",
        iso.display(),
        detail.chars().take(300).collect::<String>()
    )
}

/// little-endian u32 at `o` (0 past the end of a short record)
fn le32(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map_or(0, |v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
}

/// `len` user bytes from sector `lba` (2048 per sector, whatever the image's sector layout)
fn read_at(f: &mut File, (stride, data): (u64, u64), lba: u64, len: usize) -> std::io::Result<Vec<u8>> {
    // P39: a truncated image (or a bogus size) is an error before the allocation
    let nsec = (len as u64).div_ceil(SECTOR);
    let end = (lba + nsec.saturating_sub(1)) * stride + data + (len as u64 - nsec.saturating_sub(1) * SECTOR);
    if len > 0 && end > f.metadata()?.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the disc image is truncated (incomplete download or copy?)"));
    }
    let mut v = vec![0u8; len];
    if stride == SECTOR {
        f.seek(SeekFrom::Start(lba * SECTOR))?;
        f.read_exact(&mut v)?;
        return Ok(v);
    }
    for (i, chunk) in v.chunks_mut(SECTOR as usize).enumerate() {
        f.seek(SeekFrom::Start((lba + i as u64) * stride + data))?;
        f.read_exact(chunk)?;
    }
    Ok(v)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn rec(name: &[u8], lba: u32, size: u32, dir: bool) -> Vec<u8> {
        let mut r = vec![0u8; 33];
        r[2..6].copy_from_slice(&lba.to_le_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[25] = if dir { 2 } else { 0 };
        r[32] = name.len() as u8;
        r.extend_from_slice(name);
        if r.len() % 2 == 1 {
            r.push(0);
        }
        r[0] = r.len() as u8;
        r
    }

    /// P39: a minimal ISO-9660 image (2048-byte sectors) holding `files` ("DIR/SUB/NAME" -> data);
    /// every directory takes one sector.
    pub(crate) fn build_iso(files: &[(&str, &[u8])]) -> Vec<u8> {
        // directory tree: path -> (subdirs, files)
        let mut dirs: Vec<String> = vec![String::new()];
        for (p, _) in files {
            let parts: Vec<&str> = p.split('/').collect();
            for i in 1..parts.len() {
                let d = parts[..i].join("/");
                if !dirs.contains(&d) {
                    dirs.push(d);
                }
            }
        }
        let dir_lba = |d: &str| 18 + dirs.iter().position(|x| x == d).unwrap() as u32;
        let mut next = 18 + dirs.len() as u32;
        let mut file_lba = Vec::new();
        for (_, data) in files {
            file_lba.push(next);
            next += (data.len() as u32).div_ceil(2048).max(1);
        }
        let mut img = vec![0u8; next as usize * 2048];
        let pvd = 16 * 2048;
        img[pvd..pvd + 6].copy_from_slice(b"\x01CD001");
        img[pvd + 156..pvd + 156 + 34].copy_from_slice(&rec(&[0], 18, 2048, true));
        for d in &dirs {
            let mut s = rec(&[0], dir_lba(d), 2048, true);
            s.extend(rec(&[1], 18, 2048, true));
            let parent = |p: &str| p.rsplit_once('/').map_or(String::new(), |x| x.0.to_string());
            for c in dirs.iter().filter(|c| !c.is_empty() && parent(c) == *d) {
                s.extend(rec(c.rsplit('/').next().unwrap().as_bytes(), dir_lba(c), 2048, true));
            }
            for (i, (p, data)) in files.iter().enumerate().filter(|(_, (p, _))| parent(p) == *d) {
                let n = format!("{};1", p.rsplit('/').next().unwrap());
                s.extend(rec(n.as_bytes(), file_lba[i], data.len() as u32, false));
            }
            let o = dir_lba(d) as usize * 2048;
            img[o..o + s.len()].copy_from_slice(&s);
        }
        for (i, (_, data)) in files.iter().enumerate() {
            let o = file_lba[i] as usize * 2048;
            img[o..o + data.len()].copy_from_slice(data);
        }
        img
    }

    pub(crate) fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("p39_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn p39_iso_damaged_images() {
        let d = tmp("iso");
        let big = vec![7u8; 5000];
        let img = build_iso(&[("SYSTEM.CNF", b"BOOT2 = cdrom0:\\PBPX_955.06;1"), ("LEVEL/LOADER/A.BIN", &big)]);
        let p = d.join("ok.iso");
        std::fs::write(&p, &img).unwrap();
        let fs = IsoFs::open(&p).unwrap();
        assert_eq!(fs.read("level\\loader\\a.bin").unwrap(), big);
        assert!(matches!(fs.read("LEVEL/NOPE.BIN"), Err(EngineError::FileNotFound(_))));
        // truncated in the middle of the last file: open works, reading that file is an error
        let p = d.join("cut.iso");
        std::fs::write(&p, &img[..img.len() - 3000]).unwrap();
        let fs = IsoFs::open(&p).unwrap();
        assert!(fs.read("SYSTEM.CNF").is_ok());
        assert!(matches!(fs.read("LEVEL/LOADER/A.BIN"), Err(EngineError::Io(_))));
        // truncated before the directories / empty / garbage: errors
        for (n, b) in [("dir.iso", &img[..17 * 2048 + 100]), ("pvd.iso", &img[..16 * 2048 + 3]), ("empty.iso", &[][..])] {
            std::fs::write(d.join(n), b).unwrap();
            assert!(IsoFs::open(d.join(n)).is_err(), "{n}");
        }
        let garbage: Vec<u8> = (0..300_000u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        std::fs::write(d.join("garbage.bin"), &garbage).unwrap();
        assert!(IsoFs::open(d.join("garbage.bin")).is_err());
        // short / bogus records, a directory cycle, a 4 GB directory: no panic, no hang
        let mut bad = img.clone();
        let root = 18 * 2048;
        bad[root + 68..root + 68 + 10].copy_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        std::fs::write(d.join("short.iso"), &bad).unwrap();
        let _ = IsoFs::open(d.join("short.iso"));
        let mut cyc = img.clone();
        let lv = 19 * 2048; // LEVEL: its LOADER entry points back at the root
        let off = lv + 68;
        assert_eq!(cyc[off + 32], 6);
        cyc[off + 2..off + 6].copy_from_slice(&18u32.to_le_bytes());
        std::fs::write(d.join("cycle.iso"), &cyc).unwrap();
        assert!(IsoFs::open(d.join("cycle.iso")).is_ok());
        let mut huge = img.clone();
        huge[root + 68 + 10..root + 68 + 14].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(d.join("huge.iso"), &huge).unwrap();
        assert!(IsoFs::open(d.join("huge.iso")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// P39 R5: the disc image removed / swapped / locked while the game runs (USB drive pulled): every later
    /// read (level switch, movie, audio stream) is an error, never a panic or a hang, and the player text says why
    #[test]
    fn p39_iso_transient_read_error_retried() {
        let d = tmp("isoretry");
        let p = d.join("usb.iso");
        std::fs::write(&p, build_iso(&[("SYSTEM.CNF", b"x"), ("A.BIN", &[3u8; 5000])])).unwrap();
        let fs = IsoFs::open(&p).unwrap();
        // one transient failure: retried after READ_RETRY_MS, the read succeeds
        FAIL_READS.with(|n| n.set(1));
        let t = std::time::Instant::now();
        assert_eq!(fs.read("A.BIN").unwrap(), [3u8; 5000]);
        assert!(t.elapsed().as_millis() as u64 >= READ_RETRY_MS);
        // two in a row: the error (-> disc-gone message), no third attempt, no panic
        FAIL_READS.with(|n| n.set(2));
        assert!(matches!(fs.read("A.BIN"), Err(EngineError::Io(e)) if e.contains("not ready")));
        assert_eq!(FAIL_READS.with(|n| n.get()), 0);
        // a missing file is not retried
        let t = std::time::Instant::now();
        assert!(matches!(fs.read("NOPE.BIN"), Err(EngineError::FileNotFound(_))));
        assert!((t.elapsed().as_millis() as u64) < READ_RETRY_MS);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn p39_iso_removed_mid_game() {
        let d = tmp("isogone");
        let img = build_iso(&[("SYSTEM.CNF", b"BOOT2 = cdrom0:\\PBPX_955.06;1"), ("LEVEL/EP1/L01/GEN/EP1L01.P2R", &[1u8; 9000])]);
        let p = d.join("Scooby Ñ.iso");
        std::fs::write(&p, &img).unwrap();
        let fs = IsoFs::open(&p).unwrap();
        assert!(fs.read("LEVEL/EP1/L01/GEN/EP1L01.P2R").is_ok());
        // swapped for a shorter file (another image copied over it)
        std::fs::write(&p, &img[..20 * 2048]).unwrap();
        assert!(matches!(fs.read("LEVEL/EP1/L01/GEN/EP1L01.P2R"), Err(EngineError::Io(_))));
        // gone
        std::fs::remove_file(&p).unwrap();
        let Err(EngineError::Io(e)) = fs.read("LEVEL/EP1/L01/GEN/EP1L01.P2R") else { panic!("read of a removed image") };
        let m = disc_read_message(&p, &e);
        assert!(m.contains("is gone") && m.contains("Scooby Ñ.iso") && m.contains("reconnect"), "{m}");
        // the level loader thread (EngineRuntime::spawn_level_load) answers with an error, it does not hang
        let rx = crate::engine::runtime::spawn_level_load(p.to_string_lossy().into_owned(), "EP1/L01".into(), "EP1L01.P2R".into());
        let r = rx.0.lock().unwrap().recv_timeout(std::time::Duration::from_secs(20)).expect("level loader answered");
        assert!(r.is_err());
        // still there but unreadable (a locked file / lost share): the other wording, the detail is capped
        std::fs::write(&p, b"x").unwrap();
        let m = disc_read_message(&p, &"y".repeat(5000));
        assert!(m.contains("can't be read any more") && m.len() < 1000, "{m}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn p39_iso_unicode_long_path_and_cue() {
        let d = tmp("isopath");
        let mut dir = d.join("Jürgen Ñ 测试").join("OneDrive - Contoso").join("Документы");
        while dir.as_os_str().len() < 240 {
            dir = dir.join("Scooby-Doo Mystery Mayhem Ünïcødé");
        }
        std::fs::create_dir_all(&dir).unwrap();
        let img = build_iso(&[("SYSTEM.CNF", b"x")]);
        std::fs::write(dir.join("Scooby Ñ.iso"), &img).unwrap();
        assert_eq!(IsoFs::open(dir.join("Scooby Ñ.iso")).unwrap().read("SYSTEM.CNF").unwrap(), b"x");
        std::fs::write(dir.join("Scooby Ñ.cue"), "FILE \"Scooby Ñ.iso\" BINARY\n  TRACK 01 MODE2/2048\n").unwrap();
        assert!(IsoFs::open(dir.join("Scooby Ñ.cue")).is_ok());
        std::fs::write(dir.join("none.cue"), "REM nothing\n").unwrap();
        assert!(IsoFs::open(dir.join("none.cue")).is_err());
        std::fs::write(dir.join("missing.cue"), "FILE \"gone.bin\" BINARY\n").unwrap();
        assert!(IsoFs::open(dir.join("missing.cue")).is_err());
        std::fs::write(dir.join("bin.cue"), [0xff, 0xfe, 0, 0x46]).unwrap();
        assert!(IsoFs::open(dir.join("bin.cue")).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// P39 R6: the disc picked from a > 260-char path, through a junction, and as a UNC path
    /// (`\\localhost\C$\...`, when the admin share is reachable) opens, validates and reads
    #[test]
    fn p39_iso_over_260_unc_junction() {
        let d = tmp("isounc");
        let mut dir = d.join("Program Files (x86)").join("Jürgen Ñ 测试");
        while dir.as_os_str().len() < 300 {
            dir = dir.join("Scooby-Doo Mystery Mayhem Ünïcødé 测试");
        }
        std::fs::create_dir_all(&dir).unwrap();
        let img = build_iso(&[("SYSTEM.CNF", b"BOOT2 = cdrom0:\\PBPX_955.06;1"), ("LEVEL/LOADER/A.BIN", &[5u8; 3000]), ("GLOBAL/G", b"g"), ("STREAM/S", b"s"), ("MOVIES/M", b"m")]);
        let iso = dir.join("Scooby Doo Mystery Mayhem (Europe) [Ñ].iso");
        std::fs::write(&iso, &img).unwrap();
        assert!(iso.as_os_str().len() > 300);
        let mut ok = vec![iso.clone()];
        #[cfg(windows)]
        {
            let j = d.join("junction Ñ");
            // CREATE_NO_WINDOW: never a console flash on the user's desktop
            let st = std::os::windows::process::CommandExt::creation_flags(std::process::Command::new("cmd").args(["/d", "/c", "mklink", "/J"]).arg(&j).arg(d.join("Program Files (x86)")), 0x0800_0000).output();
            if st.is_ok_and(|o| o.status.success()) {
                ok.push(j.join(iso.strip_prefix(d.join("Program Files (x86)")).unwrap()));
            }
            let s = iso.to_string_lossy().into_owned();
            if let Some(rest) = s.strip_prefix("C:\\").or_else(|| s.strip_prefix("c:\\")) {
                let unc = std::path::PathBuf::from(format!("\\\\localhost\\C$\\{rest}"));
                if unc.exists() {
                    ok.push(unc);
                } else {
                    eprintln!("p39: \\\\localhost\\C$ not reachable, UNC case skipped");
                }
            }
        }
        for p in &ok {
            let fs = IsoFs::open(p).unwrap_or_else(|e| panic!("{}: {e:?}", p.display()));
            assert_eq!(fs.read("LEVEL/LOADER/A.BIN").unwrap(), [5u8; 3000], "{}", p.display());
            assert!(crate::engine::settings::validate_iso(p).is_ok(), "{}", p.display());
        }
        eprintln!("p39: opened {} forms of the path", ok.len());
        let _ = std::fs::remove_dir_all(&d);
    }
}
