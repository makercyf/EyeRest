use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
use std::path::Path;
#[cfg(target_os = "windows")]
use windows::core::{PCWSTR, PWSTR};
#[cfg(target_os = "windows")]
use windows::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST,
    OPENFILENAMEW,
};

pub fn pick_executable_name() -> AppResult<Option<String>> {
    #[cfg(target_os = "windows")]
    {
        let mut file_buffer = vec![0u16; 32_768];
        let filter = wide("Executable files (*.exe)\0*.exe\0\0");
        let title = wide("Select an executable");
        let mut dialog = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            lpstrFilter: PCWSTR(filter.as_ptr()),
            lpstrFile: PWSTR(file_buffer.as_mut_ptr()),
            nMaxFile: file_buffer.len() as u32,
            lpstrTitle: PCWSTR(title.as_ptr()),
            Flags: OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
            ..Default::default()
        };

        if unsafe { GetOpenFileNameW(&mut dialog) }.as_bool() {
            let length = file_buffer
                .iter()
                .position(|character| *character == 0)
                .unwrap_or_default();
            let path = String::from_utf16_lossy(&file_buffer[..length]);
            let name = Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_uppercase())
                .ok_or_else(|| AppError::from("selected executable has no file name"))?;
            return Ok(Some(name));
        }

        let error = unsafe { CommDlgExtendedError() };
        if error.0 == 0 {
            return Ok(None);
        }
        return Err(AppError::from(format!(
            "could not open executable picker (Windows error {})",
            error.0
        )));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}
