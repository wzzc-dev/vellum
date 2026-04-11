use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, VisualContext, Window, div, px,
};
use gpui_component::{
    ActiveTheme,
    input::{Backspace, Delete, Enter, IndentInline, Input, InputEvent, InputState, OutdentInline},
    scroll::ScrollableElement,
};

use crate::{
    EditCommand, SelectionState,
    core::controller::{DocumentSource, EditorController, EditorSnapshot, SyncPolicy},
};

use super::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, EDITOR_CONTEXT, MAX_EDITOR_WIDTH,
    input_bridge::build_document_input, surface::render_document_surface,
};

#[derive(Debug, Clone)]
pub enum EditorEvent {
    Changed(EditorSnapshot),
}

pub struct MarkdownEditor {
    pub(super) controller: EditorController,
    pub(super) snapshot: EditorSnapshot,
    pub(super) document_input: Entity<InputState>,
    _input_subscription: Subscription,
    _input_observer: Subscription,
    autosave_generation: u64,
    pub(super) syncing_input: bool,
    pub(super) input_focused: bool,
    pub(super) block_bounds: Rc<RefCell<HashMap<u64, gpui::Bounds<gpui::Pixels>>>>,
}

impl EventEmitter<EditorEvent> for MarkdownEditor {}

impl MarkdownEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let controller = EditorController::new(
            DocumentSource::Empty {
                suggested_path: None,
            },
            SyncPolicy::default(),
        );
        let snapshot = controller.snapshot();
        let document_input =
            cx.new(|cx| build_document_input(&snapshot.display_map.visible_text, window, cx));
        let view = cx.entity();
        let input_subscription = window.subscribe(
            &document_input,
            cx,
            move |_, event: &InputEvent, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.handle_input_event(event, window, cx);
                });
            },
        );
        let observed_view = cx.entity();
        let input_observer = window.observe(&document_input, cx, move |_, window, cx| {
            let _ = observed_view.update(cx, |this, cx| {
                this.handle_observed_input_change(window, cx);
            });
        });

        Self {
            controller,
            snapshot,
            document_input,
            _input_subscription: input_subscription,
            _input_observer: input_observer,
            autosave_generation: 0,
            syncing_input: false,
            input_focused: false,
            block_bounds: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        self.snapshot.clone()
    }

    pub(super) fn emit_changed(&mut self, cx: &mut Context<Self>) {
        let snapshot = self.snapshot();
        cx.emit(EditorEvent::Changed(snapshot));
        cx.notify();
    }

    pub(crate) fn apply_markup(
        &mut self,
        before: &str,
        after: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.dispatch(EditCommand::ToggleInlineMarkup {
            before: before.to_string(),
            after: after.to_string(),
        });
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn adjust_current_block(
        &mut self,
        deepen: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let command = if deepen {
            EditCommand::Indent
        } else {
            EditCommand::Outdent
        };
        let effects = self.controller.dispatch(command);
        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn exit_edit_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(self.snapshot.selection.cursor()),
        });
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn focus_adjacent_block(
        &mut self,
        direction: isize,
        preferred_column: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.dispatch(EditCommand::MoveCaret {
            direction,
            preferred_column,
        });
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::Undo);
        self.apply_effects(window, cx, effects);
    }

    pub(crate) fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let effects = self.controller.dispatch(EditCommand::Redo);
        self.apply_effects(window, cx, effects);
    }

    fn handle_enter(
        &mut self,
        secondary: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let effects = self
            .controller
            .dispatch(EditCommand::InsertBreak { plain: secondary });
        if !effects.changed && !effects.selection_changed {
            return false;
        }

        self.schedule_autosave(window, cx);
        self.apply_effects(window, cx, effects);
        true
    }

    fn handle_delete_backward(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let effects = self.controller.dispatch(EditCommand::DeleteBackward);
        if !effects.changed {
            return false;
        }

        self.schedule_autosave(window, cx);
        self.apply_effects(window, cx, effects);
        true
    }

    fn handle_delete_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let effects = self.controller.dispatch(EditCommand::DeleteForward);
        if !effects.changed {
            return false;
        }

        self.schedule_autosave(window, cx);
        self.apply_effects(window, cx, effects);
        true
    }

    fn handle_indent(&mut self, deepen: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let command = if deepen {
            EditCommand::Indent
        } else {
            EditCommand::Outdent
        };
        let effects = self.controller.dispatch(command);
        if !effects.changed {
            return false;
        }

        self.schedule_autosave(window, cx);
        self.apply_effects(window, cx, effects);
        true
    }

    pub(super) fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let token = self.autosave_generation.wrapping_add(1);
        self.autosave_generation = token;
        let view = cx.entity();
        let autosave_delay = self.controller.autosave_delay();
        window
            .spawn(cx, async move |cx| {
                gpui::Timer::after(autosave_delay).await;
                let _ = cx.update_window_entity(&view, |this, window, cx| {
                    if this.autosave_generation == token && this.snapshot.dirty {
                        let _ = this.save(window, cx);
                    }
                });
            })
            .detach();
    }

    fn render_conflict_banner(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if !self.snapshot.has_conflict {
            return None;
        }

        Some(
            div()
                .mb_4()
                .rounded(px(8.))
                .border_1()
                .border_color(cx.theme().warning.opacity(0.24))
                .bg(cx.theme().warning.opacity(0.08))
                .px_3()
                .py_2()
                .child("External file changes detected. Save or reload to resolve."),
        )
    }

    pub(super) fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.document_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
        self.input_focused = true;
    }
}

