use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
pub fn sync(enabled: bool) -> AppResult<()> {
    use std::slice;
    use windows::core::{w, PCWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let mut key = HKEY::default();
    let open_status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    };
    ensure_success(open_status.0, "open the Windows startup registry key")?;

    let result = if enabled {
        let executable = std::env::current_exe()?;
        let command = format!("\"{}\"", executable.display());
        let utf16 = command
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let bytes = unsafe {
            slice::from_raw_parts(
                utf16.as_ptr().cast::<u8>(),
                std::mem::size_of_val(utf16.as_slice()),
            )
        };
        let status = unsafe { RegSetValueExW(key, w!("EyeRest"), None, REG_SZ, Some(bytes)) };
        ensure_success(status.0, "register EyeRest for Windows startup")
    } else {
        let status = unsafe { RegDeleteValueW(key, w!("EyeRest")) };
        if status.0 == 0 || status.0 == 2 {
            Ok(())
        } else {
            ensure_success(status.0, "remove EyeRest from Windows startup")
        }
    };

    let _ = unsafe { RegCloseKey(key) };
    result
}

#[cfg(target_os = "windows")]
fn ensure_success(status: u32, action: &str) -> AppResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(AppError::from(format!(
            "Could not {action} (Windows error {status})."
        )))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn sync(_enabled: bool) -> AppResult<()> {
    Ok(())
}
