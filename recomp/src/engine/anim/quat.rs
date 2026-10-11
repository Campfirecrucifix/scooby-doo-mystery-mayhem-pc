//! DTQuaternion helpers used by the animation blend (Classes/DT/DTQuaternion.cpp), 1:1.
//! Quaternions are `[x, y, z, w]` (DTQuaternion +0 x, +4 y, +8 z, +0xc w).
#![allow(non_snake_case)]

pub struct DTQuaternion;

impl DTQuaternion {
    /// DTQuaternion::DotProduct (0x0010eb90)
    pub fn DotProduct(a: &[f32; 4], b: &[f32; 4]) -> f32 {
        a[3] * b[3] + a[2] * b[2] + a[0] * b[0] + a[1] * b[1]
    }

    /// DTQuaternion::Lerp (0x0010e850): component lerp toward `b`, `b` negated when the dot is
    /// negative. Not normalised.
    pub fn Lerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
        let mut b = *b;
        if a[3] * b[3] + a[2] * b[2] + a[1] * b[1] + a[0] * b[0] < 0.0 {
            b = [-b[0], -b[1], -b[2], -b[3]];
        }
        [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), a[2] + t * (b[2] - a[2]), a[3] + t * (b[3] - a[3])]
    }

    /// DTQuaternion::InvSqrtApprox (0x0010e9e0): linear 1/sqrt approximation around 0.959066
    pub fn InvSqrtApprox(x: f32) -> f32 {
        (x - 0.959066) * -0.5325156 + 1.0214351
    }

    /// DTQuaternion::LengthSquare
    pub fn LengthSquare(q: &[f32; 4]) -> f32 {
        q[3] * q[3] + q[2] * q[2] + q[1] * q[1] + q[0] * q[0]
    }

    /// DTQuaternion::FastNormalise (0x0010e900): 1..3 refinement steps of InvSqrtApprox.
    /// quirk: approximate, the result is not exactly unit length.
    pub fn FastNormalise(q: &mut [f32; 4]) {
        let l = Self::LengthSquare(q);
        let mut s = Self::InvSqrtApprox(l);
        if l <= 0.915212 {
            s *= Self::InvSqrtApprox(l * s * s);
            if l <= 0.6521197 {
                s *= Self::InvSqrtApprox(l * s * s);
            }
        }
        q.iter_mut().for_each(|x| *x *= s);
    }

    /// DTQuaternion::CorrectFactorForLerp (0x0010eb20)
    pub fn CorrectFactorForLerp(t: f32, dot: f32) -> f32 {
        let k = 1.0 - dot * 0.8227969;
        let k = k * k * 0.5854922;
        t * (k + (t * 2.0 - 3.0) * k * t + 1.0)
    }

    /// DTQuaternion::QLerp (0x0010ea50): corrected-factor lerp + FastNormalise.
    /// quirk: the correction uses the signed dot (Lerp flips afterwards).
    pub fn QLerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
        let d = Self::DotProduct(a, b);
        let f = if t <= 0.5 { Self::CorrectFactorForLerp(t, d) } else { 1.0 - Self::CorrectFactorForLerp(1.0 - t, d) };
        let mut q = Self::Lerp(a, b, f);
        Self::FastNormalise(&mut q);
        q
    }
}
