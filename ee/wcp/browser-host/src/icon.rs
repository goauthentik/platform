//! Decodes the embedded 24-bit BMP tile icon into a `tauri::image::Image` for
//! the sign-in window's title bar / taskbar icon.

const ICON_BYTES: &[u8] = include_bytes!("../res/icon.bmp");

pub fn load() -> Option<tauri::image::Image<'static>> {
    let (width, height, rgba) = decode_bmp(ICON_BYTES)?;
    Some(tauri::image::Image::new_owned(rgba, width, height))
}

/// Minimal uncompressed 24-bit BMP decoder, enough for the one asset we
/// ship: undoes BMP's bottom-up row order and 4-byte row padding, and expands
/// each BGR pixel to opaque RGBA.
fn decode_bmp(data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if data.len() < 54 || &data[0..2] != b"BM" {
        return None;
    }
    let pixel_offset = u32::from_le_bytes(data[10..14].try_into().ok()?) as usize;
    let width = i32::from_le_bytes(data[18..22].try_into().ok()?);
    let height = i32::from_le_bytes(data[22..26].try_into().ok()?);
    let bpp = u16::from_le_bytes(data[28..30].try_into().ok()?);
    if bpp != 24 || width <= 0 || height <= 0 {
        return None;
    }

    let row_bytes = (width as usize * 3).div_ceil(4) * 4;
    let mut rgba = vec![0u8; width as usize * height as usize * 4];

    for src_row in 0..height as usize {
        let row_start = pixel_offset + src_row * row_bytes;
        if row_start + width as usize * 3 > data.len() {
            return None;
        }
        // BMP rows are stored bottom-up; flip into top-down output order.
        let dst_row = height as usize - 1 - src_row;
        for x in 0..width as usize {
            let src = row_start + x * 3;
            let dst = (dst_row * width as usize + x) * 4;
            // BMP stores BGR; `Image` wants RGBA.
            rgba[dst] = data[src + 2];
            rgba[dst + 1] = data[src + 1];
            rgba[dst + 2] = data[src];
            rgba[dst + 3] = 0xFF;
        }
    }

    Some((width as u32, height as u32, rgba))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The shipped asset has to stay a bottom-up 24-bit `BI_RGB` BMP. A
    /// 32-bit `BITMAPV5HEADER`/`BI_BITFIELDS` export decodes to `None` here,
    /// silently dropping the window icon — and for the same asset in
    /// `credprovider` it makes `LoadImage` fail outright, rendering the logon
    /// tile blank.
    #[test]
    fn decodes_the_shipped_asset() {
        let (width, height, rgba) =
            decode_bmp(ICON_BYTES).expect("icon.bmp must be a bottom-up 24-bit BI_RGB BMP");
        assert_eq!((width, height), (128, 128));
        assert_eq!(rgba.len(), 128 * 128 * 4);
        assert!(
            rgba.chunks_exact(4).all(|px| px[3] == 0xFF),
            "every pixel should be opaque"
        );
    }
}
