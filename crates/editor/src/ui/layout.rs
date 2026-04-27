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
        BlockKind::SourceCode => BlockPresentation {
            font_size: CODE_FONT_SIZE,
            line_height: CODE_LINE_HEIGHT,
            row_spacing_y: 2.,
            block_padding_y: 2.,
            preview_paragraph_gap_rem: 0.,
            surface_kind: EditableSurfaceKind::CodeEditor,
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
    fn source_code_uses_code_editor_surface() {
        assert_eq!(
            block_presentation(&BlockKind::SourceCode).surface_kind,
            EditableSurfaceKind::CodeEditor
        );
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
