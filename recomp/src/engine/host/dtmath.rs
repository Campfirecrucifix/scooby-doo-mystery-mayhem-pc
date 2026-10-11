//! WP-S2: DTQuadratic / DTCubic / DTQuartic::Solve (Classes/DT, 0x0010d3f0..0x0010debb), 1:1 in
//! f32 like the compiled code (used by APIphysic::JumpOnCoor flags 4/8).

/// C `_dpflt(fabs((double)x), eps) == 0` (|x| not below eps)
fn not_tiny(x: f32, eps: u64) -> bool {
    !((x as f64).abs() < f64::from_bits(eps))
}

/// DTQuadratic::Solve (0x0010d3f0): a t^2 + b t + c. DTIsNeg = sign bit, DTIsNull = +0.0 bits.
pub fn quadratic_solve(a: f32, b: f32, c: f32) -> Vec<f32> {
    if !not_tiny(a, 0x3ee4_f8b5_8000_0000) {
        return vec![-c / b];
    }
    let d = b * b - a * 4.0 * c;
    if d.to_bits() & 0x8000_0000 != 0 {
        vec![]
    } else if d.to_bits() == 0 {
        vec![(b / a) * -0.5]
    } else {
        vec![((b + d.sqrt()) / a) * -0.5, ((b - d.sqrt()) / a) * -0.5]
    }
}

/// DTCubic::Solve (0x0010d590): a t^3 + b t^2 + c t + d (Cardano). quirk: the second cube root
/// of the one-root case with a non-negative argument uses pow(x, (double)(1.0f/3)) as compiled.
pub fn cubic_solve(a: f32, b: f32, c: f32, d: f32) -> Vec<f32> {
    if !not_tiny(a, 0x3ee4_f8b5_88e3_68f1) {
        return quadratic_solve(b, c, d);
    }
    let inv = 1.0 / a;
    let b1 = b * inv;
    let bb = b1 * b1;
    let c1 = c * inv;
    let q = c1 * 0.333_333_34 - bb * 0.111_111_11;
    let r = (b1 * c1 * 0.166_666_67 - d * inv * 0.5) - bb * b1 * 0.037_037_037;
    let q3 = q * q * q;
    let disc = q3 + r * r;
    if 0.0 < disc {
        let s = disc.sqrt();
        let u = r + s;
        let mut x = -b1 * 0.333_333_34;
        if u < 0.0 {
            x -= ((-u) as f64).powf(0.333_333_333_333_333_3) as f32;
        } else {
            x += (u as f64).powf(0.333_333_333_333_333_3) as f32;
        }
        let v = r - s;
        if v < 0.0 {
            x -= ((-v) as f64).powf(0.333_333_333_333_333_3) as f32;
        } else {
            x += (v as f64).powf(0.333_333_343_267_440_8) as f32;
        }
        vec![x]
    } else if disc < 0.0 {
        let th = (r / (-q3).sqrt()).acos() * 0.333_333_34;
        let o = -b1 * 0.333_333_34;
        let k = (-q).sqrt() * 2.0;
        vec![o + k * th.cos(), o + k * (th + 2.094_395_2).cos(), o + k * (th + 4.188_790_3).cos()]
    } else {
        let e = c1 * 3.0 - bb;
        if e == 0.0 {
            vec![-b1 * 0.333_333_34]
        } else {
            let d9 = d * inv * 9.0;
            vec![((b1 * c1 - d9) * 0.5) / e, (d9 + (bb * b1 - b1 * 4.0 * c1)) / e]
        }
    }
}

/// DTQuartic::Solve (0x0010da00): q[0] t^4 + q[1] t^3 + q[2] t^2 + q[3] t + q[4] (Ferrari).
/// quirk kept: in the biquadratic two-positive-pairs case the 4th root is `-sol[0] - b/4`
/// (sol[0] already shifted), as compiled.
pub fn quartic_solve(q: [f32; 5]) -> Vec<f32> {
    let mut s = [0f32; 4];
    let inv = 1.0 / q[0];
    let b = q[1] * inv;
    let bb = b * b;
    let c = q[2] * inv;
    let p3 = (q[3] * inv + bb * 0.125 * b) - b * c * 0.5;
    let o = b * 0.25;
    let p2 = c - bb * 0.375;
    let p4 = ((q[4] * inv - bb * 0.011_718_75 * bb) + bb * 0.0625 * c) - o * q[3] * inv;
    let n: usize;
    if p3 == 0.0 {
        if p2 == 0.0 {
            if 0.0 < p4 {
                n = 0;
            } else if p4 == 0.0 {
                n = 1;
                s[0] = b * -0.25;
            } else {
                n = 2;
                s[0] = (-p4).sqrt().sqrt();
                s[1] = -(-p4).sqrt().sqrt() - o;
                s[0] -= o;
            }
        } else {
            let d = p2 * p2 - p4 * 4.0;
            if d < 0.0 {
                n = 0;
            } else if d == 0.0 {
                n = 1;
                s[0] = (-p2 * 0.5).sqrt() - o;
            } else {
                let z1 = (-p2 + d.sqrt()) * 0.5;
                let mut k = 0;
                if z1 == 0.0 {
                    k = 1;
                    s[0] = b * -0.25;
                } else if 0.0 < z1 {
                    k = 2;
                    s[0] = z1.sqrt();
                    s[1] = -z1.sqrt() - o;
                    s[0] -= o;
                }
                let z2 = (-p2 - d.sqrt()) * 0.5;
                if z2 == 0.0 {
                    s[k] = b * -0.25;
                    k += 1;
                } else if 0.0 < z2 {
                    s[k] = z2.sqrt();
                    s[k + 1] = -s[0] - o;
                    s[k] -= o;
                    k += 2;
                }
                n = k;
            }
        }
    } else {
        let p4x4 = p4 * 4.0;
        let roots = cubic_solve(1.0, p2 * 2.0, p2 * p2 - p4x4, -p3 * p3);
        let mut y = 0.0f32;
        for r in &roots {
            y = *r;
            if 0.0 <= y {
                break;
            }
        }
        let sy = y.sqrt();
        let z = ((p2 + y) - p3 / sy) * 0.5;
        let e1 = y - z * 4.0;
        let mut k = 0;
        if 0.0 <= e1 {
            if e1 == 0.0 {
                k = 1;
                s[0] = sy * -0.5 - o;
            } else {
                k = 2;
                s[0] = (-sy + e1.sqrt()) * 0.5 - o;
                s[1] = (-sy - e1.sqrt()) * 0.5 - o;
            }
        }
        let e2 = y - p4x4 / z;
        if 0.0 <= e2 {
            if e2 == 0.0 {
                s[k] = sy * 0.5 - o;
                k += 1;
            } else {
                s[k] = (sy + e2.sqrt()) * 0.5 - o;
                s[k + 1] = (sy - e2.sqrt()) * 0.5 - o;
                k += 2;
            }
        }
        n = k;
    }
    s[..n].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quartic_roots() {
        // (t-1)(t-2)(t+1)(t+3) = t^4 + t^3 - 7t^2 - t + 6
        let mut r = quartic_solve([1.0, 1.0, -7.0, -1.0, 6.0]);
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let e = [-3.0, -1.0, 1.0, 2.0];
        assert_eq!(r.len(), 4);
        for (a, b) in r.iter().zip(e) {
            assert!((a - b).abs() < 1e-3, "{r:?}");
        }
        assert_eq!(cubic_solve(1.0, -6.0, 11.0, -6.0).len(), 3);
        assert_eq!(quadratic_solve(1.0, 0.0, 1.0).len(), 0);
    }
}
