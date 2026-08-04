#![allow(dead_code)]

#[cfg(target_os = "windows")]
use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
use tauri::{Runtime, WebviewWindow, Window};

#[cfg(target_os = "windows")]
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            WindowsAndMessaging::{
                LoadImageW, SendMessageW, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_SHARED, SM_CXICON,
                SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
            },
        },
    },
};

#[cfg(target_os = "windows")]
const APP_ICON_RESOURCE_ID: usize = 32512;
#[cfg(target_os = "windows")]
const DEFAULT_DPI: u32 = 96;

#[cfg(target_os = "windows")]
pub fn platform_name() -> &'static str {
    "windows"
}

#[cfg(not(target_os = "windows"))]
pub fn platform_name() -> &'static str {
    "unsupported"
}

#[cfg(target_os = "windows")]
pub fn apply_dpi_aware_webview_icons<R: Runtime>(window: &WebviewWindow<R>) -> AppResult<()> {
    apply_dpi_aware_icons(window.hwnd()?)
}

#[cfg(target_os = "windows")]
pub fn apply_dpi_aware_window_icons<R: Runtime>(window: &Window<R>) -> AppResult<()> {
    apply_dpi_aware_icons(window.hwnd()?)
}

#[cfg(target_os = "windows")]
fn apply_dpi_aware_icons(hwnd: HWND) -> AppResult<()> {
    let reported_dpi = unsafe { GetDpiForWindow(hwnd) };
    let dpi = if reported_dpi == 0 {
        DEFAULT_DPI
    } else {
        reported_dpi
    };
    let small_size = icon_size_for_dpi(SM_CXSMICON, SM_CYSMICON, dpi)?;
    let taskbar_size = icon_size_for_dpi(SM_CXICON, SM_CYICON, dpi)?;
    let module = unsafe { GetModuleHandleW(None) }
        .map_err(|error| AppError::from(format!("could not locate app icon resources: {error}")))?;
    let instance: HINSTANCE = module.into();

    let small_icon = load_icon_resource(instance, small_size)?;
    let taskbar_icon = load_icon_resource(instance, taskbar_size)?;

    unsafe {
        SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(small_icon.0 as isize)),
        );
        SendMessageW(
            hwnd,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(taskbar_icon.0 as isize)),
        );
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn icon_size_for_dpi(
    width_metric: windows::Win32::UI::WindowsAndMessaging::SYSTEM_METRICS_INDEX,
    height_metric: windows::Win32::UI::WindowsAndMessaging::SYSTEM_METRICS_INDEX,
    dpi: u32,
) -> AppResult<(i32, i32)> {
    let width = unsafe { GetSystemMetricsForDpi(width_metric, dpi) };
    let height = unsafe { GetSystemMetricsForDpi(height_metric, dpi) };
    if width <= 0 || height <= 0 {
        return Err(AppError::from(format!(
            "Windows returned an invalid icon size {width}x{height} for {dpi} DPI"
        )));
    }
    Ok((width, height))
}

#[cfg(target_os = "windows")]
fn load_icon_resource(
    instance: HINSTANCE,
    (width, height): (i32, i32),
) -> AppResult<windows::Win32::Foundation::HANDLE> {
    let resource_name = PCWSTR(APP_ICON_RESOURCE_ID as *const u16);
    unsafe {
        LoadImageW(
            Some(instance),
            resource_name,
            IMAGE_ICON,
            width,
            height,
            LR_SHARED,
        )
    }
    .map_err(|error| {
        AppError::from(format!(
            "could not load {width}x{height} app icon resource: {error}"
        ))
    })
}