impl Render for MarkdownEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let conflict_banner = self
            .render_conflict_banner(cx)
            .map(|banner| banner.into_any_element());
        self.block_bounds.borrow_mut().clear();

        div()
            .id("markdown-editor")
            .key_context(EDITOR_CONTEXT)
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
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
            .capture_action({
                let enter_view = view.clone();
                move |action: &Enter, window, app: &mut App| {
                    let handled = enter_view.update(app, |this, cx| {
                        this.handle_enter(action.secondary, window, cx)
                    });
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &Backspace, window, app: &mut App| {
                    let handled =
                        view.update(app, |this, cx| this.handle_delete_backward(window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &Delete, window, app: &mut App| {
                    let handled =
                        view.update(app, |this, cx| this.handle_delete_forward(window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &IndentInline, window, app: &mut App| {
                    let handled = view.update(app, |this, cx| this.handle_indent(true, window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &OutdentInline, window, app: &mut App| {
                    let handled =
                        view.update(app, |this, cx| this.handle_indent(false, window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .child(
                div().size_full().flex().flex_col().child(
                    div().flex_1().overflow_y_scrollbar().child(
                        div().w_full().px_8().pt(px(28.)).pb(px(44.)).child(
                            div()
                                .mx_auto()
                                .max_w(px(MAX_EDITOR_WIDTH))
                                .w_full()
                                .when_some(conflict_banner, |this, banner| this.child(banner))
                                .child(
                                    div()
                                        .relative()
                                        .w_full()
                                        .child(
                                            Input::new(&self.document_input)
                                                .appearance(false)
                                                .bordered(false)
                                                .focus_bordered(false)
                                                .absolute()
                                                .top(px(0.))
                                                .left(px(0.))
                                                .right(px(0.))
                                                .bottom(px(0.))
                                                .w_full()
                                                .h_full()
                                                .px(px(0.))
                                                .py(px(0.))
                                                .text_size(px(BODY_FONT_SIZE))
                                                .line_height(px(BODY_LINE_HEIGHT))
                                                .opacity(0.),
                                        )
                                        .child(render_document_surface(
                                            &view,
                                            &self.snapshot,
                                            self.input_focused,
                                            self.block_bounds.clone(),
                                            window,
                                            cx,
                                        )),
                                ),
                        ),
                    ),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        AppContext, Entity, EntityInputHandler as _, Modifiers, MouseButton, TestAppContext,
        VisualContext, VisualTestContext, point, px,
    };
    use gpui_component::{Root, input::Position};

    use super::*;
    use crate::ui::surface::{
        caret_visual_offset_for_block, rendered_text_for_block, rendered_visible_end,
        rendered_visible_len, shape_block_lines, text_content_x_offset,
    };
    use crate::{BlockKind, RenderSpanKind, SelectionAffinity};

    #[test]
    fn heading_enter_shows_single_caret_in_empty_following_block() {
        let snapshot = snapshot_for_text_with_selection("# H1\n\n", 6);
        let blocks = &snapshot.display_map.blocks;

        assert_eq!(blocks.len(), 2);
        assert_eq!(rendered_text_for_block(&blocks[0]), "H1");
        assert_eq!(rendered_text_for_block(&blocks[1]), "");
        assert_eq!(snapshot.visible_selection.cursor(), 4);
        assert_eq!(
            caret_visual_offset_for_block(blocks, 0, snapshot.visible_selection.cursor()),
            None
        );
        assert_eq!(
            caret_visual_offset_for_block(blocks, 1, snapshot.visible_selection.cursor()),
            Some(0)
        );
    }

    #[test]
    fn line_break_spans_remain_mapped_but_not_rendered_for_heading_block() {
        let snapshot = snapshot_for_text_with_selection("# H1\n\n## H2", 6);
        let heading = &snapshot.display_map.blocks[0];

        assert!(
            heading
                .spans
                .iter()
                .any(|span| span.kind == RenderSpanKind::LineBreak)
        );
        assert_eq!(rendered_text_for_block(heading), "H1");
        assert_eq!(rendered_visible_end(heading), 2);
    }

    #[test]
    fn empty_paragraph_between_headings_occupies_one_editable_line() {
        let snapshot = snapshot_for_text_with_selection("# H1\n\n", 6);
        let empty_block = &snapshot.display_map.blocks[1];

        assert_eq!(rendered_visible_len(empty_block), 0);
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                1,
                snapshot.visible_selection.cursor(),
            ),
            Some(0)
        );
    }

    #[gpui::test]
    fn two_headings_separated_by_single_empty_paragraph_render_single_visual_gap(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# H1\n\n## H2");

        let (first_lines, second_lines) = cx.update_window_entity(&view, |editor, window, _| {
            let blocks = editor.snapshot.display_map.blocks.clone();
            (
                shape_block_lines(&blocks[0], px(640.), window).len(),
                shape_block_lines(&blocks[1], px(640.), window).len(),
            )
        });

        assert_eq!(first_lines, 1);
        assert_eq!(second_lines, 1);
    }

    #[gpui::test]
    fn list_and_following_paragraph_render_single_visual_gap(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "- item\n\nNext");

        let (first_lines, second_lines) = cx.update_window_entity(&view, |editor, window, _| {
            let blocks = editor.snapshot.display_map.blocks.clone();
            (
                shape_block_lines(&blocks[0], px(640.), window).len(),
                shape_block_lines(&blocks[1], px(640.), window).len(),
            )
        });

        assert_eq!(first_lines, 1);
        assert_eq!(second_lines, 1);
    }

    #[gpui::test]
    fn blockquote_and_following_paragraph_render_single_visual_gap(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "> Quote\n\nNext");

        let (first_lines, second_lines) = cx.update_window_entity(&view, |editor, window, _| {
            let blocks = editor.snapshot.display_map.blocks.clone();
            (
                shape_block_lines(&blocks[0], px(640.), window).len(),
                shape_block_lines(&blocks[1], px(640.), window).len(),
            )
        });

        assert_eq!(first_lines, 1);
        assert_eq!(second_lines, 1);
    }

    #[test]
    fn blockquote_overlay_uses_text_area_origin_without_extra_gutter_offset() {
        let snapshot = snapshot_for_text_with_selection("> Quote", 2);
        let blockquote = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Blockquote))
            .expect("blockquote block");

        assert_eq!(text_content_x_offset(blockquote), px(0.));
    }

    #[gpui::test]
    fn single_surface_syncs_document_text(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First");
        let input = document_input(&view, cx);

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value("Changed".to_string(), window, cx);
        });
        cx.run_until_parked();

        assert_eq!(snapshot(&view, cx).document_text, "Changed");
    }

    #[gpui::test]
    fn single_surface_uses_visible_heading_text_and_preserves_markup(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# Title");
        let input = document_input(&view, cx);

        let visible_text = cx.update_window_entity(&input, |input, _, _| input.text().to_string());
        assert_eq!(visible_text, "Title");

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value("Changed".to_string(), window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "# Changed");
        assert_eq!(snapshot.display_map.visible_text, "Changed");
    }

    #[gpui::test]
    fn single_surface_hides_list_prefix_but_preserves_source_markup(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "- item");
        let input = document_input(&view, cx);

        assert_eq!(snapshot(&view, cx).display_map.visible_text, "item");

        let visible_text = cx.update_window_entity(&input, |input, _, _| input.text().to_string());
        assert_eq!(visible_text, "item");

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value("changed".to_string(), window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "- changed");
        assert_eq!(snapshot.display_map.visible_text, "changed");
    }

    #[gpui::test]
    fn single_surface_hides_blockquote_prefix_but_preserves_source_markup(cx: &mut TestAppContext) {
        assert_visible_edit_round_trip(cx, "> quote", "quote", "changed", "> changed");
    }

    #[gpui::test]
    fn single_surface_preserves_bold_markup_when_editing_visible_text(cx: &mut TestAppContext) {
        assert_visible_edit_round_trip(
            cx,
            "Hello **world**",
            "Hello world",
            "Hello there",
            "Hello **there**",
        );
    }

    #[gpui::test]
    fn single_surface_preserves_inline_code_markup_when_editing_visible_text(
        cx: &mut TestAppContext,
    ) {
        assert_visible_edit_round_trip(
            cx,
            "Hello `world`",
            "Hello world",
            "Hello there",
            "Hello `there`",
        );
    }

    #[gpui::test]
    fn single_surface_preserves_link_markup_when_editing_visible_text(cx: &mut TestAppContext) {
        assert_visible_edit_round_trip(
            cx,
            "Hello [world](https://example.com)",
            "Hello world",
            "Hello there",
            "Hello [there](https://example.com)",
        );
    }

    #[gpui::test]
    fn single_surface_renders_task_marker_as_checkbox_and_preserves_markdown(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "- [ ] task");
        let input = document_input(&view, cx);

        assert_eq!(
            snapshot(&view, cx).display_map.visible_text,
            "\u{2610} task"
        );
        let visible_text = cx.update_window_entity(&input, |input, _, _| input.text().to_string());
        assert_eq!(visible_text, "\u{2610} task");

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value("\u{2610} changed".to_string(), window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "- [ ] changed");
        assert_eq!(snapshot.display_map.visible_text, "\u{2610} changed");
    }

    #[gpui::test]
    fn moving_to_start_of_hidden_heading_reveals_marker_boundary(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# Title");
        cx.update_window_entity(&view, |editor, window, cx| {
            let mut selection = SelectionState::collapsed(1);
            selection.affinity = SelectionAffinity::Upstream;
            let effects = editor
                .controller
                .dispatch(EditCommand::SetSelection { selection });
            editor.apply_effects(window, cx, effects);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.display_map.visible_text, "# Title");
        assert_eq!(snapshot.selection.cursor(), 1);
        assert_eq!(snapshot.visible_selection.cursor(), 1);
    }

    #[gpui::test]
    fn vertical_move_between_blockquote_lines_keeps_hidden_prefixes(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "> First\n> Second");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 5,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 1,
                    character: 0,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let down_snapshot = snapshot(&view, cx);
        let blockquote = down_snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Blockquote))
            .expect("blockquote block");
        let mut markers = blockquote.spans.iter().filter(|span| {
            span.kind == RenderSpanKind::HiddenSyntax && span.source_text.starts_with('>')
        });
        let _first_marker = markers.next().expect("first blockquote marker");
        let second_marker = markers.next().expect("second blockquote marker");

        assert!(second_marker.hidden);
        assert_eq!(
            down_snapshot.selection.cursor(),
            second_marker.source_range.end
        );
        assert_eq!(
            down_snapshot.visible_selection.cursor(),
            second_marker.visible_range.start
        );

        let (visible_text, cursor) = cx.update_window_entity(&input, |input, _, _| {
            (input.text().to_string(), input.cursor())
        });
        assert_eq!(visible_text, "First\nSecond");
        assert_eq!(cursor, down_snapshot.visible_selection.cursor());

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 0,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let up_snapshot = snapshot(&view, cx);
        let blockquote = up_snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Blockquote))
            .expect("blockquote block");
        let first_marker = blockquote
            .spans
            .iter()
            .find(|span| {
                span.kind == RenderSpanKind::HiddenSyntax && span.source_text.starts_with('>')
            })
            .expect("first blockquote marker");
        assert!(first_marker.hidden);
        assert_eq!(
            up_snapshot.selection.cursor(),
            first_marker.source_range.end
        );
        assert_eq!(
            up_snapshot.visible_selection.cursor(),
            first_marker.visible_range.start
        );
    }

    #[gpui::test]
    fn enter_splits_paragraph_in_single_surface(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 5,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "First\n\n");
        assert_eq!(snapshot.selection.cursor(), 7);
    }

    #[gpui::test]
    fn backspace_merges_trailing_empty_block_in_single_surface(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "First\n\n");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 2,
                    character: 0,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "First");
        assert_eq!(snapshot.selection.cursor(), 5);
    }

    #[gpui::test]
    fn formatting_command_collapses_selection_in_snapshot_and_hidden_input(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "hello");

        cx.update_window_entity(&view, |editor, window, cx| {
            let selection = SelectionState {
                anchor_byte: 1,
                head_byte: 4,
                preferred_column: None,
                affinity: SelectionAffinity::Downstream,
            };
            let effects = editor
                .controller
                .dispatch(EditCommand::SetSelection { selection });
            editor.apply_effects(window, cx, effects);
            editor.apply_markup("**", "**", window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "h**ell**o");
        assert!(snapshot.selection.is_collapsed());

        let input = document_input(&view, cx);
        let (cursor, selection_range) = cx.update_window_entity(&input, |input, window, cx| {
            (
                input.cursor(),
                input
                    .selected_text_range(true, window, cx)
                    .map(|selection| selection.range),
            )
        });
        assert_eq!(cursor, snapshot.visible_selection.cursor());
        assert!(
            selection_range
                .as_ref()
                .map(std::ops::Range::is_empty)
                .unwrap_or(true)
        );
    }

    #[gpui::test]
    fn mouse_down_on_lower_block_updates_caret_before_mouse_up(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# Heading\n\nParagraph");

        let (target_bounds, expected_cursor) = cx.update_window_entity(&view, |editor, _, _| {
            let block = editor.snapshot.display_map.blocks[1].clone();
            let bounds = editor
                .block_bounds
                .borrow()
                .get(&block.id)
                .copied()
                .expect("rendered block bounds should exist");
            (bounds, block.content_range.start)
        });

        cx.simulate_mouse_down(
            point(target_bounds.left() + px(1.), target_bounds.top() + px(8.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.selection.cursor(), expected_cursor);
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                0,
                snapshot.visible_selection.cursor(),
            ),
            None
        );
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                1,
                snapshot.visible_selection.cursor(),
            ),
            Some(0)
        );
    }

    fn assert_visible_edit_round_trip(
        cx: &mut TestAppContext,
        source_text: &str,
        expected_visible_text: &str,
        edited_visible_text: &str,
        expected_source_text: &str,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, source_text);
        let input = document_input(&view, cx);

        let visible_text = cx.update_window_entity(&input, |input, _, _| input.text().to_string());
        assert_eq!(visible_text, expected_visible_text);

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value(edited_visible_text.to_string(), window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected_source_text);
        assert_eq!(snapshot.display_map.visible_text, edited_visible_text);
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

    fn load_document(cx: &mut VisualTestContext, view: &Entity<MarkdownEditor>, text: &str) {
        cx.update_window_entity(view, |editor, window, cx| {
            editor.controller = EditorController::new(
                DocumentSource::Text {
                    path: None,
                    suggested_path: None,
                    text: text.to_string(),
                    modified_at: None,
                },
                SyncPolicy::default(),
            );
            editor.snapshot = editor.controller.snapshot();
            editor.sync_input_from_snapshot(window, cx);
        });
        cx.run_until_parked();
    }

    fn document_input(
        view: &Entity<MarkdownEditor>,
        cx: &mut VisualTestContext,
    ) -> Entity<InputState> {
        cx.update_window_entity(view, |editor, _, _| editor.document_input.clone())
    }

    fn snapshot(view: &Entity<MarkdownEditor>, cx: &VisualTestContext) -> EditorSnapshot {
        cx.read(|app| view.read(app).snapshot())
    }

    fn snapshot_for_text_with_selection(text: &str, cursor: usize) -> EditorSnapshot {
        let mut controller = EditorController::new(
            DocumentSource::Text {
                path: None,
                suggested_path: None,
                text: text.to_string(),
                modified_at: None,
            },
            SyncPolicy::default(),
        );
        controller.dispatch(EditCommand::SetSelection {
            selection: SelectionState::collapsed(cursor),
        });
        controller.snapshot()
    }
}
