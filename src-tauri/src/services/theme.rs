use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTheme {
    pub id: String,
    pub name: String,
    pub background: String,
    pub text_color: String,
    pub muted_text_color: String,
    pub accent_color: String,
    pub danger_color: String,
    pub button_background: String,
    pub button_text_color: String,
    pub button_radius: u16,
    pub font_family: String,
    pub background_image: Option<String>,
    pub layout: String,
    pub reduced_motion: bool,
    pub high_contrast: bool,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            id: "default-dark".into(),
            name: "Default Dark".into(),
            background: "#111111".into(),
            text_color: "#ffffff".into(),
            muted_text_color: "#c8c8c8".into(),
            accent_color: "#62d26f".into(),
            danger_color: "#f05d5e".into(),
            button_background: "#ffffff".into(),
            button_text_color: "#111111".into(),
            button_radius: 12,
            font_family: "Inter, Segoe UI, sans-serif".into(),
            background_image: None,
            layout: "centered".into(),
            reduced_motion: false,
            high_contrast: false,
        }
    }
}

impl OverlayTheme {
    pub fn normalized(mut self) -> AppResult<Self> {
        self.id = normalize_id(&self.id);
        if self.id.is_empty() {
            self.id = "custom-theme".into();
        }

        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            self.name = "Custom Theme".into();
        }

        self.background = normalize_hex_color(&self.background)?;
        self.text_color = normalize_hex_color(&self.text_color)?;
        self.muted_text_color = normalize_hex_color(&self.muted_text_color)?;
        self.accent_color = normalize_hex_color(&self.accent_color)?;
        self.danger_color = normalize_hex_color(&self.danger_color)?;
        self.button_background = normalize_hex_color(&self.button_background)?;
        self.button_text_color = normalize_hex_color(&self.button_text_color)?;
        self.button_radius = self.button_radius.min(32);
        self.font_family = self.font_family.trim().to_string();
        if self.font_family.is_empty() {
            self.font_family = OverlayTheme::default().font_family;
        }

        if !matches!(self.layout.as_str(), "centered" | "calm" | "compact") {
            self.layout = "centered".into();
        }

        validate_contrast(&self)?;
        Ok(self)
    }
}

pub fn validate_theme(theme: &OverlayTheme) -> AppResult<()> {
    validate_contrast(theme)
}

fn validate_contrast(theme: &OverlayTheme) -> AppResult<()> {
    let background = parse_hex_rgb(&theme.background)?;
    let text = parse_hex_rgb(&theme.text_color)?;
    let muted = parse_hex_rgb(&theme.muted_text_color)?;
    let button_background = parse_hex_rgb(&theme.button_background)?;
    let button_text = parse_hex_rgb(&theme.button_text_color)?;

    if contrast_ratio(background, text) < 4.5 {
        return Err(AppError::from(
            "Theme text color must have at least 4.5:1 contrast against the background",
        ));
    }

    if contrast_ratio(background, muted) < 3.0 {
        return Err(AppError::from(
            "Theme muted text color must have at least 3:1 contrast against the background",
        ));
    }

    if contrast_ratio(button_background, button_text) < 4.5 {
        return Err(AppError::from(
            "Button text color must have at least 4.5:1 contrast against the button background",
        ));
    }

    Ok(())
}

fn normalize_id(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn normalize_hex_color(value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    let expanded = match hex.len() {
        3 => hex.chars().flat_map(|ch| [ch, ch]).collect::<String>(),
        6 => hex.to_string(),
        _ => {
            return Err(AppError::from(
                "Theme colors must be #RGB or #RRGGBB values",
            ))
        }
    };

    if !expanded.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(AppError::from("Theme colors must be valid hex colors"));
    }

    Ok(format!("#{}", expanded.to_ascii_lowercase()))
}

fn parse_hex_rgb(value: &str) -> AppResult<(u8, u8, u8)> {
    let normalized = normalize_hex_color(value)?;
    let hex = normalized.trim_start_matches('#');
    let red =
        u8::from_str_radix(&hex[0..2], 16).map_err(|_| AppError::from("Invalid red channel"))?;
    let green =
        u8::from_str_radix(&hex[2..4], 16).map_err(|_| AppError::from("Invalid green channel"))?;
    let blue =
        u8::from_str_radix(&hex[4..6], 16).map_err(|_| AppError::from("Invalid blue channel"))?;
    Ok((red, green, blue))
}

fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance((red, green, blue): (u8, u8, u8)) -> f64 {
    fn channel(value: u8) -> f64 {
        let value = value as f64 / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_short_hex_colors() {
        let mut theme = OverlayTheme::default();
        theme.background = "#000".into();
        assert_eq!(theme.normalized().unwrap().background, "#000000");
    }

    #[test]
    fn rejects_low_contrast_text() {
        let mut theme = OverlayTheme::default();
        theme.text_color = "#222222".into();
        assert!(theme.normalized().is_err());
    }
}
