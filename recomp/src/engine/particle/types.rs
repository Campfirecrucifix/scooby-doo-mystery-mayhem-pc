//! PLType_BC / PLType (class 0x2d "PL_TYPE", 0x1bc bytes): the particle type ("how particles of
//! an emitter look and move"). Classes/PL/PLType_BC.cpp, PLType.cpp.

#![allow(non_snake_case)]

use super::*;

/// The PLType_BC data (everything except the emitter list +0x18c, which is `PLType::emitters`).
#[derive(Debug, Clone, PartialEq)]
pub struct PLTypeData {
    /// +0x8 id (0x2d000000 | index for loaded types, 0x2d800001.. for created ones)
    pub id: u32,
    /// +0x10 RpMaterial* (PLType::SetTexture creates it; Render3dParticles reads its texture)
    pub has_material: bool,
    /// +0x14 rotation curve
    pub rotation: PLInterpol,
    /// +0x34 scale curve (all values stored / 1.25)
    pub scale: PLInterpol,
    /// +0x54 / +0x74 / +0x94 / +0xb4 R, G, B, A curves
    pub color: [PLInterpol; 4],
    /// +0xd4 char[0x20] display name
    pub name: String,
    /// +0x154 flags: 1 = 3D oriented particles (own matrix), 8 = frozen (no update),
    /// 0x10 = freeze when the world is paused, 0x80/0x200 blend modes, 0x100 stencil read
    pub flags: u32,
    /// +0x158 bounding-sphere radius (ComputeBSphereRadius)
    pub bsphere_radius: f32,
    /// +0x15c gravity (acceleration)
    pub gravity: V3,
    /// +0x168 orientation (stored only)
    pub orientation: V3,
    /// +0x174 lifespan (s)
    pub lifespan: f32,
    /// +0x178 launch cone angle in degrees (int)
    pub angle: u32,
    /// +0x17c launch period for continuous emitters (s)
    pub rate: f32,
    /// +0x180 friction
    pub friction: f32,
    /// +0x184 launch speed
    pub speed: f32,
    /// +0x188 particles per launch
    pub count: u16,
    /// +0x198 / +0x19c / +0x1a0 statistics of the last Update (active particles, unused, launches)
    pub nb_active: i32,
    pub nb_19c: i32,
    pub nb_launched: i32,
    /// +0x1a4..+0x1b4 user ints (SetInt masks 0x10..0x100)
    pub user: [u32; 5],
    /// +0x1b8 RwTexture* (PLType) -> texture name
    pub texture: Option<String>,
}

/// PLType: the data plus the emitters using it (PLLinkList<PLEmitter> +0x18c; index 0 = head).
#[derive(Debug, Clone, PartialEq)]
pub struct PLType {
    pub d: PLTypeData,
    pub emitters: Vec<PLEmitter>,
}

impl Default for PLType {
    fn default() -> Self {
        Self::PLType()
    }
}

impl PLType {
    pub const CLASS_ID: u32 = 0x2d;
    pub const CLASS_NAME: &'static str = "PL_TYPE";
    pub const SIZE: usize = 0x1bc;

    /// PLType_BC::PLType_BC (0x0020f350) + PLType::PLType (0x00238c60)
    #[allow(clippy::new_ret_no_self)]
    pub fn PLType() -> Self {
        let mut d = PLTypeData {
            id: 0,
            has_material: false,
            rotation: PLInterpol::default(),
            scale: PLInterpol::default(),
            color: [PLInterpol::default(); 4],
            name: String::new(),
            flags: 0x10,
            bsphere_radius: 0.0,
            gravity: [0.0; 3],
            orientation: [0.0; 3],
            lifespan: 1.5,
            angle: 0xa0,
            rate: 0.1,
            friction: 0.0,
            speed: 0.0,
            count: 6,
            nb_active: 0,
            nb_19c: 0,
            nb_launched: 0,
            user: [0; 5],
            texture: None,
        };
        d.SetScale(0.01, 0.02, 0.1, 1.0);
        d.SetRotation(0.0, 20.0, 50.0, 1.0);
        d.SetColor(0xf, 1.0, 1.0, 1.0, 1.0);
        d.name = "No name yet".into();
        PLType { d, emitters: Vec::new() }
    }
    pub fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    pub fn GetClassName(&self) -> &'static str {
        Self::CLASS_NAME
    }
}

