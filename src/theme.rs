use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const fn c(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

fn parse_hex(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    if let Ok(v) = u32::from_str_radix(s, 16) {
        Color::Rgb(
            ((v >> 16) & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            (v & 0xFF) as u8,
        )
    } else {
        Color::Reset
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeColors {
    pub accent: Option<String>,
    pub bg: Option<String>,
    pub fg: Option<String>,
    pub selected_bg: Option<String>,
    pub header_bg: Option<String>,
    pub header_fg: Option<String>,
    pub footer_bg: Option<String>,
    pub footer_fg: Option<String>,
    pub border: Option<String>,
    pub row_alt_bg: Option<String>,
    pub detail_label: Option<String>,
    pub detail_value: Option<String>,
    pub detail_border: Option<String>,
    pub status_green: Option<String>,
    pub status_yellow: Option<String>,
    pub status_red: Option<String>,
    pub status_blue_gray: Option<String>,
    pub status_brown: Option<String>,
    pub status_warm_red: Option<String>,
    pub priority_p1: Option<String>,
    pub priority_p2: Option<String>,
    pub priority_p3: Option<String>,
    pub priority_p4: Option<String>,
    pub priority_p5: Option<String>,
    pub loading_fg: Option<String>,
    pub error_fg: Option<String>,
    pub tick_fg: Option<String>,
}

impl ThemeColors {
    pub fn merge_into(self, base: &mut Theme) {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(v) = self.$field {
                    base.$field = parse_hex(&v);
                }
            };
        }
        apply!(accent);
        apply!(bg);
        apply!(fg);
        apply!(selected_bg);
        apply!(header_bg);
        apply!(header_fg);
        apply!(footer_bg);
        apply!(footer_fg);
        apply!(border);
        apply!(row_alt_bg);
        apply!(detail_label);
        apply!(detail_value);
        apply!(detail_border);
        if let Some(v) = self.status_green {
            base.priority_p3 = parse_hex(&v);
        }
        if let Some(v) = self.status_yellow {
            base.loading_fg = parse_hex(&v);
        }
        if let Some(v) = self.status_red {
            base.error_fg = parse_hex(&v);
        }
        if let Some(v) = self.status_blue_gray {
            base.detail_border = parse_hex(&v);
        }
        if let Some(v) = self.status_brown {
            base.priority_p2 = parse_hex(&v);
        }
        if let Some(v) = self.status_warm_red {
            base.priority_p1 = parse_hex(&v);
        }
        apply!(priority_p1);
        apply!(priority_p2);
        apply!(priority_p3);
        apply!(priority_p4);
        apply!(priority_p5);
        apply!(loading_fg);
        apply!(error_fg);
        apply!(tick_fg);
    }
}

#[derive(Clone)]
pub struct Theme {
    pub selected_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub row_alt_bg: Color,
    pub border: Color,
    pub footer_bg: Color,
    pub footer_fg: Color,
    pub detail_label: Color,
    pub detail_value: Color,
    pub detail_border: Color,
    pub priority_p1: Color,
    pub priority_p2: Color,
    pub priority_p3: Color,
    pub priority_p4: Color,
    pub priority_p5: Color,
    pub loading_fg: Color,
    pub error_fg: Color,
    pub tick_fg: Color,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: c(0x89B4FA),
            bg: c(0x11111B),
            fg: c(0xCDD6F4),
            selected_bg: c(0x2E3C4F),
            header_bg: c(0x1F2A3A),
            header_fg: c(0x89B4FA),
            row_fg: c(0xCDD6F4),
            row_alt_bg: c(0x181825),
            border: c(0x45475A),
            footer_bg: c(0x1E1E2E),
            footer_fg: c(0xA6ADC8),
            detail_label: c(0x89B4FA),
            detail_value: c(0xCDD6F4),
            detail_border: c(0x45475A),
            priority_p1: c(0xF38BA8),
            priority_p2: c(0xFAB387),
            priority_p3: c(0xA6E3A1),
            priority_p4: c(0x94E2D5),
            priority_p5: c(0x89B4FA),
            loading_fg: c(0xF9E2AF),
            error_fg: c(0xF38BA8),
            tick_fg: c(0x89B4FA),
        }
    }
}

