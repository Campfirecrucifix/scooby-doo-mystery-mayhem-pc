//! Bank file path resolution, 1:1 with SOUND_BankLoad (0x0025c870) / EFTSCTable/EFSETTable loads.
//! Bank 0 (global): LEVEL\<lv>\<lang>\AUDIO\f, else GLOBAL\<ep><lang>\AUDIO\f, else
//! GLOBAL\<ep>GEN\AUDIO\f. Other banks: LEVEL\<lv>\<lang>\AUDIO\f, else LEVEL\<lv>\GEN\AUDIO\f.
//! Returned in try order with '/' separators (ISO paths). `ep` is the audio-mgr episode prefix
//! (gpoNewAudioMgr+0x32c, e.g. "EP1\\" -> pass "EP1/").
pub fn bank_paths(bank: u32, level: &str, lang: &str, ep: &str, file: &str) -> Vec<String> {
    let mut v = vec![format!("LEVEL/{level}/{lang}/AUDIO/{file}")];
    if bank == 0 {
        v.push(format!("GLOBAL/{ep}{lang}/AUDIO/{file}"));
        v.push(format!("GLOBAL/{ep}GEN/AUDIO/{file}"));
    } else {
        v.push(format!("LEVEL/{level}/GEN/AUDIO/{file}"));
    }
    v
}

/// STREAM sample names are stored with '\' (e.g. STREAM\EN\EP1\LEVEL1\HEROES\SHG881EN.VAG).
pub fn stream_path(name: &str) -> String { name.replace(char::from(0x5c), "/") }
