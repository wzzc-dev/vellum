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
        UndoEdit,
        RedoEdit,
        SecondaryEnter,
    ]
);

pub use core::{
    BlockKind, BlockProjection, BlockSnapshot, BlockSpan, CaretPosition, ConflictState,
    CursorAnchorPolicy, DisplayMap, DocumentBuffer, DocumentSource, DocumentState, EditCommand,
    EditorController, EditorEffects, EditorSnapshot, EmbeddedNodeKind, FileSyncEvent,
    HiddenSyntaxPolicy, HitTestResult, RenderBlock, RenderInlineStyle, RenderSpan, RenderSpanKind,
    SelectionAffinity, SelectionModel, SelectionState, SyncPolicy, SyncState, Transaction,
};
pub use ui::{EditorEvent, MarkdownEditor, bind_keys};
