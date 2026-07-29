use crate::core::suppression::SuppressionReason;
use crate::error::AppResult;

#[cfg(target_os = "windows")]
use windows::core::BOOL;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentProcessId;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, GW_OWNER, WS_CAPTION, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_THICKFRAME,
};

pub fn reason() -> SuppressionReason {
    SuppressionReason::FullscreenApp
}

pub fn is_any_fullscreen_window() -> AppResult<bool> {
    #[cfg(target_os = "windows")]
    {
        let mut fullscreen_found = false;
        unsafe {
            EnumWindows(
                Some(find_fullscreen_window),
                LPARAM((&mut fullscreen_found as *mut bool) as isize),
            )
            .map_err(|error| crate::error::AppError::from(error.to_string()))?;
        }
        Ok(fullscreen_found)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn find_fullscreen_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found = unsafe { &mut *(lparam.0 as *mut bool) };
    if is_fullscreen_window(hwnd) {
        *found = true;
    }

    // Returning FALSE makes EnumWindows look like an API failure to the Rust binding.
    // Continue enumeration after a match so a fullscreen window on any monitor suppresses reminders.
    BOOL(1)
}

#[cfg(target_os = "windows")]
fn is_fullscreen_window(hwnd: HWND) -> bool {
    if hwnd.0.is_null()
        || !unsafe { IsWindowVisible(hwnd) }.as_bool()
        || unsafe { IsIconic(hwnd) }.as_bool()
        || belongs_to_current_process(hwnd)
        || is_cloaked(hwnd)
        || has_owner(hwnd)
        || has_non_app_extended_style(hwnd)
    {
        return false;
    }

    let mut window_rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut window_rect) }.is_err() {
        return false;
    }

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.0.is_null() {
        return false;
    }

    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool()
        || !rects_cover_same_area(window_rect, monitor_info.rcMonitor)
    {
        return false;
    }

    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    let has_normal_frame = (style & WS_CAPTION.0) != 0 && (style & WS_THICKFRAME.0) != 0;
    !has_normal_frame
}

#[cfg(target_os = "windows")]
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
    }
}

#[cfg(target_os = "windows")]
fn has_owner(hwnd: HWND) -> bool {
    unsafe { GetWindow(hwnd, GW_OWNER) }
        .map(|owner| !owner.0.is_null())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn has_non_app_extended_style(hwnd: HWND) -> bool {
    let extended_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    (extended_style & WS_EX_TOOLWINDOW.0) != 0 || (extended_style & WS_EX_NOACTIVATE.0) != 0
}

#[cfg(target_os = "windows")]
fn belongs_to_current_process(hwnd: HWND) -> bool {
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        process_id == GetCurrentProcessId()
    }
}

#[cfg(target_os = "windows")]
fn rects_cover_same_area(window: RECT, monitor: RECT) -> bool {
    const TOLERANCE: i32 = 2;
    (window.left - monitor.left).abs() <= TOLERANCE
        && (window.top - monitor.top).abs() <= TOLERANCE
        && (window.right - monitor.right).abs() <= TOLERANCE
        && (window.bottom - monitor.bottom).abs() <= TOLERANCE
}
