//! WP-F: APIhud_* of EFApi_HUD.cpp as plain EFHudMgr methods (same names/arguments as the
//! `Host::APIhud_*` entries of api_gen.rs, minus `mem`; pointer arguments are passed resolved:
//! strings as `&str`, out-pointers as return values).
#![allow(non_snake_case)]

use super::{EFHudMgr, EFHudWindowKind};

impl EFHudMgr {
    /// WindowAdd(type, parent, flags, priority) (type & 0xff)
    pub fn APIhud_WindowAdd(&mut self, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        self.WindowAdd(a0 as u8, a1, a2, a3)
    }
    pub fn APIhud_WindowDelete(&mut self, a0: u32) -> u32 {
        self.WindowDelete(a0);
        0
    }
    pub fn APIhud_WindowSetFlags(&mut self, a0: u32, a1: u32) -> u32 {
        if std::env::var_os("GAME_TEXTLOG").is_some() {
            eprintln!("[hud] SetFlags win {a0:#x} {a1:#x}");
        }
        if let Some(w) = self.WindowMut(a0) {
            w.flags |= a1;
        }
        0
    }
    pub fn APIhud_WindowResetFlags(&mut self, a0: u32, a1: u32) -> u32 {
        if std::env::var_os("GAME_TEXTLOG").is_some() {
            eprintln!("[hud] ResetFlags win {a0:#x} {a1:#x}");
        }
        if let Some(w) = self.WindowMut(a0) {
            w.flags &= !a1;
        }
        0
    }
    pub fn APIhud_WindowGetFlags(&mut self, a0: u32) -> u32 {
        self.Window(a0).map(|w| w.flags).unwrap_or(0)
    }
    /// sets the timer (+0x84) and flag 0x80000000
    pub fn APIhud_WindowSetTimer(&mut self, a0: u32, a1: f32) -> u32 {
        if let Some(w) = self.WindowMut(a0) {
            w.timer = a1;
            w.flags |= 0x8000_0000;
        }
        0
    }
    pub fn APIhud_WindowSetPos(&mut self, a0: u32, a1: f32, a2: f32) -> u32 {
        if let Some(w) = self.WindowMut(a0) {
            w.pos = [a1, a2];
        }
        0
    }
    pub fn APIhud_WindowSetDims(&mut self, a0: u32, a1: f32, a2: f32) -> u32 {
        if let Some(w) = self.WindowMut(a0) {
            w.dims = [a1, a2];
        }
        0
    }
    /// empty in the original
    pub fn APIhud_WindowSetGrid(&mut self, _a0: u32, _a1: u32, _a2: u32, _a3: u32, _a4: u32, _a5: u32, _a6: u32) -> u32 {
        0
    }
    pub fn APIhud_WindowSetBackGround(&mut self, a0: u32, a1: u32, a2: f32, a3: f32) -> u32 {
        if let Some(w) = self.WindowMut(a0) {
            w.bg_color = a1;
            w.bg_border = [a2, a3];
        }
        0
    }
    /// empty in the original (stores 0 into an unrelated global)
    pub fn APIhud_WindowSetName(&mut self, _a0: u32, _a1: &str) -> u32 {
        0
    }
    /// quirk: the original returns whatever FindHudWindow left in v0 (it is void); here the handle.
    pub fn APIhud_WindowFind(&mut self, a0: u32) -> u32 {
        self.FindHudWindow(a0)
    }
    pub fn APIhud_WindowSetID(&mut self, a0: u32, a1: u32) -> u32 {
        match self.WindowMut(a0) {
            Some(w) => {
                w.id = a1;
                1
            }
            None => 0,
        }
    }
    /// quirk: the original returns the raw sibling *pointer*, or the argument when there is none
    /// (0 when the window does not exist). Here: the sibling's handle / the argument / 0.
    pub fn APIhud_WindowGetNextSibling(&mut self, a0: u32) -> u32 {
        self.link(a0, |w| w.next)
    }
    pub fn APIhud_WindowGetFirstChild(&mut self, a0: u32) -> u32 {
        self.link(a0, |w| w.child)
    }
    pub fn APIhud_WindowGetParent(&mut self, a0: u32) -> u32 {
        self.link(a0, |w| w.parent)
    }
    fn link(&self, a0: u32, f: impl Fn(&super::EFHudWindow) -> Option<u16>) -> u32 {
        match self.Window(a0) {
            None => 0,
            Some(w) => match f(w) {
                Some(s) => self.slots[s as usize].1.as_ref().map(|w| w.handle).unwrap_or(a0),
                None => a0,
            },
        }
    }
    /// Returns (x, y) (the C++ writes through the two pointers only when the window exists).
    pub fn APIhud_WindowGetPos(&mut self, a0: u32) -> Option<(f32, f32)> {
        self.Window(a0).map(|w| (w.pos[0], w.pos[1]))
    }
    pub fn APIhud_WindowGetDims(&mut self, a0: u32) -> Option<(f32, f32)> {
        self.Window(a0).map(|w| (w.dims[0], w.dims[1]))
    }
    /// EFHudMgr::Save is empty in the original.
    pub fn APIhud_WindowSaveAll(&mut self, _a0: &str) -> u32 {
        0
    }
    /// EFHud2DWindow_BC::SetTexture(EFTextureDict::GetTexture(name)).
    pub fn APIhud_Window2DSetTexture(&mut self, a0: u32, a1: &str) -> u32 {
        if let Some(d) = self.WindowMut(a0).and_then(|w| w.win2d()) {
            d.texture = Some(a1.to_string());
        }
        0
    }
    /// quirk: no type check in the original (+0x8c/+0x90 of any window); only 2D windows here.
    pub fn APIhud_Window2DSetTiling(&mut self, a0: u32, a1: f32, a2: f32) -> u32 {
        if let Some(d) = self.WindowMut(a0).and_then(|w| w.win2d()) {
            d.tiling = [a1, a2];
        }
        0
    }
    pub fn APIhud_Window2DSetColor(&mut self, a0: u32, a1: u32) -> u32 {
        if let Some(d) = self.WindowMut(a0).and_then(|w| w.win2d()) {
            d.color = a1;
        }
        0
    }
    /// EFWorld::GetGameObject(a1) -> obj+0x174[a2]; the caller resolves the object value.
    pub fn APIhud_Window3DSetObj(&mut self, a0: u32, owner: u32, obj_value: u32) -> u32 {
        if let Some(w) = self.WindowMut(a0) {
            if let EFHudWindowKind::Win3D(d) = &mut w.kind {
                d.obj = obj_value;
                d.owner = owner;
            }
        }
        0
    }
    /// SetTextToDisplay(text id, varargs): `args` = the vararg words (up to 20).
    pub fn APIhud_TextWindowSetText(&mut self, a0: u32, a1: u32, args: &[u32]) -> u32 {
        if std::env::var_os("GAME_TEXTLOG").is_some() {
            let txt: String = self.text.as_ref().and_then(|x| x.GetText(a1)).map(|b| b.iter().take(160).filter(|&&c| (0x20..0x7f).contains(&c)).map(|&c| c as char).collect()).unwrap_or_default();
            eprintln!("[hud] SetText win {a0:#x} text {a1:#x} {txt:?}");
        }
        if let Some(t) = self.WindowMut(a0).and_then(|w| w.text()) {
            t.text_id = a1;
            t.args = [0; 20];
            for (d, s) in t.args.iter_mut().zip(args) {
                *d = *s;
            }
            t.string_cmds = None;
        }
        0
    }
    pub fn APIhud_TextWindowSetMargins(&mut self, a0: u32, a1: f32, a2: f32, a3: f32, a4: f32) -> u32 {
        if let Some(t) = self.WindowMut(a0).and_then(|w| w.text()) {
            t.margins = [a1, a2, a3, a4];
        }
        0
    }
    /// SetStringToDisplay(fmt, va): the caller passes the vsnprintf result (max 0x3ff chars).
    pub fn APIhud_TextWindowSetStringToDisplay(&mut self, a0: u32, formatted: &str) -> u32 {
        let fonts = &self.fonts;
        let Some(s) = self.Resolve(a0) else { return 0 };
        let font = match &self.slots[s as usize].1.as_ref().unwrap().kind {
            EFHudWindowKind::Text(t) => t.font,
            _ => return 0,
        };
        let ascii = fonts.get(font as usize).and_then(|f| f.as_ref()).map(|f| f.ascii.clone()).unwrap_or_else(|| vec![0; 128]);
        let b = formatted.as_bytes();
        let b = &b[..b.len().min(0x3ff)];
        let cmds = if b.is_empty() { None } else { Some(ConvertStringToTextCommands(b, &ascii)) };
        if let Some(t) = self.slots[s as usize].1.as_mut().unwrap().text() {
            t.text_id = 0xffff_ffff;
            t.string_cmds = cmds;
        }
        0
    }
    /// SetTextToDisplay(char *string, va): string kept unformatted; args stored for %(color).
    pub fn APIhud_TextWindowSetString(&mut self, a0: u32, a1: &str, args: &[u32]) -> u32 {
        let Some(s) = self.Resolve(a0) else { return 0 };
        let font = match &self.slots[s as usize].1.as_ref().unwrap().kind {
            EFHudWindowKind::Text(t) => t.font,
            _ => return 0,
        };
        let ascii = self.fonts.get(font as usize).and_then(|f| f.as_ref()).map(|f| f.ascii.clone()).unwrap_or_else(|| vec![0; 128]);
        let t = self.slots[s as usize].1.as_mut().unwrap().text().unwrap();
        t.text_id = 0xffff_ffff;
        t.string_cmds = None;
        if !a1.is_empty() {
            t.args = [0; 20];
            for (d, s) in t.args.iter_mut().zip(args) {
                *d = *s;
            }
            t.string_cmds = Some(ConvertStringToTextCommands(a1.as_bytes(), &ascii));
        }
        0
    }
    pub fn APIhud_TextWindowSetColor(&mut self, a0: u32, a1: u32) -> u32 {
        if let Some(t) = self.WindowMut(a0).and_then(|w| w.text()) {
            t.color = a1;
        }
        0
    }
    pub fn APIhud_TextWindowSetFont(&mut self, a0: u32, a1: u32) -> u32 {
        if let Some(t) = self.WindowMut(a0).and_then(|w| w.text()) {
            t.font = a1;
        }
        0
    }
    pub fn APIhud_TextSetCurrentLanguage(&mut self, a0: u32) -> u32 {
        self.language = a0;
        0
    }
    pub fn APIhud_TextGetCurrentLanguage(&mut self) -> u32 {
        self.language
    }
    pub fn APIhud_ColorCopy(&mut self, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        self.ColorCopy(a0, a1, a2, a3);
        0
    }
    pub fn APIhud_ColorBlend(&mut self, a0: u32, a1: u32, a2: u32, a3: u32, a4: f32, a5: u32) -> u32 {
        self.ColorBlend(a0, a1, a2, a3, a4, a5);
        0
    }
    pub fn APIhud_ColorFadeStart(&mut self, a0: u32, a1: u32, a2: u32, a3: u32, a4: f32, a5: u32) -> u32 {
        self.ColorFadeStart(a0, a1, a2, a3, a4, a5);
        0
    }
    pub fn APIhud_ColorFadeStop(&mut self, a0: u32, a1: u32) -> u32 {
        self.ColorFadeStop(a0, a1);
        0
    }
    pub fn APIhud_ColorIsFading(&mut self, a0: u32, a1: u32) -> u8 {
        self.ColorIsFading(a0, a1) as u8
    }
}

