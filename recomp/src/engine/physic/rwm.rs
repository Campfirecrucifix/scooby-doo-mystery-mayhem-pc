//! WP-E2: native replacement of the RenderWare frame/matrix calls used by EFPhysic* / EFDM*
//! (RwFrameGetLTM, RwFrameRotate, RwFrameTranslate, RwV3dTransformPoints/Vectors, RwMatrixInvert,
//! RwMatrixOrthoNormalize, RwV3dNormalize/Length). NOT 1:1 (RenderWare internals): plain f32 math
//! on an `RwMatrix` with the RW layout {right +0, up +0x10, at +0x20, pos +0x30}, row vectors
//! (`p' = p.x*right + p.y*up + p.z*at + pos`).
//!
//! Frames: the physics only ever touches the root frame of an instance, so the frame's modelling
//! matrix (frame+0x10) and its LTM are the same matrix here (`EFPhysic::frame`).
#![allow(non_snake_case)]

pub type V3 = [f32; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RwMatrix {
    pub right: V3,
    pub up: V3,
    pub at: V3,
    pub pos: V3,
}

impl Default for RwMatrix {
    fn default() -> Self {
        RwMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], at: [0.0, 0.0, 1.0], pos: [0.0; 3] }
    }
}

/// rwCOMBINEREPLACE / PRECONCAT / POSTCONCAT
pub const rwCOMBINEREPLACE: i32 = 0;
pub const rwCOMBINEPRECONCAT: i32 = 1;
pub const rwCOMBINEPOSTCONCAT: i32 = 2;

pub fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
pub fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
pub fn scale(a: V3, s: f32) -> V3 { [a[0] * s, a[1] * s, a[2] * s] }
pub fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn RwV3dLength(a: V3) -> f32 { dot(a, a).sqrt() }
/// RwV3dNormalize: returns the length; a zero vector stays zero (RW returns 0 and leaves it).
pub fn RwV3dNormalize(a: V3) -> V3 {
    let l = RwV3dLength(a);
    if l > 0.0 { scale(a, 1.0 / l) } else { a }
}

impl RwMatrix {
    pub fn identity() -> Self { Self::default() }
    pub fn from_pos(pos: V3) -> Self { RwMatrix { pos, ..Default::default() } }
    /// RwV3dTransformPoints (1 point)
    pub fn TransformPoint(&self, p: V3) -> V3 { add(self.TransformVector(p), self.pos) }
    /// RwV3dTransformVectors (1 vector)
    pub fn TransformVector(&self, v: V3) -> V3 {
        add(add(scale(self.right, v[0]), scale(self.up, v[1])), scale(self.at, v[2]))
    }
    /// a * b (row vectors: apply a then b)
    pub fn Multiply(a: &RwMatrix, b: &RwMatrix) -> RwMatrix {
        RwMatrix {
            right: b.TransformVector(a.right),
            up: b.TransformVector(a.up),
            at: b.TransformVector(a.at),
            pos: b.TransformPoint(a.pos),
        }
    }
    /// rotation matrix about `axis` (normalised here as RwMatrixRotate does) by `deg` degrees
    pub fn Rotation(axis: V3, deg: f32) -> RwMatrix {
        let a = RwV3dNormalize(axis);
        let (s, c) = (deg.to_radians()).sin_cos();
        let t = 1.0 - c;
        let (x, y, z) = (a[0], a[1], a[2]);
        RwMatrix {
            right: [t * x * x + c, t * x * y + s * z, t * x * z - s * y],
            up: [t * x * y - s * z, t * y * y + c, t * y * z + s * x],
            at: [t * x * z + s * y, t * y * z - s * x, t * z * z + c],
            pos: [0.0; 3],
        }
    }
    /// RwFrameRotate(frame, axis, angle, combine)
    pub fn Rotate(&mut self, deg: f32, axis: V3, combine: i32) {
        let r = RwMatrix::Rotation(axis, deg);
        *self = match combine {
            rwCOMBINEREPLACE => r,
            rwCOMBINEPRECONCAT => RwMatrix::Multiply(&r, self),
            _ => RwMatrix::Multiply(self, &r),
        };
    }
    /// RwFrameTranslate(frame, v, combine): REPLACE sets a pure translation, PRECONCAT moves
    /// along the local axes, POSTCONCAT in world space.
    pub fn Translate(&mut self, v: V3, combine: i32) {
        match combine {
            rwCOMBINEREPLACE => *self = RwMatrix::from_pos(v),
            rwCOMBINEPRECONCAT => self.pos = add(self.pos, self.TransformVector(v)),
            _ => self.pos = add(self.pos, v),
        }
    }
    /// RwMatrixOrthoNormalize
    pub fn OrthoNormalize(&self) -> RwMatrix {
        let at = RwV3dNormalize(self.at);
        let right = RwV3dNormalize(cross(self.up, at));
        let up = cross(at, right);
        RwMatrix { right, up, at, pos: self.pos }
    }
    /// RwMatrixInvert (general 3x3 inverse + translation)
    pub fn Invert(&self) -> RwMatrix {
        let (r, u, a) = (self.right, self.up, self.at);
        let c0 = cross(u, a);
        let det = dot(r, c0);
        let id = if det != 0.0 { 1.0 / det } else { 0.0 };
        let c1 = cross(a, r);
        let c2 = cross(r, u);
        // columns of the inverse are c0,c1,c2 / det
        let m = RwMatrix {
            right: [c0[0] * id, c1[0] * id, c2[0] * id],
            up: [c0[1] * id, c1[1] * id, c2[1] * id],
            at: [c0[2] * id, c1[2] * id, c2[2] * id],
            pos: [0.0; 3],
        };
        let p = m.TransformVector(self.pos);
        RwMatrix { pos: scale(p, -1.0), ..m }
    }
}

/// DT_GetBits(x) & 0x7f800000 < 0x34000000 test used by EFPhysicMgr::CollisionStage
/// (|x| < 2^-23 counts as zero).
pub fn DTIsTiny(x: f32) -> bool { (x.to_bits() & 0x7f80_0000) < 0x3400_0000 }

/// DT_EPSILON_FLOAT / DT_MIN_FLOAT (EXE.1 globals; FLT_EPSILON / FLT_MIN assumed)
pub const DT_EPSILON_FLOAT: f32 = f32::EPSILON;
pub const DT_MIN_FLOAT: f32 = f32::MIN_POSITIVE;
