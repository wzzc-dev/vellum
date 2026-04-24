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
