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

/// Directory of a disc image: upper-case path -> (lba, size).
#[derive(Debug)]
pub struct IsoFs {
    path: PathBuf,
    files: HashMap<String, (u32, u32)>,
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
        let mut f = File::open(path.as_ref()).map_err(io)?;
        let pvd = read_at(&mut f, 16 * SECTOR, SECTOR as usize).map_err(io)?;
        if &pvd[1..6] != b"CD001" {
            return Err(EngineError::Format("not an ISO-9660 image".into()));
        }
        let root = &pvd[156..156 + 34];
        let mut files = HashMap::new();
        let mut stack = vec![(String::new(), le32(root, 2), le32(root, 10))];
        while let Some((prefix, lba, size)) = stack.pop() {
            let dir = read_at(&mut f, lba as u64 * SECTOR, size as usize).map_err(io)?;
            let mut off = 0usize;
            while off < dir.len() {
                let len = dir[off] as usize;
                if len == 0 {
                    // records never cross a sector: skip to the next one
                    off = (off / SECTOR as usize + 1) * SECTOR as usize;
                    continue;
                }
                let rec = &dir[off..(off + len).min(dir.len())];
                let nlen = rec[32] as usize;
                let name = &rec[33..33 + nlen];
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
                off += len;
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
        super::gamedata::set_iso_path(path.as_ref());
        Ok(Self { path: path.as_ref().to_path_buf(), files })
    }

    /// DTFsize: file size, or -1 when missing.
    pub fn size(&self, name: &str) -> i32 {
        self.files.get(&norm(name)).map_or(-1, |e| e.1 as i32)
    }

    /// DTFopen + DTFread(whole) + DTFclose.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, EngineError> {
        let &(lba, size) = self.files.get(&norm(name)).ok_or_else(|| EngineError::FileNotFound(name.to_string()))?;
        let mut f = File::open(&self.path).map_err(|e| EngineError::Io(e.to_string()))?;
        read_at(&mut f, lba as u64 * SECTOR, size as usize).map_err(|e| EngineError::Io(e.to_string()))
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|s| s.as_str())
    }
}

fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn read_at(f: &mut File, pos: u64, len: usize) -> std::io::Result<Vec<u8>> {
    f.seek(SeekFrom::Start(pos))?;
    let mut v = vec![0u8; len];
    f.read_exact(&mut v)?;
    Ok(v)
}
