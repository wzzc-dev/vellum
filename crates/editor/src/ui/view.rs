use gpui::{
    Context, EventEmitter, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
    div, px,
};

use crate::core::controller::{DocumentSource, EditorController, EditorSnapshot, SyncPolicy};

use super::{EDITOR_CONTEXT, interaction::EditorInteractionState};

#[derive(Debug, Clone)]
pub enum EditorEvent {
    Changed(EditorSnapshot),
}

pub struct MarkdownEditor {
    pub(super) controller: EditorController,
    pub(super) snapshot: EditorSnapshot,
    pub(super) interaction: EditorInteractionState,
}

impl EventEmitter<EditorEvent> for MarkdownEditor {}

impl MarkdownEditor {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        let controller = EditorController::new(
            DocumentSource::Empty {
                suggested_path: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        Self {
            controller,
            snapshot,
            interaction: EditorInteractionState::new(),
        }
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        self.snapshot.clone()
    }
}

impl Render for MarkdownEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("markdown-editor")
            .key_context(EDITOR_CONTEXT)
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .on_action(cx.listener(Self::on_bold_selection))
            .on_action(cx.listener(Self::on_italic_selection))
            .on_action(cx.listener(Self::on_link_selection))
            .on_action(cx.listener(Self::on_promote_block))
            .on_action(cx.listener(Self::on_demote_block))
            .on_action(cx.listener(Self::on_exit_block_edit))
            .on_action(cx.listener(Self::on_focus_prev_block))
            .on_action(cx.listener(Self::on_focus_next_block))
            .on_action(cx.listener(Self::on_undo_edit))
            .on_action(cx.listener(Self::on_redo_edit))
            .child(self.render_editor(window, cx))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::PathBuf, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext, VisualContext, VisualTestContext, Window};
    use gpui_component::{Root, input::Position};

    use crate::{EditCommand, ui::component_ui::InputEvent};

    use super::*;

    #[test]
    fn snapshot_is_forwarded_from_controller() {
        let controller = EditorController::new(
            DocumentSource::Text {
                path: Some(PathBuf::from("note.md")),
                suggested_path: Some(PathBuf::from("note.md")),
                text: "hello world".to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        assert_eq!(snapshot.display_name, "note.md");
        assert_eq!(snapshot.word_count, 2);
    }

    #[gpui::test]
    fn bare_down_moves_to_next_block_at_boundary(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\nSecond", 0, Some(0));
        let initial_ids = block_ids(&view, cx);

        cx.simulate_keystrokes("down");

        assert_eq!(active_block_id(&view, cx), Some(initial_ids[1]));
    }

    #[gpui::test]
    fn bare_up_moves_to_previous_block_at_boundary(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\nSecond", 1, Some(0));
        let initial_ids = block_ids(&view, cx);

        cx.simulate_keystrokes("up");

        assert_eq!(active_block_id(&view, cx), Some(initial_ids[0]));
    }

    #[gpui::test]
    fn bare_down_stays_within_multiline_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "one\ntwo\nthree\n\nnext", 0, Some(1));
        let initial_active = active_block_id(&view, cx);

        cx.simulate_keystrokes("down");

        assert_eq!(active_block_id(&view, cx), initial_active);
    }

    #[gpui::test]
    fn bare_enter_splits_paragraph_into_next_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First", 0, Some(5));

        cx.simulate_keystrokes("enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(
            block_texts(&snapshot),
            vec!["First".to_string(), "".to_string()]
        );
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[1].id));
        assert_eq!(snapshot.active_cursor_offset, Some(0));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn bare_enter_between_blocks_focuses_materialized_empty_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\nSecond", 0, Some(5));

        cx.simulate_keystrokes("enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(
            block_texts(&snapshot),
            vec![
                "First".to_string(),
                "".to_string(),
                "Second".to_string()
            ]
        );
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[1].id));
        assert_eq!(snapshot.active_cursor_offset, Some(0));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn bare_backspace_with_text_stays_within_same_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First", 0, Some(5));
        let initial_active = active_block_id(&view, cx);

        cx.simulate_keystrokes("backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.blocks.len(), 1);
        assert_eq!(snapshot.blocks[0].text, "Firs");
        assert_eq!(snapshot.active_block_id, initial_active);
        assert_eq!(snapshot.active_cursor_offset, Some(4));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn bare_backspace_moves_from_trailing_empty_block_to_previous_block_end(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\n", 1, Some(0));

        cx.simulate_keystrokes("backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(block_texts(&snapshot), vec!["First".to_string()]);
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[0].id));
        assert_eq!(snapshot.active_cursor_offset, Some(5));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn bare_backspace_moves_from_materialized_middle_empty_block_to_previous_block_end(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\n\n\nSecond", 1, Some(0));

        cx.simulate_keystrokes("backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(
            block_texts(&snapshot),
            vec!["First".to_string(), "Second".to_string()]
        );
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[0].id));
        assert_eq!(snapshot.active_cursor_offset, Some(5));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn deleting_block_to_empty_then_backspace_crosses_into_previous_block(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\nSecond", 1, Some(6));
        let input = active_input(&view, cx).expect("active input");

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value("", window, cx);
        });
        cx.run_until_parked();
        assert!(active_input_has_focus(&view, cx));

        cx.simulate_keystrokes("backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(block_texts(&snapshot), vec!["First".to_string()]);
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[0].id));
        assert_eq!(snapshot.active_cursor_offset, Some(5));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn shift_enter_keeps_editing_inside_body_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "Alpha beta", 0, Some(5));
        let initial_active = active_block_id(&view, cx);

        cx.simulate_keystrokes("shift-enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.blocks.len(), 1);
        assert_eq!(snapshot.active_block_id, initial_active);
        assert!(snapshot.blocks[0].text.contains('\n'));
    }

    #[gpui::test]
    fn bare_enter_keeps_code_block_in_same_input_surface(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "```rs\nabc\n```", 0, Some(7));
        let initial_active = active_block_id(&view, cx);
        let initial_text = snapshot(&view, cx).blocks[0].text.clone();

        cx.simulate_keystrokes("enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.blocks.len(), 1);
        assert_eq!(snapshot.active_block_id, initial_active);
        assert_ne!(snapshot.blocks[0].text, initial_text);
    }

    #[gpui::test]
    fn change_fallback_splits_paragraph_when_enter_action_is_missed(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First", 0, Some(5));
        let input = active_input(&view, cx).expect("active input");

        cx.update_window_entity(&input, |input, window, cx| {
            input.insert("\n", window, cx);
        });
        cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_input_event(&InputEvent::PressEnter { secondary: false }, window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(
            block_texts(&snapshot),
            vec!["First".to_string(), "".to_string()]
        );
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[1].id));
        assert_eq!(snapshot.active_cursor_offset, Some(0));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn change_fallback_splits_paragraph_when_newline_is_inserted_without_cursor_advancing(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First", 0, Some(5));
        let input = active_input(&view, cx).expect("active input");

        cx.update_window_entity(&input, |input, window, cx| {
            input.insert("\n", window, cx);
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 5,
                },
                window,
                cx,
            );
        });
        cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_input_event(&InputEvent::PressEnter { secondary: false }, window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(
            block_texts(&snapshot),
            vec!["First".to_string(), "".to_string()]
        );
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[1].id));
        assert_eq!(snapshot.active_cursor_offset, Some(0));
        assert!(active_input_has_focus(&view, cx));
    }

    #[gpui::test]
    fn press_enter_recovers_when_pending_change_was_lost(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First", 0, Some(5));
        let input = active_input(&view, cx).expect("active input");

        cx.update_window_entity(&input, |input, window, cx| {
            input.insert("\n", window, cx);
        });
        cx.update_window_entity(&view, |editor, window, cx| {
            editor.interaction.clear_pending_enter_change();
            editor.handle_input_event(&InputEvent::PressEnter { secondary: false }, window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(
            block_texts(&snapshot),
            vec!["First".to_string(), "".to_string()]
        );
        assert_eq!(snapshot.active_block_id, Some(snapshot.blocks[1].id));
        assert_eq!(snapshot.active_cursor_offset, Some(0));
        assert!(active_input_has_focus(&view, cx));
    }

    fn build_editor_window(
        cx: &mut TestAppContext,
    ) -> (Entity<MarkdownEditor>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::bind_keys(cx);
        });
        let editor = Rc::new(RefCell::new(None));
        let editor_for_root = editor.clone();
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| MarkdownEditor::new(window, cx));
            *editor_for_root.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });

        (
            editor
                .borrow()
                .clone()
                .expect("editor view should be captured from component root"),
            cx,
        )
    }

    fn load_document(
        cx: &mut VisualTestContext,
        view: &Entity<MarkdownEditor>,
        text: &str,
        block_index: usize,
        cursor_offset: Option<usize>,
    ) {
        let active_input = cx.update_window_entity(view, |editor, window, cx| {
            let mut controller = EditorController::new(
                DocumentSource::Text {
                    path: None,
                    suggested_path: None,
                    text: text.to_string(),
                    modified_at: None,
                },
                SyncPolicy::default(),
            );
            let effects = controller.dispatch(EditCommand::ActivateBlock {
                index: block_index,
                cursor_offset,
            });

            editor.controller = controller;
            editor.apply_effects(window, cx, effects);
            editor
                .interaction
                .active_session()
                .map(|session| session.input.entity().clone())
        });
        cx.update(|window: &mut Window, _| {
            window.refresh();
            window.activate_window();
        });
        if let Some(active_input) = active_input {
            cx.focus(&active_input);
        }
        cx.run_until_parked();
    }

    fn block_ids(view: &Entity<MarkdownEditor>, cx: &VisualTestContext) -> Vec<u64> {
        snapshot(view, cx)
            .blocks
            .iter()
            .map(|block| block.id)
            .collect()
    }

    fn active_block_id(view: &Entity<MarkdownEditor>, cx: &VisualTestContext) -> Option<u64> {
        snapshot(view, cx).active_block_id
    }

    fn active_input(
        view: &Entity<MarkdownEditor>,
        cx: &mut VisualTestContext,
    ) -> Option<Entity<gpui_component::input::InputState>> {
        cx.update_window_entity(view, |editor, _, _| {
            editor
                .interaction
                .active_session()
                .map(|session| session.input.entity().clone())
        })
    }

    fn active_input_has_focus(view: &Entity<MarkdownEditor>, cx: &mut VisualTestContext) -> bool {
        cx.update_window_entity(view, |editor, window, cx| {
            editor
                .interaction
                .active_session()
                .map(|session| session.input.contains_focus(window, cx))
                .unwrap_or(false)
        })
    }

    fn snapshot(view: &Entity<MarkdownEditor>, cx: &VisualTestContext) -> EditorSnapshot {
        cx.read(|app| view.read(app).snapshot())
    }

    fn block_texts(snapshot: &EditorSnapshot) -> Vec<String> {
        snapshot
            .blocks
            .iter()
            .map(|block| block.text.clone())
            .collect()
    }
}
