use crate::error::AppResult;

pub fn play_rest_complete_sound(enabled: bool) -> AppResult<()> {
    if !enabled {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Diagnostics::Debug::MessageBeep;
        use windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION;

        MessageBeep(MB_ICONINFORMATION).map_err(|error| error.to_string())?;
    }

    #[cfg(not(target_os = "windows"))]
    print!("\x07");

    Ok(())
}
