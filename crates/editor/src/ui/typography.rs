use std::sync::atomic::{AtomicU16, Ordering};

pub const DEFAULT_BODY_FONT_SIZE: u16 = 17;
pub const MIN_BODY_FONT_SIZE: u16 = 12;
pub const MAX_BODY_FONT_SIZE: u16 = 28;

const DEFAULT_BODY_LINE_HEIGHT: f32 = 28.;
const DEFAULT_CODE_FONT_SIZE: f32 = 15.;
const DEFAULT_CODE_LINE_HEIGHT: f32 = 24.;

static BODY_FONT_SIZE: AtomicU16 = AtomicU16::new(DEFAULT_BODY_FONT_SIZE);

pub fn normalize_body_font_size(size: u16) -> u16 {
    size.clamp(MIN_BODY_FONT_SIZE, MAX_BODY_FONT_SIZE)
}

pub fn set_body_font_size(size: u16) -> u16 {
    let normalized = normalize_body_font_size(size);
    BODY_FONT_SIZE.store(normalized, Ordering::Relaxed);
    normalized
}

pub(crate) fn body_font_size() -> f32 {
    f32::from(BODY_FONT_SIZE.load(Ordering::Relaxed))
}

pub(crate) fn body_line_height() -> f32 {
    scale_typography(DEFAULT_BODY_LINE_HEIGHT)
}

pub(crate) fn code_font_size() -> f32 {
    scale_typography(DEFAULT_CODE_FONT_SIZE)
}

pub(crate) fn code_line_height() -> f32 {
    scale_typography(DEFAULT_CODE_LINE_HEIGHT)
}

pub(crate) fn scale_typography(value: f32) -> f32 {
    value * body_font_size() / f32::from(DEFAULT_BODY_FONT_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_body_font_size_bounds() {
        assert_eq!(normalize_body_font_size(0), MIN_BODY_FONT_SIZE);
        assert_eq!(normalize_body_font_size(DEFAULT_BODY_FONT_SIZE), DEFAULT_BODY_FONT_SIZE);
        assert_eq!(normalize_body_font_size(99), MAX_BODY_FONT_SIZE);
    }
}
