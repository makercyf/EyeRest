#![allow(dead_code)]

#[cfg(target_os = "windows")]
pub fn platform_name() -> &'static str {
    "windows"
}

#[cfg(not(target_os = "windows"))]
pub fn platform_name() -> &'static str {
    "unsupported"
}
