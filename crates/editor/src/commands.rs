use crate::{
    App, BoldSelection, Context, DemoteBlock, ExitBlockEdit, FocusNextBlock, FocusPrevBlock,
    INPUT_CONTEXT, ItalicSelection, KeyBinding, LinkSelection, MarkdownEditor, PromoteBlock,
    RedoEdit, UndoEdit, Window,
};

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-b", BoldSelection, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-i", ItalicSelection, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-k", LinkSelection, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-[", PromoteBlock, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-]", DemoteBlock, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("escape", ExitBlockEdit, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-z", UndoEdit, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-shift-z", RedoEdit, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-y", RedoEdit, Some(crate::EDITOR_CONTEXT)),
        KeyBinding::new("ctrl-b", BoldSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-i", ItalicSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-k", LinkSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-[", PromoteBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-]", DemoteBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-z", UndoEdit, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-shift-z", RedoEdit, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-y", RedoEdit, Some(INPUT_CONTEXT)),
    ]);
}

impl MarkdownEditor {
    pub(crate) fn on_bold_selection(
        &mut self,
        _: &BoldSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("**", "**", "bold text", window, cx);
    }

    pub(crate) fn on_italic_selection(
        &mut self,
        _: &ItalicSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("*", "*", "italic text", window, cx);
    }

    pub(crate) fn on_link_selection(
        &mut self,
        _: &LinkSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("[", "](https://)", "link text", window, cx);
    }

    pub(crate) fn on_promote_block(
        &mut self,
        _: &PromoteBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_current_block(false, window, cx);
    }

    pub(crate) fn on_demote_block(
        &mut self,
        _: &DemoteBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_current_block(true, window, cx);
    }

    pub(crate) fn on_exit_block_edit(
        &mut self,
        _: &ExitBlockEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.exit_edit_mode(window, cx);
    }

    pub(crate) fn on_focus_prev_block(
        &mut self,
        _: &FocusPrevBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(-1, window, cx);
    }

    pub(crate) fn on_focus_next_block(
        &mut self,
        _: &FocusNextBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(1, window, cx);
    }

    pub(crate) fn on_undo_edit(
        &mut self,
        _: &UndoEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.undo(window, cx);
    }

    pub(crate) fn on_redo_edit(
        &mut self,
        _: &RedoEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.redo(window, cx);
    }
}
