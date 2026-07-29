#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    if !acquire_single_instance() {
        return;
    }

    eyerest_lib::run();
}

#[cfg(target_os = "windows")]
fn acquire_single_instance() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    let handle = match unsafe { CreateMutexW(None, false, w!("Local\\EyeRest.SingleInstance")) } {
        Ok(handle) => handle,
        Err(_) => return true,
    };

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let _ = unsafe { CloseHandle(handle) };
        return false;
    }

    // The handle remains open until Windows releases it when the process exits.
    let _mutex_handle = handle;
    true
}

#[cfg(not(target_os = "windows"))]
fn acquire_single_instance() -> bool {
    true
}
