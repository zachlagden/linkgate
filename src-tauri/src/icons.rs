use std::hash::{Hash, Hasher};
use std::path::Path;

use base64::Engine;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::UI::Shell::SHDefExtractIconW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

use crate::{logging, paths};

const ICON_SIZE: u32 = 64;

pub fn data_url(file: &Path, index: i32) -> Option<String> {
    let png = cached_png(file, index)?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}

fn cache_key(file: &Path, index: i32) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    file.to_string_lossy().to_lowercase().hash(&mut hasher);
    index.hash(&mut hasher);
    std::fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .hash(&mut hasher);
    format!("{:016x}.png", hasher.finish())
}

fn cached_png(file: &Path, index: i32) -> Option<Vec<u8>> {
    let cache = paths::icons_dir().join(cache_key(file, index));
    if let Ok(bytes) = std::fs::read(&cache) {
        return Some(bytes);
    }
    let png = match extract_png(file, index) {
        Ok(png) => png,
        Err(error) => {
            logging::error("icon_extract_failed", format!("{}: {error}", file.display()));
            return None;
        }
    };
    if std::fs::create_dir_all(paths::icons_dir()).is_ok() {
        let _ = std::fs::write(&cache, &png);
    }
    Some(png)
}

fn extract_png(file: &Path, index: i32) -> Result<Vec<u8>, String> {
    let wide: Vec<u16> = file
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut icon = HICON::default();
    unsafe {
        SHDefExtractIconW(PCWSTR(wide.as_ptr()), index, 0, Some(&mut icon), None, ICON_SIZE)
            .ok()
            .map_err(|e| e.to_string())?;
    }
    if icon.is_invalid() {
        return Err("no icon".into());
    }
    let result = unsafe { icon_to_rgba(icon) };
    unsafe {
        let _ = DestroyIcon(icon);
    }
    let (width, height, rgba) = result?;
    encode_png(width, height, &rgba)
}

unsafe fn icon_to_rgba(icon: HICON) -> Result<(u32, u32, Vec<u8>), String> {
    let mut info = ICONINFO::default();
    GetIconInfo(icon, &mut info).map_err(|e| e.to_string())?;
    let color = info.hbmColor;
    let mask = info.hbmMask;
    let cleanup = || {
        let _ = DeleteObject(HGDIOBJ(color.0));
        let _ = DeleteObject(HGDIOBJ(mask.0));
    };
    if color.is_invalid() {
        cleanup();
        return Err("monochrome icon".into());
    }
    let mut bitmap = BITMAP::default();
    let read = GetObjectW(
        HGDIOBJ(color.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bitmap as *mut _ as *mut _),
    );
    if read == 0 {
        cleanup();
        return Err("unreadable bitmap".into());
    }
    let width = bitmap.bmWidth as u32;
    let height = bitmap.bmHeight as u32;
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let dc = CreateCompatibleDC(None);
    let lines = GetDIBits(
        dc,
        color,
        0,
        height,
        Some(pixels.as_mut_ptr() as *mut _),
        &mut header,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);
    cleanup();
    if lines == 0 {
        return Err("GetDIBits failed".into());
    }
    let has_alpha = pixels.chunks_exact(4).any(|p| p[3] != 0);
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        if !has_alpha {
            pixel[3] = 255;
        }
    }
    Ok((width, height, pixels))
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