impl PLTypeData {
    /// PLType_BC::ComputeBSphereRadius (0x0020f7e0) (vtbl 0x50, PLType forwards to it)
    pub fn ComputeBSphereRadius(&mut self) {
        let (a, b, c) = (self.scale.v0, self.scale.v1, self.scale.v2);
        let m = if b < a { a } else { b };
        let m = if c <= m { m } else { c };
        let g = (self.gravity[0] * self.gravity[0] + self.gravity[1] * self.gravity[1] + self.gravity[2] * self.gravity[2]).sqrt();
        self.bsphere_radius = m + self.lifespan * (self.speed + self.lifespan * g * 0.5) + 0.1;
    }
    /// PLType_BC::SetInter (0x0020f890)
    pub fn SetInter(&mut self, mask: u32, a: f32, b: f32, c: f32, d: f32) -> u32 {
        for bit in [1, 2, 4, 8] {
            if mask & bit != 0 {
                self.SetColor(bit, a, b, c, d);
            }
        }
        if mask & 0x10 != 0 {
            self.SetScale(a, b, c, d);
        }
        if mask & 0x20 != 0 {
            self.SetRotation(a, b, c, d);
        }
        0
    }
    /// PLType_BC::SetInt (0x0020f9c0)
    pub fn SetInt(&mut self, mask: u32, v: u32) -> u32 {
        if mask & 1 != 0 {
            self.flags = v;
        }
        if mask & 4 != 0 {
            self.angle = v;
        }
        if mask & 8 != 0 {
            self.count = v as u16;
        }
        for (i, bit) in [0x10, 0x20, 0x40, 0x80, 0x100].into_iter().enumerate() {
            if mask & bit != 0 {
                self.user[i] = v;
            }
        }
        0
    }
    /// PLType_BC::GetInt (0x0020fa40): exact selector match; 4 = unknown selector.
    pub fn GetInt(&self, sel: u32, out: &mut u32) -> u32 {
        *out = match sel {
            0x100 => self.user[4],
            0x80 => self.user[3],
            0x40 => self.user[2],
            0x20 => self.user[1],
            0x10 => self.user[0],
            8 => self.count as u32,
            4 => self.angle,
            1 => self.flags,
            _ => return 4,
        };
        0
    }
    /// PLType_BC::SetReal (0x0020fb10)
    pub fn SetReal(&mut self, sel: u32, v: f32) -> u32 {
        if sel == 1 {
            self.friction = v;
        }
        if sel == 4 {
            self.rate = v;
        }
        if sel == 2 {
            self.SetLifespan(v);
        }
        if sel == 8 {
            self.speed = v;
        }
        0
    }
    /// PLType_BC::SetVec (0x0020fb90)
    pub fn SetVec(&mut self, sel: u32, x: f32, y: f32, z: f32) -> u32 {
        if sel == 1 {
            self.SetGravity(x, y, z);
        }
        if sel == 2 {
            self.SetOrientation(x, y, z);
        }
        0
    }
    /// PLType_BC::SetLifespan (0x0020fc10): every curve's t2 follows the lifespan (scale: /1.25).
    pub fn SetLifespan(&mut self, v: f32) {
        self.lifespan = v;
        self.rotation.t2 = v;
        self.rotation.UpdateDelta();
        self.scale.t2 = self.lifespan / 1.25;
        self.scale.UpdateDelta();
        for c in self.color.iter_mut() {
            c.t2 = v;
            c.UpdateDelta();
        }
        self.ComputeBSphereRadius();
    }
    /// PLType_BC::SetGravity (0x0020fd50)
    pub fn SetGravity(&mut self, x: f32, y: f32, z: f32) {
        self.gravity = [x, y, z];
        self.ComputeBSphereRadius();
    }
    /// PLType_BC::SetOrientation (0x0020fd80)
    pub fn SetOrientation(&mut self, x: f32, y: f32, z: f32) {
        self.orientation = [x, y, z];
    }
    /// PLType_BC::SetRotation (0x0020fd90)
    pub fn SetRotation(&mut self, a: f32, b: f32, c: f32, t1: f32) {
        let l = self.lifespan;
        self.rotation.set3(a, b, c, t1, l);
    }
    /// PLType_BC::SetScale (0x0020fe00): all values (incl. t1 and t2) divided by 1.25.
    pub fn SetScale(&mut self, a: f32, b: f32, c: f32, t1: f32) {
        let l = self.lifespan / 1.25;
        self.scale.set3(a / 1.25, b / 1.25, c / 1.25, t1 / 1.25, l);
        self.ComputeBSphereRadius();
    }
    /// PLType_BC::SetColor (0x0020fff0): mask 1 R, 2 G, 4 B, 8 A.
    pub fn SetColor(&mut self, mask: u32, a: f32, b: f32, c: f32, t1: f32) {
        let l = self.lifespan;
        for (i, bit) in [1u32, 2, 4, 8].into_iter().enumerate() {
            if mask & bit != 0 {
                self.color[i].set3(a, b, c, t1, l);
            }
        }
    }
    /// PLType_BC::SetDisplayName (0x00210360)
    pub fn SetDisplayName(&mut self, s: &str) {
        self.name = s.to_string();
    }
    /// PLType::SetTexture (0x00238d50) (vtbl 0x4c): RwTexDictionaryFindNamedTexture in the current
    /// dictionary (`dict` = its texture names, None = no current dictionary); 4 = not found.
    /// Replacement: name lookup is case-insensitive (RW rwstrcmp on PS2 builds is ASCII case
    /// sensitive; the shipped data uses matching case).
    pub fn SetTexture(&mut self, name: &str, dict: Option<&[String]>) -> u32 {
        let Some(dict) = dict else { return 4 };
        match dict.iter().find(|t| t.eq_ignore_ascii_case(name)) {
            None => {
                self.texture = None;
                4
            }
            Some(t) => {
                self.texture = Some(t.clone());
                self.has_material = true;
                0
            }
        }
    }
    /// PLType::GetTexture (0x00238de0): "N\A" without texture.
    pub fn GetTexture(&self) -> String {
        self.texture.clone().unwrap_or_else(|| "N\\A".into())
    }

