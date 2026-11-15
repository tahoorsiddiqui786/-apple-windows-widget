//! Extracts the real Windows icon of an app/file/folder as RGBA pixels, via
//! `IShellItemImageFactory` (the same source Explorer and the Start menu use).

use core::ffi::c_void;

use windows::core::Interface;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::Shell::{IShellItem, IShellItemImageFactory, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY};

pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Returns the item's icon at (up to) `size` px, cropped to its visible
/// content and squared, so icons with different built-in padding line up.
pub fn icon_rgba(item: &IShellItem, size: i32) -> Result<Rgba, String> {
    let factory: IShellItemImageFactory = item.cast().map_err(|e| e.message().to_string())?;
    let hbm = unsafe { factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK) }
        .map_err(|e| e.message().to_string())?;
    let result = hbitmap_to_rgba(hbm);
    unsafe {
        let _ = DeleteObject(HGDIOBJ(hbm.0));
    }
    result.map(crop_to_content)
}

fn hbitmap_to_rgba(hbm: HBITMAP) -> Result<Rgba, String> {
    let mut bm = BITMAP::default();
    let got = unsafe {
        GetObjectW(HGDIOBJ(hbm.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut c_void))
    };
    if got == 0 || bm.bmWidth <= 0 || bm.bmHeight == 0 {
        return Err("icon bitmap has no size".into());
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
    let lines = unsafe {
        let hdc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            hdc,
            hbm,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        lines
    };
    if lines == 0 {
        return Err("could not read icon pixels".into());
    }

    let has_alpha = buf.chunks_exact(4).any(|p| p[3] != 0);
    // The shell hands back premultiplied BGRA for most icons; detect it rather
    // than assume, since a few providers return straight alpha.
    let premultiplied = has_alpha && buf.chunks_exact(4).all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3]);
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2); // BGRA -> RGBA
        if !has_alpha {
            px[3] = 255;
        } else if premultiplied && px[3] > 0 && px[3] < 255 {
            let a = px[3] as u32;
            for c in px.iter_mut().take(3) {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Ok(Rgba { width: w as u32, height: h as u32, pixels: buf })
}

/// Crops transparent margins and pads back to a square with a small inset.
fn crop_to_content(img: Rgba) -> Rgba {
    let (w, h) = (img.width as usize, img.height as usize);
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if img.pixels[(y * w + x) * 4 + 3] > 10 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return img; // fully transparent; leave as is
    }
    let content = (max_x - min_x + 1).max(max_y - min_y + 1);
    let pad = (content as f32 * 0.03).round() as usize;
    let side = content + pad * 2;
    if side >= w && side >= h {
        return img;
    }
    let cx = (min_x + max_x + 1) as isize / 2;
    let cy = (min_y + max_y + 1) as isize / 2;
    let origin_x = cx - side as isize / 2;
    let origin_y = cy - side as isize / 2;
    let mut out = vec![0u8; side * side * 4];
    for y in 0..side {
        let sy = origin_y + y as isize;
        if sy < 0 || sy >= h as isize {
            continue;
        }
        for x in 0..side {
            let sx = origin_x + x as isize;
            if sx < 0 || sx >= w as isize {
                continue;
            }
            let si = (sy as usize * w + sx as usize) * 4;
            let di = (y * side + x) * 4;
            out[di..di + 4].copy_from_slice(&img.pixels[si..si + 4]);
        }
    }
    Rgba { width: side as u32, height: side as u32, pixels: out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_centers_small_content() {
        let mut pixels = vec![0u8; 100 * 100 * 4];
        for y in 40..60 {
            for x in 30..50 {
                let i = (y * 100 + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
            }
        }
        let out = crop_to_content(Rgba { width: 100, height: 100, pixels });
        assert_eq!(out.width, out.height);
        assert!(out.width >= 20 && out.width <= 24);
    }
}
