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
    ]
);

pub use core::{
    BlockKind, BlockProjection, BlockSnapshot, BlockSpan, ConflictState, CursorAnchorPolicy,
    DocumentBuffer, DocumentSource, DocumentState, EditCommand, EditorController, EditorEffects,
    EditorSnapshot, FileSyncEvent, SelectionState, SyncPolicy, SyncState, Transaction,
};
pub use ui::{EditorEvent, MarkdownEditor, bind_keys};
