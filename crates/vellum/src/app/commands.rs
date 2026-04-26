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

    pub(super) fn on_open_find_panel(
        &mut self,
        _: &OpenFindPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_panel();
        cx.notify();
    }

    pub(super) fn on_close_find_panel(
        &mut self,
        _: &CloseFindPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_find_panel();
        cx.notify();
    }

    pub(super) fn on_find_next_match(
        &mut self,
        _: &FindNextMatch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_panel();
        if let Some(offset) = self.navigate_find_match(false) {
            self.active_editor_entity().update(cx, |editor, cx| {
                editor.select_source_offset(offset, window, cx);
            });
        }
        cx.notify();
    }

    pub(super) fn on_find_previous_match(
        &mut self,
        _: &FindPreviousMatch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_panel();
        if let Some(offset) = self.navigate_find_match(true) {
            self.active_editor_entity().update(cx, |editor, cx| {
                editor.select_source_offset(offset, window, cx);
            });
        }
        cx.notify();
    }

    pub(super) fn on_open_find_replace_panel(
        &mut self,
        _: &OpenFindReplacePanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_find_replace_panel();
        cx.notify();
    }

    pub(super) fn on_replace_one(
        &mut self,
        _: &ReplaceOne,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_current_match(window, cx);
        cx.notify();
    }

    pub(super) fn on_replace_all(
        &mut self,
        _: &ReplaceAll,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_all_matches(window, cx);
        cx.notify();
    }

    pub(super) fn on_open_folder(
        &mut self,
        _: &OpenFolder,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.request_open_folder(window, cx);
    }

    pub(super) fn on_new_file(&mut self, _: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_file(window, cx);
    }

    pub(super) fn on_save_now(&mut self, _: &SaveNow, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document(window, cx) {
            self.set_status(format!("Save failed: {err}"));
            cx.notify();
        }
    }

    pub(super) fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = self.save_document_as(window, cx) {
            self.set_status(format!("Save As failed: {err}"));
            cx.notify();
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

    pub(super) fn on_toggle_status_bar(
        &mut self,
        _: &ToggleStatusBar,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_status_bar_pinned(!self.status_bar_pinned, window, cx);
    }

    pub(super) fn on_close_tab(
        &mut self,
        _: &CloseTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_active_tab(window, cx);
    }

    pub(super) fn on_previous_tab(
        &mut self,
        _: &PreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.len() > 1 {
            let index = if self.active_tab_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab_index - 1
            };
            self.switch_to_tab(index, window, cx);
        }
    }

    pub(super) fn on_next_tab(
        &mut self,
        _: &NextTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tabs.len() > 1 {
            let index = (self.active_tab_index + 1) % self.tabs.len();
            self.switch_to_tab(index, window, cx);
        }
    }

    pub(super) fn on_manage_plugins(
        &mut self,
        _: &ManagePlugins,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_right_panel(RightPanelView::Plugins, cx);
    }
}