impl Theme {
    pub fn light() -> Self {
        Self {
            accent: c(0x1E66F5),
            bg: c(0xEFF1F5),
            fg: c(0x4C4F69),
            selected_bg: c(0xDCE0E8),
            header_bg: c(0xE6E9EF),
            header_fg: c(0x1E66F5),
            row_fg: c(0x4C4F69),
            row_alt_bg: c(0xEFF1F5),
            border: c(0xBCC0CC),
            footer_bg: c(0xE6E9EF),
            footer_fg: c(0x5C5F77),
            detail_label: c(0x1E66F5),
            detail_value: c(0x4C4F69),
            detail_border: c(0xBCC0CC),
            priority_p1: c(0xD20F39),
            priority_p2: c(0xFE640B),
            priority_p3: c(0x40A02B),
            priority_p4: c(0x04A5E5),
            priority_p5: c(0x1E66F5),
            loading_fg: c(0xD20F39),
            error_fg: c(0xD20F39),
            tick_fg: c(0x1E66F5),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            accent: c(0x7AA2F7),
            bg: c(0x1A1B26),
            fg: c(0xA9B1D6),
            selected_bg: c(0x283457),
            header_bg: c(0x16161E),
            header_fg: c(0x7AA2F7),
            row_fg: c(0xA9B1D6),
            row_alt_bg: c(0x1F2135),
            border: c(0x3B4261),
            footer_bg: c(0x16161E),
            footer_fg: c(0x565F89),
            detail_label: c(0x7AA2F7),
            detail_value: c(0xA9B1D6),
            detail_border: c(0x3B4261),
            priority_p1: c(0xF7768E),
            priority_p2: c(0xFF9E64),
            priority_p3: c(0x9ECE6A),
            priority_p4: c(0x7DCFFF),
            priority_p5: c(0x7AA2F7),
            loading_fg: c(0xE0AF68),
            error_fg: c(0xF7768E),
            tick_fg: c(0x7AA2F7),
        }
    }

    pub fn dracula() -> Self {
        Self {
            accent: c(0xBD93F9),
            bg: c(0x282A36),
            fg: c(0xF8F8F2),
            selected_bg: c(0x44475A),
            header_bg: c(0x21222C),
            header_fg: c(0xBD93F9),
            row_fg: c(0xF8F8F2),
            row_alt_bg: c(0x2C2E3E),
            border: c(0x6272A4),
            footer_bg: c(0x21222C),
            footer_fg: c(0x6272A4),
            detail_label: c(0xBD93F9),
            detail_value: c(0xF8F8F2),
            detail_border: c(0x6272A4),
            priority_p1: c(0xFF5555),
            priority_p2: c(0xFFB86C),
            priority_p3: c(0x50FA7B),
            priority_p4: c(0x8BE9FD),
            priority_p5: c(0xBD93F9),
            loading_fg: c(0xF1FA8C),
            error_fg: c(0xFF5555),
            tick_fg: c(0xBD93F9),
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            accent: c(0x83A598),
            bg: c(0x282828),
            fg: c(0xEBDBB2),
            selected_bg: c(0x3C3836),
            header_bg: c(0x1D2021),
            header_fg: c(0x83A598),
            row_fg: c(0xEBDBB2),
            row_alt_bg: c(0x32302F),
            border: c(0x504945),
            footer_bg: c(0x1D2021),
            footer_fg: c(0x928374),
            detail_label: c(0x83A598),
            detail_value: c(0xEBDBB2),
            detail_border: c(0x504945),
            priority_p1: c(0xFB4934),
            priority_p2: c(0xFE8019),
            priority_p3: c(0xB8BB26),
            priority_p4: c(0x8EC07C),
            priority_p5: c(0x83A598),
            loading_fg: c(0xFABD2F),
            error_fg: c(0xFB4934),
            tick_fg: c(0x83A598),
        }
    }

