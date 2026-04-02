use gpui::{ClickEvent, Context, KeyDownEvent, VisualContext, Window};

use super::{
    command_adapter::EditorCommandAdapter,
    component_ui::InputEvent,
    view::MarkdownEditor,
};

impl MarkdownEditor {
    pub(crate) fn handle_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let Some(session) = self.interaction.active_session() else {
                    return;
                };
                let (text, cursor_offset) = session.input.text_and_cursor(cx);
                let effects = self.controller.dispatch(
                    EditorCommandAdapter::sync_active_text(text, cursor_offset),
                );
                self.apply_effects(window, cx, effects);
                self.schedule_autosave(window, cx);
            }
            InputEvent::Blur | InputEvent::Focus | InputEvent::PressEnter { .. } => {}
        }
    }

    fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let token = self.interaction.next_autosave_token();
        let view = cx.entity();
        let autosave_delay = self.controller.autosave_delay();
        window
            .spawn(cx, async move |cx| {
                gpui::Timer::after(autosave_delay).await;
                let _ = cx.update_window_entity(&view, |this, window, cx| {
                    if this.interaction.autosave_generation() == token && this.snapshot.dirty {
                        let _ = this.save(window, cx);
                    }
                });
            })
            .detach();
    }

    fn activate_block(
        &mut self,
        block_ix: usize,
        cursor_offset: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.dispatch(EditorCommandAdapter::begin_block_edit(
            block_ix,
            cursor_offset,
        ));
        self.apply_effects(window, cx, effects);
    }

    pub(super) fn activate_block_from_click(
        &mut self,
        block_ix: usize,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cursor_offset = self
            .snapshot
            .blocks
            .get(block_ix)
            .and_then(|block| self.interaction.cursor_offset_for_click(block, event, window));

        self.activate_block(block_ix, cursor_offset, window, cx);
    }

    pub(crate) fn focus_adjacent_block(
        &mut self,
        direction: isize,
        preferred_column: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.dispatch(EditorCommandAdapter::move_to_adjacent_block(
            direction,
            preferred_column,
        ));
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditorCommandAdapter::stop_block_edit());
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditorCommandAdapter::undo_last_edit());
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditorCommandAdapter::redo_last_edit());
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn apply_markup(
        &mut self,
        before: &str,
        after: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.interaction.active_session() else {
            return;
        };

        let (selection, cursor_offset) = session.input.selection_and_cursor(window, cx);

        let effects = self.controller.dispatch(EditorCommandAdapter::wrap_selection_with_markup(
            selection,
            cursor_offset,
            before.to_string(),
            after.to_string(),
            placeholder.to_string(),
        ));
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn adjust_current_block(
        &mut self,
        deepen: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::reshape_active_block(deepen));
        self.apply_effects(window, cx, effects);
    }

    pub(super) fn reload_conflict_from_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::reload_conflicted_document());
        self.apply_effects(window, cx, effects);
    }

    pub(super) fn keep_current_conflicted_version(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::keep_conflicted_document());
        self.apply_effects(window, cx, effects);
    }

    pub(super) fn handle_active_navigation_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((direction, column)) = self.interaction.navigation_target(event, window, cx)
        else {
            return false;
        };

        self.focus_adjacent_block(direction, Some(column), window, cx);
        true
    }
}
