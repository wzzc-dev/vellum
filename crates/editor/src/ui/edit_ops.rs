use std::ops::Range;

use gpui::{ClickEvent, Context, Entity, KeyDownEvent, VisualContext, Window};
use gpui_component::input::InputState;

use crate::core::text_ops::supports_semantic_enter;

use super::{
    command_adapter::EditorCommandAdapter, component_ui::InputEvent, view::MarkdownEditor,
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
                self.handle_active_input_text_change("change", None, window, cx);
            }
            InputEvent::Blur | InputEvent::Focus => {
                debug_enter("focus-or-blur clears pending enter state");
                self.interaction.clear_pending_enter_intent();
                self.flush_pending_enter_change(None, window, cx);
            }
            InputEvent::PressEnter { secondary } => {
                debug_enter(format!("press-enter secondary={secondary}"));
                self.interaction.clear_pending_enter_intent();
                if self.flush_pending_enter_change(Some(*secondary), window, cx) {
                    return;
                }
            }
        }
    }

    pub(super) fn handle_observed_input_state_change(
        &mut self,
        observed_input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_active_input_text_change("observe", Some(observed_input), window, cx);
    }

    fn handle_active_input_text_change(
        &mut self,
        source: &str,
        observed_input: Option<&Entity<InputState>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session) = self.interaction.active_session().cloned() else {
            return false;
        };
        if let Some(observed_input) = observed_input
            && !session.input.is_entity(observed_input)
        {
            return false;
        }
        if session.input.has_marked_text(window, cx) {
            debug_enter(format!("{source} ignored while marked text is active"));
            return false;
        }

        let Some(block) = self.snapshot.block_by_id(session.block_id) else {
            return false;
        };
        let (text, cursor_offset) = session.input.text_and_cursor(cx);
        if block.text == text {
            debug_enter(format!(
                "{source} ignored unchanged text block_id={} cursor={}",
                session.block_id, cursor_offset
            ));
            return false;
        }

        debug_enter(format!(
            "{source} block_id={} cursor={} text={:?}",
            session.block_id, cursor_offset, text
        ));
        if self.stage_pending_enter_change(
            session.block_id,
            text.clone(),
            cursor_offset,
            window,
            cx,
        ) {
            return true;
        }

        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::sync_active_text(text, cursor_offset));
        if !effects.changed && !effects.active_block_changed {
            debug_enter(format!("{source} produced no effects"));
            return false;
        }

        self.apply_effects(window, cx, effects);
        self.schedule_autosave(window, cx);
        true
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
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::begin_block_edit(
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
        let cursor_offset = self.snapshot.blocks.get(block_ix).and_then(|block| {
            self.interaction
                .cursor_offset_for_click(block, event, window)
        });

        self.activate_block(block_ix, cursor_offset, window, cx);
    }

    pub(crate) fn focus_adjacent_block(
        &mut self,
        direction: isize,
        preferred_column: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::move_to_adjacent_block(
                direction,
                preferred_column,
            ));
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::stop_block_edit());
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::undo_last_edit());
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::redo_last_edit());
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

        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::wrap_selection_with_markup(
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

    pub(super) fn reload_conflict_from_disk(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::reload_conflicted_document());
        self.apply_effects(window, cx, effects);
    }

    pub(super) fn keep_current_conflicted_version(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::keep_conflicted_document());
        self.apply_effects(window, cx, effects);
    }

    pub(super) fn handle_active_navigation_action(
        &mut self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(session) = self.interaction.active_session() else {
            return false;
        };
        if !session.input.contains_focus(window, cx) {
            return false;
        }

        let Some((direction, column)) = self
            .interaction
            .navigation_target_for_direction(direction, window, cx)
        else {
            return false;
        };

        self.focus_adjacent_block(direction, Some(column), window, cx);
        true
    }

    pub(super) fn handle_active_semantic_enter_action(
        &mut self,
        secondary: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if secondary {
            return false;
        }

        let Some(session) = self.interaction.active_session() else {
            return false;
        };
        if !session.input.contains_focus(window, cx) {
            return false;
        }

        let Some(block) = self.snapshot.block_by_id(session.block_id) else {
            return false;
        };
        if !supports_semantic_enter(&block.kind) {
            return false;
        }

        let (selection, cursor_offset) = session.input.selection_and_cursor(window, cx);
        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::semantic_enter(
                selection,
                cursor_offset,
            ));
        if !effects.changed && !effects.active_block_changed {
            return false;
        }

        self.interaction.clear_pending_enter_intent();
        self.interaction.clear_pending_enter_change();
        self.apply_effects(window, cx, effects);
        self.schedule_autosave(window, cx);
        true
    }

    pub(super) fn record_active_enter_keydown(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        let is_enter = event.keystroke.key == "enter";
        let has_non_shift_modifier =
            modifiers.control || modifiers.alt || modifiers.platform || modifiers.function;
        debug_enter(format!(
            "keydown key={} shift={} ctrl={} alt={} cmd={} fn={}",
            event.keystroke.key,
            modifiers.shift,
            modifiers.control,
            modifiers.alt,
            modifiers.platform,
            modifiers.function
        ));

        if !is_enter || has_non_shift_modifier || modifiers.shift {
            self.interaction.clear_pending_enter_intent();
            return;
        }

        let Some(session) = self.interaction.active_session() else {
            self.interaction.clear_pending_enter_intent();
            return;
        };
        let block_id = session.block_id;
        if !session.input.contains_focus(window, cx) {
            self.interaction.clear_pending_enter_intent();
            return;
        }

        self.interaction.set_pending_enter_intent(block_id, false);
    }

    fn semantic_enter_effects_from_input_change(
        &mut self,
        block_id: u64,
        new_text: &str,
    ) -> Option<crate::core::controller::EditorEffects> {
        let block = self.snapshot.block_by_id(block_id)?;
        if !supports_semantic_enter(&block.kind) {
            return None;
        }

        let (selection, semantic_cursor_offset) =
            semantic_enter_command_from_change(&block.text, new_text)?;

        let effects = self
            .controller
            .dispatch(EditorCommandAdapter::semantic_enter(
                selection,
                semantic_cursor_offset,
            ));
        (effects.changed || effects.active_block_changed).then_some(effects)
    }

    fn stage_pending_enter_change(
        &mut self,
        block_id: u64,
        text: String,
        cursor_offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(block) = self.snapshot.block_by_id(block_id) else {
            self.interaction.clear_pending_enter_change();
            return false;
        };
        if !supports_semantic_enter(&block.kind) {
            debug_enter(format!(
                "stage-enter-change ignored unsupported block {:?}",
                block.kind
            ));
            self.interaction.clear_pending_enter_change();
            return false;
        }
        if semantic_enter_command_from_change(&block.text, &text).is_none() {
            debug_enter("stage-enter-change did not match semantic newline insertion");
            self.interaction.clear_pending_enter_change();
            return false;
        }

        debug_enter(format!(
            "stage-enter-change matched block_id={} cursor={}",
            block_id, cursor_offset
        ));
        self.interaction
            .set_pending_enter_change(block_id, text, cursor_offset);
        let view = cx.entity();
        window.on_next_frame(move |window, cx| {
            let _ = view.update(cx, |this, cx| {
                this.flush_pending_enter_change(None, window, cx);
            });
        });
        true
    }

    fn flush_pending_enter_change(
        &mut self,
        secondary: Option<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.interaction.take_pending_enter_change() else {
            debug_enter("flush-enter-change no pending change");
            return false;
        };

        let effects = if secondary != Some(true) {
            debug_enter(format!(
                "flush-enter-change semantic block_id={} cursor={} secondary={secondary:?}",
                pending.block_id, pending.cursor_offset
            ));
            self.semantic_enter_effects_from_input_change(pending.block_id, &pending.text)
                .unwrap_or_else(|| {
                    debug_enter("flush-enter-change semantic match failed, falling back to sync");
                    self.controller
                        .dispatch(EditorCommandAdapter::sync_active_text(
                            pending.text.clone(),
                            pending.cursor_offset,
                        ))
                })
        } else {
            debug_enter(format!(
                "flush-enter-change plain-sync secondary={secondary:?} block_id={}",
                pending.block_id
            ));
            self.controller
                .dispatch(EditorCommandAdapter::sync_active_text(
                    pending.text,
                    pending.cursor_offset,
                ))
        };

        if !effects.changed && !effects.active_block_changed {
            debug_enter("flush-enter-change produced no effects");
            return false;
        }

        debug_enter(format!(
            "flush-enter-change applied changed={} active_block_changed={}",
            effects.changed, effects.active_block_changed
        ));
        self.apply_effects(window, cx, effects);
        self.schedule_autosave(window, cx);
        true
    }
}

