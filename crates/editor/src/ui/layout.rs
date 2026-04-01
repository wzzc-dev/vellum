use std::cmp;

use gpui::{Bounds, Pixels, Point, Window, point, px};

use crate::core::document::BlockKind;

use super::{BODY_FONT_SIZE, BODY_LINE_HEIGHT, CODE_FONT_SIZE, CODE_LINE_HEIGHT};

#[derive(Clone, Copy)]
pub(crate) struct BlockLayoutMetrics {
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) row_spacing_y: f32,
    pub(crate) block_padding_y: f32,
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

pub(crate) fn byte_offset_for_click_position(
    kind: &BlockKind,
    text: &str,
    click_position: Point<Pixels>,
    bounds: Bounds<Pixels>,
    window: &Window,
) -> usize {
    if text.is_empty() {
        return 0;
    }

    if bounds.size.width <= px(0.) {
        return text.len();
    }

    let metrics = block_layout_metrics(kind);
    let line_height = px(metrics.line_height);
    let font_size = px(metrics.font_size);
    let mut local = click_position - bounds.origin;
    local.x = local.x.max(px(0.));
    local.y = local.y.max(px(0.));

    let run = window.text_style().clone().to_run(text.len());
    let Ok(lines) = window.text_system().shape_text(
        text.to_string().into(),
        font_size,
        &[run],
        Some(bounds.size.width),
        None,
    ) else {
        return text.len();
    };

    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);
    for (line_ix, line) in lines.iter().enumerate() {
        let line_height_span = line.size(line_height).height;
        if local.y <= y_offset + line_height_span {
            let position = point(local.x, (local.y - y_offset).max(px(0.)));
            let local_offset = match line.closest_index_for_position(position, line_height) {
                Ok(offset) | Err(offset) => offset,
            };
            return (byte_offset + local_offset).min(text.len());
        }

        y_offset += line_height_span;
        byte_offset += line.len();
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    text.len()
}

pub(crate) fn block_layout_metrics(kind: &BlockKind) -> BlockLayoutMetrics {
    match kind {
        BlockKind::Heading { depth: 1 } => BlockLayoutMetrics {
            font_size: 34.,
            line_height: 42.,
            row_spacing_y: 8.,
            block_padding_y: 6.,
        },
        BlockKind::Heading { depth: 2 } => BlockLayoutMetrics {
            font_size: 28.,
            line_height: 36.,
            row_spacing_y: 7.,
            block_padding_y: 5.,
        },
        BlockKind::Heading { depth: 3 } => BlockLayoutMetrics {
            font_size: 24.,
            line_height: 32.,
            row_spacing_y: 6.,
            block_padding_y: 4.,
        },
        BlockKind::Heading { depth: 4 } => BlockLayoutMetrics {
            font_size: 20.,
            line_height: 28.,
            row_spacing_y: 5.,
            block_padding_y: 4.,
        },
        BlockKind::Heading { .. } => BlockLayoutMetrics {
            font_size: 18.,
            line_height: 26.,
            row_spacing_y: 5.,
            block_padding_y: 4.,
        },
        BlockKind::CodeFence { .. } => BlockLayoutMetrics {
            font_size: CODE_FONT_SIZE,
            line_height: CODE_LINE_HEIGHT,
            row_spacing_y: 6.,
            block_padding_y: 6.,
        },
        BlockKind::Table => BlockLayoutMetrics {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 6.,
            block_padding_y: 5.,
        },
        BlockKind::ThematicBreak => BlockLayoutMetrics {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 8.,
            block_padding_y: 6.,
        },
        _ => BlockLayoutMetrics {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 4.,
            block_padding_y: 3.,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_offsets_to_rows_and_columns() {
        assert_eq!(position_for_byte_offset("abc\ndef", 0), (0, 0));
        assert_eq!(position_for_byte_offset("abc\ndef", 4), (1, 0));
        assert_eq!(position_for_byte_offset("abc\ndef", 7), (1, 3));
    }
}
