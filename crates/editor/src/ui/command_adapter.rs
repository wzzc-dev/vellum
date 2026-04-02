use std::ops::Range;

use crate::core::controller::EditCommand;

pub(crate) struct EditorCommandAdapter;

impl EditorCommandAdapter {
    pub(crate) fn begin_block_edit(block_ix: usize, cursor_offset: Option<usize>) -> EditCommand {
        EditCommand::ActivateBlock {
            index: block_ix,
            cursor_offset,
        }
    }

    pub(crate) fn sync_active_text(text: String, cursor_offset: usize) -> EditCommand {
        EditCommand::ReplaceActiveBlock { text, cursor_offset }
    }

    pub(crate) fn wrap_selection_with_markup(
        selection: Option<Range<usize>>,
        cursor_offset: usize,
        before: String,
        after: String,
        placeholder: String,
    ) -> EditCommand {
        EditCommand::WrapActiveSelection {
            selection,
            cursor_offset,
            before,
            after,
            placeholder,
        }
    }

    pub(crate) fn reshape_active_block(deepen: bool) -> EditCommand {
        EditCommand::AdjustActiveBlock { deepen }
    }

    pub(crate) fn move_to_adjacent_block(
        direction: isize,
        preferred_column: Option<usize>,
    ) -> EditCommand {
        EditCommand::FocusAdjacentBlock {
            direction,
            preferred_column,
        }
    }

    pub(crate) fn stop_block_edit() -> EditCommand {
        EditCommand::ExitEditMode
    }

    pub(crate) fn undo_last_edit() -> EditCommand {
        EditCommand::Undo
    }

    pub(crate) fn redo_last_edit() -> EditCommand {
        EditCommand::Redo
    }

    pub(crate) fn reload_conflicted_document() -> EditCommand {
        EditCommand::ReloadConflict
    }

    pub(crate) fn keep_conflicted_document() -> EditCommand {
        EditCommand::KeepCurrentConflict
    }
}
