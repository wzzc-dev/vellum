use std::cmp;

use gpui::{Bounds, Pixels, Point, Window, point, px};

use crate::core::document::BlockKind;

use super::{BODY_FONT_SIZE, BODY_LINE_HEIGHT, CODE_FONT_SIZE, CODE_LINE_HEIGHT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditableSurfaceKind {
    AutoGrowText,
    CodeEditor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlockPresentation {
    pub(crate) font_size: f32,
    pub(crate) line_height: f32,
    pub(crate) row_spacing_y: f32,
    pub(crate) block_padding_y: f32,
    pub(crate) preview_paragraph_gap_rem: f32,
    pub(crate) surface_kind: EditableSurfaceKind,
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

    let presentation = block_presentation(kind);
    let line_height = px(presentation.line_height);
    let font_size = px(presentation.font_size);
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

pub(crate) fn block_presentation(kind: &BlockKind) -> BlockPresentation {
    match kind {
        BlockKind::Heading { depth: 1 } => BlockPresentation {
            font_size: 34.,
            line_height: 42.,
            row_spacing_y: 8.,
            block_padding_y: 6.,
            preview_paragraph_gap_rem: 0.32,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        BlockKind::Heading { depth: 2 } => BlockPresentation {
            font_size: 28.,
            line_height: 36.,
            row_spacing_y: 7.,
            block_padding_y: 5.,
            preview_paragraph_gap_rem: 0.3,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        BlockKind::Heading { depth: 3 } => BlockPresentation {
            font_size: 24.,
            line_height: 32.,
            row_spacing_y: 6.,
            block_padding_y: 4.,
            preview_paragraph_gap_rem: 0.28,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        BlockKind::Heading { depth: 4 } => BlockPresentation {
            font_size: 20.,
            line_height: 28.,
            row_spacing_y: 5.,
            block_padding_y: 4.,
            preview_paragraph_gap_rem: 0.26,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        BlockKind::Heading { .. } => BlockPresentation {
            font_size: 18.,
            line_height: 26.,
            row_spacing_y: 5.,
            block_padding_y: 4.,
            preview_paragraph_gap_rem: 0.24,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        BlockKind::CodeFence { .. } => BlockPresentation {
            font_size: CODE_FONT_SIZE,
            line_height: CODE_LINE_HEIGHT,
            row_spacing_y: 6.,
            block_padding_y: 6.,
            preview_paragraph_gap_rem: 0.35,
            surface_kind: EditableSurfaceKind::CodeEditor,
        },
        BlockKind::Table => BlockPresentation {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 6.,
            block_padding_y: 5.,
            preview_paragraph_gap_rem: 0.35,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        BlockKind::ThematicBreak => BlockPresentation {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 8.,
            block_padding_y: 6.,
            preview_paragraph_gap_rem: 0.4,
            surface_kind: EditableSurfaceKind::AutoGrowText,
        },
        _ => BlockPresentation {
            font_size: BODY_FONT_SIZE,
            line_height: BODY_LINE_HEIGHT,
            row_spacing_y: 4.,
            block_padding_y: 3.,
            preview_paragraph_gap_rem: 0.35,
            surface_kind: EditableSurfaceKind::AutoGrowText,
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

    #[test]
    fn body_like_blocks_use_auto_grow_surface() {
        let kinds = [
            BlockKind::Raw,
            BlockKind::Paragraph,
            BlockKind::Heading { depth: 1 },
            BlockKind::Blockquote,
            BlockKind::List,
            BlockKind::Table,
            BlockKind::ThematicBreak,
            BlockKind::Html,
            BlockKind::Footnote,
            BlockKind::Unknown,
        ];

        for kind in kinds {
            assert_eq!(
                block_presentation(&kind).surface_kind,
                EditableSurfaceKind::AutoGrowText
            );
        }
    }

    #[test]
    fn code_fence_uses_code_editor_surface() {
        assert_eq!(
            block_presentation(&BlockKind::CodeFence {
                language: Some("rust".to_string()),
            })
            .surface_kind,
            EditableSurfaceKind::CodeEditor
        );
    }

    #[test]
    fn heading_presentation_preserves_typography_scale() {
        let heading = block_presentation(&BlockKind::Heading { depth: 1 });
        let paragraph = block_presentation(&BlockKind::Paragraph);

        assert_eq!(heading.font_size, 34.);
        assert_eq!(heading.line_height, 42.);
        assert!(heading.preview_paragraph_gap_rem < paragraph.preview_paragraph_gap_rem);
    }

    #[test]
    fn table_presentation_reuses_body_typography() {
        let table = block_presentation(&BlockKind::Table);
        let paragraph = block_presentation(&BlockKind::Paragraph);

        assert_eq!(table.font_size, paragraph.font_size);
        assert_eq!(table.line_height, paragraph.line_height);
        assert!(table.block_padding_y > paragraph.block_padding_y);
    }
}
