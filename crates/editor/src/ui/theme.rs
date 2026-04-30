use gpui::Hsla;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTheme {
    Default,
    Dracula,
    Solarized,
    GitHub,
}

impl SyntaxTheme {
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Dracula => "Dracula",
            Self::Solarized => "Solarized",
            Self::GitHub => "GitHub",
        }
    }

    pub fn all() -> &'static [SyntaxTheme] {
        &[Self::Default, Self::Dracula, Self::Solarized, Self::GitHub]
    }

    pub fn hue_set(self) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) {
        match self {
            Self::Default => (210.0, 140.0, 30.0, 100.0, 180.0, 10.0, 50.0, 330.0, 280.0, 60.0),
            Self::Dracula => (300.0, 60.0, 140.0, 315.0, 60.0, 180.0, 330.0, 300.0, 60.0, 60.0),
            Self::Solarized => (220.0, 68.0, 136.0, 48.0, 192.0, 20.0, 192.0, 220.0, 48.0, 48.0),
            Self::GitHub => (235.0, 0.0, 25.0, 210.0, 235.0, 25.0, 235.0, 0.0, 235.0, 0.0),
        }
    }

    pub fn link_color(self, is_dark: bool) -> Hsla {
        match self {
            Self::Default => {
                if is_dark {
                    Hsla { h: 210. / 360., s: 0.8, l: 0.65, a: 1.0 }
                } else {
                    Hsla { h: 210. / 360., s: 0.75, l: 0.45, a: 1.0 }
                }
            }
            Self::Dracula => {
                if is_dark {
                    Hsla { h: 190. / 360., s: 0.7, l: 0.7, a: 1.0 }
                } else {
                    Hsla { h: 190. / 360., s: 0.65, l: 0.4, a: 1.0 }
                }
            }
            Self::Solarized => {
                if is_dark {
                    Hsla { h: 205. / 360., s: 0.65, l: 0.6, a: 1.0 }
                } else {
                    Hsla { h: 205. / 360., s: 0.7, l: 0.42, a: 1.0 }
                }
            }
            Self::GitHub => {
                if is_dark {
                    Hsla { h: 212. / 360., s: 0.75, l: 0.65, a: 1.0 }
                } else {
                    Hsla { h: 212. / 360., s: 0.85, l: 0.45, a: 1.0 }
                }
            }
        }
    }

    pub fn highlight_color(self, is_dark: bool) -> Hsla {
        match self {
            Self::Default => {
                if is_dark {
                    Hsla { h: 45. / 360., s: 0.8, l: 0.5, a: 0.25 }
                } else {
                    Hsla { h: 45. / 360., s: 0.9, l: 0.85, a: 0.5 }
                }
            }
            Self::Dracula => {
                if is_dark {
                    Hsla { h: 50. / 360., s: 0.85, l: 0.55, a: 0.25 }
                } else {
                    Hsla { h: 50. / 360., s: 0.9, l: 0.88, a: 0.5 }
                }
            }
            Self::Solarized => {
                if is_dark {
                    Hsla { h: 44. / 360., s: 0.6, l: 0.45, a: 0.2 }
                } else {
                    Hsla { h: 44. / 360., s: 0.7, l: 0.88, a: 0.5 }
                }
            }
            Self::GitHub => {
                if is_dark {
                    Hsla { h: 51. / 360., s: 1.0, l: 0.5, a: 0.2 }
                } else {
                    Hsla { h: 51. / 360., s: 1.0, l: 0.86, a: 0.5 }
                }
            }
        }
    }

    pub fn keyword_saturation(self) -> f32 {
        match self {
            Self::Dracula => 0.75,
            Self::Solarized => 0.75,
            Self::GitHub => 0.85,
            _ => 0.7,
        }
    }

    pub fn comment_saturation(self) -> f32 {
        match self {
            Self::Dracula => 0.15,
            Self::Solarized => 0.2,
            _ => 0.3,
        }
    }
}

static CURRENT_THEME: OnceLock<std::sync::Mutex<SyntaxTheme>> = OnceLock::new();

pub fn get_syntax_theme() -> SyntaxTheme {
    *CURRENT_THEME
        .get_or_init(|| std::sync::Mutex::new(SyntaxTheme::Default))
        .lock()
        .unwrap()
}

pub fn set_syntax_theme(theme: SyntaxTheme) {
    let lock = CURRENT_THEME.get_or_init(|| std::sync::Mutex::new(SyntaxTheme::Default));
    *lock.lock().unwrap() = theme;
}
