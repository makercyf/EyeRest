use crate::core::suppression::SuppressionReason;
use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
use windows::Win32::System::SystemInformation::GetTickCount;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

pub fn reason() -> SuppressionReason {
    SuppressionReason::Idle
}

pub fn is_idle(threshold_seconds: u64) -> AppResult<bool> {
    #[cfg(target_os = "windows")]
    {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };

        let ok = unsafe { GetLastInputInfo(&mut info) };
        if !ok.as_bool() {
            return Err(AppError::from("GetLastInputInfo failed"));
        }

        let now = unsafe { GetTickCount() };
        let elapsed_ms = now.saturating_sub(info.dwTime) as u64;
        Ok(elapsed_ms >= threshold_seconds.saturating_mul(1000))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = threshold_seconds;
        Ok(false)
    }
}