    pub fn nord() -> Self {
        Self {
            accent: c(0x88C0D0),
            bg: c(0x2E3440),
            fg: c(0xECEFF4),
            selected_bg: c(0x3B4252),
            header_bg: c(0x242933),
            header_fg: c(0x88C0D0),
            row_fg: c(0xECEFF4),
            row_alt_bg: c(0x343D4F),
            border: c(0x4C566A),
            footer_bg: c(0x242933),
            footer_fg: c(0xD8DEE9),
            detail_label: c(0x88C0D0),
            detail_value: c(0xECEFF4),
            detail_border: c(0x4C566A),
            priority_p1: c(0xBF616A),
            priority_p2: c(0xD08770),
            priority_p3: c(0xA3BE8C),
            priority_p4: c(0x8FBCBB),
            priority_p5: c(0x81A1C1),
            loading_fg: c(0xEBCB8B),
            error_fg: c(0xBF616A),
            tick_fg: c(0x88C0D0),
        }
    }

    pub fn one_dark() -> Self {
        Self {
            accent: c(0x61AFEF),
            bg: c(0x282C34),
            fg: c(0xABB2BF),
            selected_bg: c(0x3E4451),
            header_bg: c(0x21252B),
            header_fg: c(0x61AFEF),
            row_fg: c(0xABB2BF),
            row_alt_bg: c(0x2C313A),
            border: c(0x4B5263),
            footer_bg: c(0x21252B),
            footer_fg: c(0x5C6370),
            detail_label: c(0x61AFEF),
            detail_value: c(0xABB2BF),
            detail_border: c(0x4B5263),
            priority_p1: c(0xE06C75),
            priority_p2: c(0xD19A66),
            priority_p3: c(0x98C379),
            priority_p4: c(0x56B6C2),
            priority_p5: c(0x61AFEF),
            loading_fg: c(0xE5C07B),
            error_fg: c(0xE06C75),
            tick_fg: c(0x61AFEF),
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            accent: c(0x268BD2),
            bg: c(0x002B36),
            fg: c(0x839496),
            selected_bg: c(0x073642),
            header_bg: c(0x001F27),
            header_fg: c(0x268BD2),
            row_fg: c(0x93A1A1),
            row_alt_bg: c(0x00212C),
            border: c(0x586E75),
            footer_bg: c(0x001F27),
            footer_fg: c(0x657B83),
            detail_label: c(0x268BD2),
            detail_value: c(0x93A1A1),
            detail_border: c(0x586E75),
            priority_p1: c(0xDC322F),
            priority_p2: c(0xCB4B16),
            priority_p3: c(0x859900),
            priority_p4: c(0x2AA198),
            priority_p5: c(0x268BD2),
            loading_fg: c(0xB58900),
            error_fg: c(0xDC322F),
            tick_fg: c(0x268BD2),
        }
    }

    pub fn rose_pine() -> Self {
        Self {
            accent: c(0xC4A7E7),
            bg: c(0x191724),
            fg: c(0xE0DEF4),
            selected_bg: c(0x26233A),
            header_bg: c(0x1F1D2E),
            header_fg: c(0xC4A7E7),
            row_fg: c(0xE0DEF4),
            row_alt_bg: c(0x1E1B2E),
            border: c(0x6E6A86),
            footer_bg: c(0x1F1D2E),
            footer_fg: c(0x908CAA),
            detail_label: c(0xC4A7E7),
            detail_value: c(0xE0DEF4),
            detail_border: c(0x6E6A86),
            priority_p1: c(0xEB6F92),
            priority_p2: c(0xF6C177),
            priority_p3: c(0x9CCFD8),
            priority_p4: c(0x31748F),
            priority_p5: c(0xC4A7E7),
            loading_fg: c(0xF6C177),
            error_fg: c(0xEB6F92),
            tick_fg: c(0xC4A7E7),
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self::default()
    }

    pub fn all_builtin() -> HashMap<&'static str, Theme> {
        let mut m = HashMap::new();
        m.insert("default", Self::default());
        m.insert("catppuccin-mocha", Self::catppuccin_mocha());
        m.insert("light", Self::light());
        m.insert("tokyo-night", Self::tokyo_night());
        m.insert("dracula", Self::dracula());
        m.insert("gruvbox-dark", Self::gruvbox_dark());
        m.insert("nord", Self::nord());
        m.insert("one-dark", Self::one_dark());
        m.insert("solarized-dark", Self::solarized_dark());
        m.insert("rose-pine", Self::rose_pine());
        m
    }

