use gpui::actions;

mod core;
mod ui;

actions!(
    vellum_editor,
    [
        BoldSelection,
        ItalicSelection,
        LinkSelection,
        ToggleInlineCode,
        ToggleStrikethrough,
        ToggleHighlight,
        ToggleSuperscript,
        ToggleSubscript,
        PromoteBlock,
        DemoteBlock,
        ExitBlockEdit,
        FocusPrevBlock,
        FocusNextBlock,
        ToggleSourceMode,
        UndoEdit,
        RedoEdit,
        SecondaryEnter,
        ToggleHeading1,
        ToggleHeading2,
        ToggleHeading3,
        ToggleHeading4,
        ToggleHeading5,
        ToggleHeading6,
        ToggleParagraph,
        ToggleBlockquote,
        ToggleBulletList,
        ToggleOrderedList,
        ToggleTaskList,
        InsertHorizontalRule,
        InsertCodeFence,
        InsertMermaidDiagram,
        InsertTable,
        InsertTableRow,
        DeleteTableRow,
        InsertTableColumn,
        DeleteTableColumn,
        AlignTableColumnLeft,
        AlignTableColumnCenter,
        AlignTableColumnRight,
        InsertInlineMath,
        InsertMathBlock,
        InsertHtmlBlock,
        InsertImage,
        InsertCallout,
        InsertToc,
        InsertFootnote,
        InsertFrontMatter,
        ToggleTypewriterMode,
        ToggleFocusHighlightMode,
        GotoLine,
    ]
);

pub use core::{
    BlockKind, BlockProjection, BlockSnapshot, BlockSpan, CaretPosition, ConflictState,
    CursorAnchorPolicy, DisplayMap, DocumentBuffer, DocumentSource, DocumentState, EditCommand,
    EditorController, EditorEffects, EditorSnapshot, EditorViewMode, EmbeddedNodeKind,
    FileSyncEvent, HiddenSyntaxPolicy, HitTestResult, OutlineItem, RenderBlock, RenderInlineStyle,
    RenderSpan, RenderSpanKind, RenderSpanMeta, SelectionAffinity, SelectionModel, SelectionState,
    SyncPolicy, SyncState, Transaction, math_source_to_display_text,
};
pub use ui::{EditorEvent, MarkdownEditor, bind_keys};
pub use ui::{
    DEFAULT_BODY_FONT_SIZE, MAX_BODY_FONT_SIZE, MIN_BODY_FONT_SIZE, normalize_body_font_size,
    set_body_font_size,
    theme::{SyntaxTheme, get_syntax_theme, set_syntax_theme},
};