    /// PLType_BC::Initialize (0x00210380) from a 0xd8-byte record of DTParticleContainerDescriptor
    /// PARTTYPES: +0 name[0x20], +0x20 texture[0x20], +0x40 friction, +0x44 lifespan, +0x48 rate,
    /// +0x4c speed, +0x50 angle, +0x54 u16 count, +0x58 gravity, +0x64 R{v0,v1,v2,t1}, +0x74 G,
    /// +0x84 B, +0x94 A, +0xa4 scale (all /1.25), +0xb4 rotation, +0xc4..+0xd0 user ints 0..3,
    /// +0xd4 flags. -1 = bad size or texture not found (quirk: the type stays half-initialised
    /// and in the manager's list).
    pub fn Initialize(&mut self, rec: &[u8], size: u32, dict: Option<&[String]>) -> i32 {
        if size != 0xd8 || rec.len() < 0xd8 {
            return -1;
        }
        let u = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        let f = |o: usize| f32::from_bits(u(o));
        let s = |o: usize| {
            let b = &rec[o..o + 0x20];
            let n = b.iter().position(|&c| c == 0).unwrap_or(0x20);
            String::from_utf8_lossy(&b[..n]).into_owned()
        };
        self.name = s(0);
        if self.SetTexture(&s(0x20), dict) == 4 {
            return -1;
        }
        self.friction = f(0x40);
        self.lifespan = f(0x44);
        self.rate = f(0x48);
        self.speed = f(0x4c);
        self.angle = u(0x50);
        self.count = u16::from_le_bytes([rec[0x54], rec[0x55]]);
        self.gravity = [f(0x58), f(0x5c), f(0x60)];
        let l = self.lifespan;
        for i in 0..4 {
            let o = 0x64 + i * 0x10;
            self.color[i].set3(f(o), f(o + 4), f(o + 8), f(o + 0xc), l);
        }
        self.scale.set3(f(0xa4) / 1.25, f(0xa8) / 1.25, f(0xac) / 1.25, f(0xb0) / 1.25, l / 1.25);
        self.rotation.set3(f(0xb4), f(0xb8), f(0xbc), f(0xc0), l);
        // quirk: only user ints 0..3 are loaded (+0x1b4 keeps its value); no ComputeBSphereRadius
        for i in 0..4 {
            self.user[i] = u(0xc4 + i * 4);
        }
        self.flags = u(0xd4);
        0
    }

    /// PLType_BC::LoadTypeFile (0x0020f5a0): editor-only type file `u32 size; u32 count; count x
    /// {u16 kind, u16 mask, payload}` with record stride PL_DATA_SIZE[kind]. The PL_DATA_SIZE table
    /// is not in the decomp: assumed 8/8/16/20 (header + payload) for kinds 0 int, 1 real, 2 vec,
    /// 3 inter. (quirk: the original freads the data over its own buffer pointer; this reads the
    /// intended buffer.) Unknown kind stops the parse.
    pub fn LoadTypeFile(&mut self, file: &[u8]) {
        const PL_DATA_SIZE: [usize; 4] = [8, 8, 16, 20];
        if file.len() < 8 {
            return;
        }
        let b = &file[4..];
        let u = |o: usize| b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).unwrap_or(0);
        let f = |o: usize| f32::from_bits(u(o));
        let n = u(0);
        let mut p = 4;
        for _ in 0..n {
            if p + 4 > b.len() {
                return;
            }
            let kind = u16::from_le_bytes([b[p], b[p + 1]]);
            let mask = u16::from_le_bytes([b[p + 2], b[p + 3]]) as u32;
            match kind {
                3 => {
                    self.SetInter(mask, f(p + 4), f(p + 8), f(p + 12), f(p + 16));
                }
                2 => {
                    self.SetVec(mask, f(p + 4), f(p + 8), f(p + 12));
                }
                1 => {
                    self.SetReal(mask, f(p + 4));
                }
                0 => {
                    self.SetInt(mask, u(p + 4));
                }
                _ => return,
            }
            p += PL_DATA_SIZE[kind as usize];
        }
    }
}
