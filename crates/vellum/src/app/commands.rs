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
}
