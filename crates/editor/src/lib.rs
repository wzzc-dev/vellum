use gpui::actions;

mod core;
mod ui;

actions!(
    vellum_editor,
    [
        BoldSelection,
        ItalicSelection,
        LinkSelection,
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
        InsertHorizontalRule,
        InsertCodeFence,
        InsertTable,
        ToggleTypewriterMode,
    ]
);

pub use core::{
    BlockKind, BlockProjection, BlockSnapshot, BlockSpan, CaretPosition, ConflictState,
    CursorAnchorPolicy, DisplayMap, DocumentBuffer, DocumentSource, DocumentState, EditCommand,
    EditorController, EditorEffects, EditorSnapshot, EditorViewMode, EmbeddedNodeKind,
    FileSyncEvent, HiddenSyntaxPolicy, HitTestResult, OutlineItem, RenderBlock, RenderInlineStyle,
    RenderSpan, RenderSpanKind, RenderSpanMeta, SelectionAffinity, SelectionModel, SelectionState,
    SyncPolicy, SyncState, Transaction,
};
pub use ui::{EditorEvent, MarkdownEditor, bind_keys};