fn semantic_enter_command_from_change(
    old_text: &str,
    new_text: &str,
) -> Option<(Option<Range<usize>>, usize)> {
    let prefix_len = common_prefix_len(old_text.as_bytes(), new_text.as_bytes());
    let suffix_len = common_suffix_len(
        old_text[prefix_len..].as_bytes(),
        new_text[prefix_len..].as_bytes(),
    );

    if prefix_len + suffix_len > old_text.len() {
        return None;
    }
    if prefix_len + suffix_len + 1 != new_text.len() {
        return None;
    }
    let inserted_end = new_text.len().saturating_sub(suffix_len);
    if new_text.get(prefix_len..inserted_end) != Some("\n") {
        return None;
    }

    let selection = prefix_len..old_text.len().saturating_sub(suffix_len);
    let semantic_cursor_offset = selection.end;
    Some((
        (!selection.is_empty()).then_some(selection),
        semantic_cursor_offset,
    ))
}

fn common_prefix_len(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_suffix_len(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

fn debug_enter(message: impl AsRef<str>) {
    if std::env::var_os("VELLUM_DEBUG_ENTER").is_some() {
        eprintln!("[vellum-enter] {}", message.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_enter_change_detects_collapsed_enter() {
        assert_eq!(
            semantic_enter_command_from_change("alpha", "alpha\n"),
            Some((None, 5))
        );
    }

    #[test]
    fn semantic_enter_change_detects_collapsed_enter_when_cursor_does_not_advance() {
        assert_eq!(
            semantic_enter_command_from_change("alpha", "alpha\n"),
            Some((None, 5))
        );
    }

    #[test]
    fn semantic_enter_change_detects_selection_replacement() {
        assert_eq!(
            semantic_enter_command_from_change("alpha beta", "al\neta"),
            Some((Some(2..7), 7))
        );
    }

    #[test]
    fn semantic_enter_change_ignores_non_enter_edits() {
        assert_eq!(semantic_enter_command_from_change("alpha", "alphax"), None);
    }
}
