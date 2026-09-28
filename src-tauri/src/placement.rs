use tauri::{Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

pub const WIDTH: f64 = 480.0;
const BORDER_DARK: u32 = 0x0036_393A;
const BORDER_LIGHT: u32 = 0x00D4_DCDE;

fn target_monitor(window: &WebviewWindow) -> Option<Monitor> {
    let under_cursor = window
        .cursor_position()
        .ok()
        .and_then(|cursor| window.monitor_from_point(cursor.x, cursor.y).ok().flatten());
    under_cursor
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
}

fn physical_size(height: f64, scale: f64) -> PhysicalSize<u32> {
    PhysicalSize::new((WIDTH * scale).round() as u32, (height * scale).round() as u32)
}

pub fn present(window: &WebviewWindow, height: f64) -> tauri::Result<()> {
    let Some(monitor) = target_monitor(window) else {
        window.show()?;
        return window.set_focus();
    };
    let origin = monitor.position();
    let area = monitor.size();
    let size = physical_size(height, monitor.scale_factor());
    window.set_position(PhysicalPosition::new(origin.x + 8, origin.y + 8))?;
    window.set_size(size)?;
    let x = origin.x + (area.width as i32 - size.width as i32) / 2;
    let y = origin.y + (area.height as i32 - size.height as i32) / 2;
    window.set_position(PhysicalPosition::new(x, y))?;
    paint_border(window);
    window.show()?;
    window.set_focus()?;
    take_foreground(window);
    Ok(())
}

fn paint_border(window: &WebviewWindow) {
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

fn take_foreground(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let hwnd = HWND(hwnd.0);
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground == hwnd {
            return;
        }
        let their_thread = GetWindowThreadProcessId(foreground, None);
        let our_thread = GetCurrentThreadId();
        let attached = their_thread != 0
            && their_thread != our_thread
            && AttachThreadInput(our_thread, their_thread, true).as_bool();
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(our_thread, their_thread, false);
        }
    }
}

pub fn resize(window: &WebviewWindow, height: f64) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let position = window.outer_position()?;
    let current = window.outer_size()?;
    let size = physical_size(height, scale);
    if size.height == current.height {
        return Ok(());
    }
    let center_y = position.y + current.height as i32 / 2;
    let mut y = center_y - size.height as i32 / 2;
    if let Some(monitor) = window.current_monitor()? {
        let top = monitor.position().y;
        let bottom = top + monitor.size().height as i32;
        y = y.clamp(top, (bottom - size.height as i32).max(top));
    }
    window.set_size(size)?;
    window.set_position(PhysicalPosition::new(position.x, y))
}
