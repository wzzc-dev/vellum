use crate::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, BlockKind, CODE_FONT_SIZE, CODE_LINE_HEIGHT, Input,
    TextViewStyle, cmp, px, rems,
};
use gpui::Styled;

#[derive(Clone, Copy)]
pub(crate) struct BlockLayoutMetrics {
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) row_spacing_y: f32,
    pub(crate) block_padding_y: f32,
    pub(crate) extra_height: f32,
}

pub(crate) fn position_for_byte_offset(text: &str, byte_offset: usize) -> (usize, usize) {
    let clamped = cmp::min(byte_offset, text.len());
    let prefix = &text[..clamped];
    let row = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let col = prefix
        .rsplit_once('\n')
        .map(|(_, tail)| tail.chars().count())
        .unwrap_or_else(|| prefix.chars().count());
    (row, col)
}

pub(crate) fn activation_cursor_offset(text: &str) -> usize {
    text.trim_end_matches(['\r', '\n']).len()
}

pub(crate) fn style_active_input_for_block(input: Input, kind: &BlockKind) -> Input {
    let metrics = block_layout_metrics(kind);
    input
        .text_size(px(metrics.font_size))
        .line_height(px(metrics.line_height))
}

pub(crate) fn block_layout_metrics(kind: &BlockKind) -> BlockLayoutMetrics {
    match kind {
        BlockKind::Heading { depth: 1 } => BlockLayoutMetrics {
            font_size: 34.,
            line_height: 42.,
            row_spacing_y: 8.,
            block_padding_y: 6.,
            extra_height: 6.,
        },
        BlockKind::Heading { depth: 2 } => BlockLayoutMetrics {
            font_size: 28.,
            line_height: 36.,
            row_spacing_y: 7.,
            block_padding_y: 5.,
            extra_height: 4.,
        },
        BlockKind::Heading { depth: 3 } => BlockLayoutMetrics {
            font_size: 24.,
            line_height: 32.,
            row_spacing_y: 6.,
            block_padding_y: 4.,
            extra_height: 4.,
        },
        BlockKind::Heading { depth: 4 } => BlockLayoutMetrics {
            font_size: 20.,
            line_height: 28.,
            row_spacing_y: 5.,
            block_padding_y: 4.,
            extra_height: 2.,
        },
        BlockKind::Heading { .. } => BlockLayoutMetrics {
            font_size: 18.,
            line_height: 26.,
            row_spacing_y: 5.,
            block_padding_y: 4.,
            extra_height: 2.,
        },
        BlockKind::CodeFence { .. } => BlockLayoutMetrics {
            font_size: CODE_FONT_SIZE,
            line_height: CODE_LINE_HEIGHT,
            row_spacing_y: 6.,
            block_padding_y: 6.,
            extra_height: 10.,
        },
        BlockKind::Table => BlockLayoutMetrics {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 6.,
            block_padding_y: 5.,
            extra_height: 12.,
        },
        BlockKind::ThematicBreak => BlockLayoutMetrics {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 8.,
            block_padding_y: 6.,
            extra_height: 18.,
        },
        _ => BlockLayoutMetrics {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 4.,
            block_padding_y: 3.,
            extra_height: 2.,
        },
    }
}

pub(crate) fn markdown_preview_style() -> TextViewStyle {
    TextViewStyle::default()
        .paragraph_gap(rems(0.45))
        .heading_font_size(|level, _| match level {
            1 => px(34.),
            2 => px(28.),
            3 => px(24.),
            4 => px(20.),
            5 => px(18.),
            _ => px(BODY_FONT_SIZE),
        })
}

pub(crate) fn count_document_words(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_word = false;

    for ch in text.chars() {
        if is_cjk_character(ch) {
            if in_word {
                count += 1;
                in_word = false;
            }
            count += 1;
        } else if ch.is_alphanumeric() {
            in_word = true;
        } else if in_word {
            count += 1;
            in_word = false;
        }
    }

    if in_word {
        count += 1;
    }

    count
}

pub(crate) fn adjust_block_markup(text: &str, deepen: bool) -> Option<String> {
    let mut lines = text.lines();
    let first = lines.next()?;
    let rest = if text.contains('\n') {
        text[first.len()..].to_string()
    } else {
        String::new()
    };

    let trimmed = first.trim_start();
    let indent = &first[..first.len().saturating_sub(trimmed.len())];

    if let Some(space_ix) = trimmed.find(' ') {
        let marker = &trimmed[..space_ix];
        if marker.chars().all(|ch| ch == '#') && !marker.is_empty() {
            let current = marker.len();
            let updated = if deepen {
                cmp::min(current + 1, 6)
            } else {
                current.saturating_sub(1)
            };
            let head = if updated == 0 {
                format!("{indent}{}", &trimmed[space_ix + 1..])
            } else {
                format!(
                    "{indent}{} {}",
                    "#".repeat(updated),
                    &trimmed[space_ix + 1..]
                )
            };
            return Some(format!("{head}{rest}"));
        }
    }

    let list_markers = ["- ", "* ", "+ ", "- [ ] ", "- [x] ", "* [ ] ", "* [x] "];
    if list_markers
        .iter()
        .any(|marker| trimmed.starts_with(marker))
        || trimmed
            .split_once(". ")
            .map(|(n, _)| n.chars().all(|ch| ch.is_ascii_digit()))
            .unwrap_or(false)
    {
        let updated_indent = if deepen {
            format!("{indent}  ")
        } else if indent.len() >= 2 {
            indent[..indent.len() - 2].to_string()
        } else {
            String::new()
        };

        let updated = text
            .lines()
            .map(|line| format!("{updated_indent}{}", line.trim_start()))
            .collect::<Vec<_>>()
            .join("\n");
        return Some(updated);
    }

    if deepen {
        Some(format!("# {text}"))
    } else {
        None
    }
}

fn is_cjk_character(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0x3040..=0x30FF
            | 0x31F0..=0x31FF
            | 0xAC00..=0xD7AF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_across_cjk_and_ascii() {
        assert_eq!(count_document_words("hello world"), 2);
        assert_eq!(count_document_words("你好 world"), 3);
    }

    #[test]
    fn maps_offsets_to_rows_and_columns() {
        assert_eq!(position_for_byte_offset("abc\ndef", 0), (0, 0));
        assert_eq!(position_for_byte_offset("abc\ndef", 4), (1, 0));
        assert_eq!(position_for_byte_offset("abc\ndef", 7), (1, 3));
    }

    #[test]
    fn adjusts_heading_markup() {
        assert_eq!(
            adjust_block_markup("# Title", true),
            Some("## Title".to_string())
        );
        assert_eq!(
            adjust_block_markup("## Title", false),
            Some("# Title".to_string())
        );
    }

    #[test]
    fn deepens_plain_text_into_heading() {
        assert_eq!(
            adjust_block_markup("Title", true),
            Some("# Title".to_string())
        );
    }
}
