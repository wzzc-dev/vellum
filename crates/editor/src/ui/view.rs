use gpui::{
    Context, EventEmitter, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Window, div, px,
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
    use std::path::PathBuf;

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
}
