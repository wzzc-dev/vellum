use super::*;

impl VellumApp {
    pub(super) fn on_open_file(
        &mut self,
        _: &OpenFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_file_dialog(window, cx);
    }

    pub(super) fn on_open_folder(
        &mut self,
        _: &OpenFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_folder_dialog(window, cx);
    }

    pub(super) fn on_new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_file(window, cx);
    }

    pub(super) fn on_save_now(&mut self, _: &SaveNow, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document(window, cx) {
            self.set_status(format!("Save failed: {err}"));
        }
    }

    pub(super) fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document_as(window, cx) {
            self.set_status(format!("Save As failed: {err}"));
        }
    }

    pub(super) fn on_toggle_sidebar(
        &mut self,
        _: &ToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar_visibility(cx);
    }

    pub(super) fn on_bold_selection(
        &mut self,
        _: &BoldSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("**", "**", "bold text", window, cx);
    }

    pub(super) fn on_italic_selection(
        &mut self,
        _: &ItalicSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("*", "*", "italic text", window, cx);
    }

    pub(super) fn on_link_selection(
        &mut self,
        _: &LinkSelection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_markup("[", "](https://)", "link text", window, cx);
    }

    pub(super) fn on_promote_block(
        &mut self,
        _: &PromoteBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_current_block(false, window, cx);
    }

    pub(super) fn on_demote_block(
        &mut self,
        _: &DemoteBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.adjust_current_block(true, window, cx);
    }

    pub(super) fn on_exit_block_edit(
        &mut self,
        _: &ExitBlockEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.exit_edit_mode(window, cx);
    }

    pub(super) fn on_focus_prev_block(
        &mut self,
        _: &FocusPrevBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(-1, window, cx);
    }

    pub(super) fn on_focus_next_block(
        &mut self,
        _: &FocusNextBlock,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_adjacent_block(1, window, cx);
    }
}
