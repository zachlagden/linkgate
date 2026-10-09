use tauri::WebviewWindow;
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};

const BORDER_DARK: u32 = 0x0036_393A;
const BORDER_LIGHT: u32 = 0x00D4_DCDE;

pub fn paint(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let color = match window.theme() {
        Ok(tauri::Theme::Light) => BORDER_LIGHT,
        _ => BORDER_DARK,
    };
    let value = COLORREF(color);
    unsafe {
        let _ = DwmSetWindowAttribute(
            HWND(hwnd.0),
            DWMWA_BORDER_COLOR,
            &value as *const COLORREF as *const _,
            std::mem::size_of::<COLORREF>() as u32,
        );
    }
}
