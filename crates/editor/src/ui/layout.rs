use std::cmp;

use crate::core::document::BlockKind;

use super::{BODY_FONT_SIZE, BODY_LINE_HEIGHT, CODE_FONT_SIZE, CODE_LINE_HEIGHT};

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