/// EFHudTextWindow_BC::ConvertStringToTextCommands: char string -> text commands
/// (u32 0, u16 codes, u16 0, align 4, u32 1). ' ' 0xfffe, '\n' 0xffff, '\t' 0xfffd,
/// "%(color)" -> end string + command 4 (colour from the next arg) + new string, other chars via
/// the font ASCII table (bytes >= 0x80 -> ascii[0x21]).
pub fn ConvertStringToTextCommands(s: &[u8], ascii: &[u16]) -> Vec<u8> {
    let mut o: Vec<u8> = vec![0, 0, 0, 0];
    let mut i = 0;
    let pad = |o: &mut Vec<u8>| {
        // terminator then (ptr + 4) & ~3 alignment of the C++
        let term = o.len();
        o.extend_from_slice(&[0, 0]);
        let next = (term + 4) & !3;
        o.resize(next, 0);
    };
    while i < s.len() {
        let c = s[i];
        let code: u16 = match c {
            b' ' => 0xfffe,
            b'\n' => 0xffff,
            b'\t' => 0xfffd,
            b'%' if s[i + 1..].starts_with(b"(color)") => {
                pad(&mut o);
                o.extend_from_slice(&4u32.to_le_bytes());
                o.extend_from_slice(&0u32.to_le_bytes());
                i += 8;
                continue;
            }
            c if c >= 0x80 => ascii.get(0x21).copied().unwrap_or(0),
            c => ascii.get(c as usize).copied().unwrap_or(0),
        };
        o.extend_from_slice(&code.to_le_bytes());
        i += 1;
    }
    pad(&mut o);
    o.extend_from_slice(&1u32.to_le_bytes());
    o
}