    pub fn from_file(path: &Path) -> Result<Theme, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read theme file {}: {}", path.display(), e))?;
        let colors: ThemeColors = toml::from_str(&contents)
            .map_err(|e| format!("Invalid theme file {}: {}", path.display(), e))?;
        let mut theme = Theme::default();
        colors.merge_into(&mut theme);
        Ok(theme)
    }

    pub fn themes_dir() -> Result<std::path::PathBuf, String> {
        let dir = dirs::config_dir()
            .ok_or_else(|| "Cannot determine config directory".to_string())?
            .join("tick")
            .join("themes");
        Ok(dir)
    }

    pub fn list_available() -> Vec<String> {
        let mut names: Vec<String> = Self::all_builtin().keys().map(|s| s.to_string()).collect();
        if let Ok(dir) = Self::themes_dir() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "toml") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            names.push(stem.to_string());
                        }
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    pub fn resolve(name: &str) -> Result<Theme, String> {
        let builtin = Self::all_builtin();
        if let Some(t) = builtin.get(name) {
            return Ok(t.clone());
        }
        let themes_dir = dirs::config_dir()
            .ok_or_else(|| "Cannot determine config directory".to_string())?
            .join("tick")
            .join("themes");
        let path = themes_dir.join(format!("{}.toml", name));
        if path.exists() {
            Self::from_file(&path)
        } else {
            let names: Vec<&str> = builtin.keys().copied().collect();
            Err(format!(
                "Theme '{}' not found. Available: {} | Or create ~/.config/tick/themes/{}.toml",
                name,
                names.join(", "),
                name
            ))
        }
    }

    pub fn priority_style(&self, priority: &str) -> Style {
        let color = match priority {
            "Highest" | "Blocker" | "P1" => self.priority_p1,
            "High" | "Critical" | "P2" => self.priority_p2,
            "Medium" | "Major" | "P3" => self.priority_p3,
            "Low" | "Minor" | "P4" => self.priority_p4,
            "Lowest" | "Trivial" | "P5" => self.priority_p5,
            _ => self.row_fg,
        };
        Style::default().fg(color)
    }

    pub fn status_style(&self, color_name: &str) -> Style {
        let color = match color_name {
            "blue-gray" => self.detail_border,
            "yellow" => self.loading_fg,
            "green" | "green-with-done" => self.priority_p3,
            "brown" => self.priority_p2,
            "warm-red" => self.priority_p1,
            _ => self.row_fg,
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    pub fn selected_style(&self) -> Style {
        Style::default()
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.header_fg)
            .bg(self.header_bg)
            .add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_themes_resolve() {
        for name in Theme::all_builtin().keys() {
            let theme = Theme::resolve(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_ne!(theme.fg, Color::Reset);
        }
    }

    #[test]
    fn catppuccin_mocha_matches_default() {
        let a = Theme::resolve("default").unwrap();
        let b = Theme::resolve("catppuccin-mocha").unwrap();
        assert_eq!(a.accent, b.accent);
        assert_eq!(a.bg, b.bg);
    }

    #[test]
    fn from_file_overrides_accent() {
        let dir = std::env::temp_dir().join(format!("tick-theme-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("custom-test.toml");
        fs::write(&path, "accent = \"#FF0000\"\n").unwrap();
        let theme = Theme::from_file(&path).unwrap();
        assert_eq!(theme.accent, parse_hex("#FF0000"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_theme_errors() {
        assert!(Theme::resolve("not-a-real-theme-xyz").is_err());
    }

    #[test]
    fn priority_and_status_styles_use_theme_colors() {
        let theme = Theme::dracula();
        let p = theme.priority_style("Highest");
        assert_eq!(p.fg, Some(theme.priority_p1));
        let s = theme.status_style("green");
        assert_eq!(s.fg, Some(theme.priority_p3));
    }

    #[test]
    fn list_available_includes_new_builtins() {
        let names = Theme::list_available();
        for expected in [
            "gruvbox-dark",
            "nord",
            "one-dark",
            "solarized-dark",
            "rose-pine",
            "catppuccin-mocha",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing theme {expected}: {names:?}"
            );
        }
    }
}
