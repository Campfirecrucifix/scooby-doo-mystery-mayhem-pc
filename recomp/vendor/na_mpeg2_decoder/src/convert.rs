use crate::video::{Frame, PixelFormat};

/// Convert a decoded YUV frame to RGBA (BT.601 limited range).
///
/// `out_rgba` must have length `frame.width * frame.height * 4`.
/// The function does not allocate.
pub fn frame_to_rgba_bt601_limited(frame: &Frame, out_rgba: &mut [u8]) {
    debug_assert_eq!(out_rgba.len(), frame.width * frame.height * 4);

    match frame.format {
        PixelFormat::Yuv420p | PixelFormat::Yuv422p | PixelFormat::Yuv444p => {}
    }

    let (cx, cy) = frame.chroma_shifts();
    let w_uv = frame.width >> cx;

    for y in 0..frame.height {
        let y_row = &frame.data_y[y * frame.linesize_y..][..frame.width];
        let uv_y = y >> cy;
        let u_row = &frame.data_u[uv_y * frame.linesize_u..][..w_uv];
        let v_row = &frame.data_v[uv_y * frame.linesize_v..][..w_uv];

        for x in 0..frame.width {
            let yv = y_row[x] as i32;
            let uv_x = x >> cx;
            let u = u_row[uv_x] as i32;
            let v = v_row[uv_x] as i32;
            let (r, g, b) = yuv_to_rgb_bt601_limited(yv, u, v);
            let o = (y * frame.width + x) * 4;
            out_rgba[o] = r;
            out_rgba[o + 1] = g;
            out_rgba[o + 2] = b;
            out_rgba[o + 3] = 255;
        }
    }
}

/// Convert directly to a smaller RGBA image without allocating a full-size
/// intermediate frame. Intended for displays with less video memory.
pub fn frame_to_rgba_bt601_limited_scaled(
    frame: &Frame,
    out_rgba: &mut [u8],
    out_width: usize,
    out_height: usize,
) {
    debug_assert!(out_width > 0 && out_height > 0);
    debug_assert_eq!(out_rgba.len(), out_width * out_height * 4);
    let (cx, cy) = frame.chroma_shifts();
    for y in 0..out_height {
        let source_y = y * frame.height / out_height;
        let y_row = &frame.data_y[source_y * frame.linesize_y..];
        let u_row = &frame.data_u[(source_y >> cy) * frame.linesize_u..];
        let v_row = &frame.data_v[(source_y >> cy) * frame.linesize_v..];
        for x in 0..out_width {
            let source_x = x * frame.width / out_width;
            let yv = y_row[source_x] as i32;
            let u = u_row[source_x >> cx] as i32;
            let v = v_row[source_x >> cx] as i32;
            let (r, g, b) = yuv_to_rgb_bt601_limited(yv, u, v);
            let o = (y * out_width + x) * 4;
            out_rgba[o] = r;
            out_rgba[o + 1] = g;
            out_rgba[o + 2] = b;
            out_rgba[o + 3] = 255;
        }
    }
}

/// Convert a decoded frame to grayscale RGBA by duplicating the luma plane.
pub fn frame_to_gray_rgba(frame: &Frame, out_rgba: &mut [u8]) {
    debug_assert_eq!(out_rgba.len(), frame.width * frame.height * 4);
    for y in 0..frame.height {
        let src = &frame.data_y[y * frame.linesize_y..][..frame.width];
        for x in 0..frame.width {
            let v = src[x];
            let o = (y * frame.width + x) * 4;
            out_rgba[o] = v;
            out_rgba[o + 1] = v;
            out_rgba[o + 2] = v;
            out_rgba[o + 3] = 255;
        }
    }
}

#[inline]
fn yuv_to_rgb_bt601_limited(y: i32, u: i32, v: i32) -> (u8, u8, u8) {
    // BT.601 limited range (MPEG). Clamp for safety.
    let c = (y - 16).max(0);
    let d = u - 128;
    let e = v - 128;

    let r = (298 * c + 409 * e + 128) >> 8;
    let g = (298 * c - 100 * d - 208 * e + 128) >> 8;
    let b = (298 * c + 516 * d + 128) >> 8;

    (clamp_u8(r), clamp_u8(g), clamp_u8(b))
}

#[inline]
fn clamp_u8(v: i32) -> u8 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v as u8
    }
}
