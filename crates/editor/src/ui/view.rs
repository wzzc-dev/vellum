use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Subscription, VisualContext, Window,
    div, px,
};
use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariants as _},
    input::{
        Backspace, Delete, DeleteToNextWordEnd, Enter, IndentInline, Input, InputEvent, InputState,
        OutdentInline,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SurfaceSelectionAnchor {
    pub(super) source_offset: usize,
    pub(super) visible_offset: usize,
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
    pub(super) drag_selection_anchor: Option<SurfaceSelectionAnchor>,
    pub(super) scroll_handle: ScrollHandle,
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
            drag_selection_anchor: None,
            scroll_handle: ScrollHandle::new(),
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

    pub(crate) fn toggle_heading(
        &mut self,
        depth: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self
            .controller
            .dispatch(EditCommand::ToggleHeading { depth });
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

        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        true
    }

    pub(crate) fn secondary_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.input_has_marked_text(window, cx) {
            return;
        }

        self.sync_selection_from_input(window, cx);
        let effects = if selection_is_within_table(&self.snapshot) {
            self.controller.exit_table()
        } else {
            self.controller
                .dispatch(EditCommand::InsertBreak { plain: true })
        };
        if !effects.changed && !effects.selection_changed {
            return;
        }

        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
    }

    fn handle_delete_table_row(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        self.sync_selection_from_input(window, cx);
        if !selection_is_within_table(&self.snapshot) {
            return false;
        }

        let effects = self.controller.delete_table_row();
        if !effects.changed && !effects.selection_changed {
            return false;
        }

        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        true
    }

    fn handle_delete_backward(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let effects = self.controller.dispatch(EditCommand::DeleteBackward);
        if !effects.changed && !effects.selection_changed {
            return false;
        }

        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        true
    }

    fn handle_delete_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let effects = self.controller.dispatch(EditCommand::DeleteForward);
        if !effects.changed && !effects.selection_changed {
            return false;
        }

        if effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
        true
    }

    fn handle_indent(&mut self, deepen: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.input_has_marked_text(window, cx) {
            return false;
        }

        let effects = if selection_is_within_table(&self.snapshot) {
            self.controller.navigate_table(!deepen)
        } else {
            let command = if deepen {
                EditCommand::Indent
            } else {
                EditCommand::Outdent
            };
            self.controller.dispatch(command)
        };
        if !effects.changed && !effects.selection_changed {
            return false;
        }

        if effects.changed {
            self.schedule_autosave(window, cx);
        }
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

    fn render_conflict_banner(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        if !self.snapshot.has_conflict {
            return None;
        }

        let view = cx.entity();
        let reload_view = view.clone();
        let keep_view = view.clone();

        Some(
            div()
                .mb_4()
                .rounded(px(8.))
                .border_1()
                .border_color(cx.theme().warning.opacity(0.24))
                .bg(cx.theme().warning.opacity(0.08))
                .px_3()
                .py_3()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .text_color(cx.theme().foreground)
                        .child("External file changes detected. Choose the disk copy or keep your current buffer."),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new("reload-disk")
                                .label("Reload Disk")
                                .ghost()
                                .compact()
                                .on_click(move |_, window, app: &mut App| {
                                    let _ = reload_view.update(app, |this, cx| {
                                        this.reload_conflict(window, cx);
                                    });
                                }),
                        )
                        .child(
                            Button::new("keep-current")
                                .label("Keep Mine")
                                .ghost()
                                .compact()
                                .on_click(move |_, window, app: &mut App| {
                                    let _ = keep_view.update(app, |this, cx| {
                                        this.keep_current_conflict(window, cx);
                                    });
                                }),
                        ),
                )
                .into_any_element(),
        )
    }

    pub(super) fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.document_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
        self.input_focused = true;
    }
}

fn selection_is_within_table(snapshot: &EditorSnapshot) -> bool {
    let range = snapshot.selection.range();
    snapshot.display_map.blocks.iter().any(|block| {
        block.kind == crate::BlockKind::Table
            && range.start >= block.content_range.start
            && range.end <= block.content_range.end
    })
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
            .on_action(cx.listener(Self::on_toggle_source_mode))
            .on_action(cx.listener(Self::on_undo_edit))
            .on_action(cx.listener(Self::on_redo_edit))
            .on_action(cx.listener(Self::on_secondary_enter))
            .on_action(cx.listener(Self::on_toggle_heading1))
            .on_action(cx.listener(Self::on_toggle_heading2))
            .on_action(cx.listener(Self::on_toggle_heading3))
            .on_action(cx.listener(Self::on_toggle_heading4))
            .on_action(cx.listener(Self::on_toggle_heading5))
            .on_action(cx.listener(Self::on_toggle_heading6))
            .on_action(cx.listener(Self::on_toggle_paragraph))
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
                move |_: &DeleteToNextWordEnd, window, app: &mut App| {
                    let handled =
                        view.update(app, |this, cx| this.handle_delete_table_row(window, cx));
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
                    div()
                        .flex_1()
                        .id("editor-scroll-container")
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll_handle)
                        .vertical_scrollbar(&self.scroll_handle)
                        .child(
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
                                           self.scroll_handle.clone(),
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
        AppContext, Entity, EntityInputHandler as _, Modifiers, MouseButton, MouseDownEvent,
        MouseUpEvent, TestAppContext, VisualContext, VisualTestContext, point, px,
    };
    use gpui_component::{Root, input::Position};

    use super::*;
    use crate::ui::surface::{
        caret_visual_offset_for_block, rendered_empty_block_line_count, rendered_text_for_block,
        rendered_visible_end, rendered_visible_len, shape_block_lines,
        surface_empty_block_line_count, text_content_x_offset,
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

    #[test]
    fn inter_block_extra_blank_lines_keep_mapping_but_collapse_visually() {
        let snapshot = snapshot_for_text_with_selection("A\n\n\n\nB", 1);
        let blocks = &snapshot.display_map.blocks;

        assert_eq!(blocks.len(), 3);
        assert_eq!(rendered_empty_block_line_count(&blocks[1]), Some(3));
        assert_eq!(surface_empty_block_line_count(blocks, 1), Some(1));
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
    fn multiple_blank_lines_between_paragraphs_render_single_visual_gap(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");

        let (raw_line_count, surface_line_count) =
            cx.update_window_entity(&view, |editor, _, _| {
                let blocks = editor.snapshot.display_map.blocks.clone();
                (
                    rendered_empty_block_line_count(&blocks[1]).unwrap_or(0),
                    surface_empty_block_line_count(&blocks, 1).unwrap_or(0),
                )
            });

        assert_eq!(raw_line_count, 3);
        assert_eq!(surface_line_count, 1);
    }

    #[gpui::test]
    fn moving_down_across_collapsed_inter_block_gap_uses_single_visual_blank_line(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
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

        let blank_snapshot = snapshot(&view, cx);
        let (blank_cursor, blank_position) = cx.update_window_entity(&input, |input, _, _| {
            (input.cursor(), input.cursor_position())
        });
        assert_eq!(blank_snapshot.selection.cursor(), 3);
        assert_eq!(blank_cursor, 2);

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: blank_position.line + 1,
                    character: 0,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let final_snapshot = snapshot(&view, cx);
        let final_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        assert_eq!(final_snapshot.selection.cursor(), 6);
        assert!(final_cursor > blank_cursor);
    }

    #[gpui::test]
    fn pressing_down_across_collapsed_gap_stops_once_then_enters_next_block(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        let first_snapshot = snapshot(&view, cx);
        let first_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        assert_eq!(first_snapshot.selection.cursor(), 3);
        assert_eq!(first_cursor, 2);

        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        let second_snapshot = snapshot(&view, cx);
        let second_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        assert_eq!(second_snapshot.selection.cursor(), 6);
        assert!(second_cursor > first_cursor);
    }

    #[gpui::test]
    fn pressing_up_across_collapsed_gap_stops_once_then_enters_previous_block(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down down");
        cx.run_until_parked();

        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let first_snapshot = snapshot(&view, cx);
        let first_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        assert_eq!(first_snapshot.selection.cursor(), 3);
        assert_eq!(first_cursor, 2);

        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let second_snapshot = snapshot(&view, cx);
        let second_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        assert_eq!(second_snapshot.selection.cursor(), 1);
        assert!(second_cursor < first_cursor);
    }

    #[gpui::test]
    fn pressing_down_twice_from_gap_enters_heading_content_start(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\n# Heading");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down down");
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        let heading = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Heading { .. }))
            .expect("heading block");
        let expected_cursor = heading.visible_range.start + 1;
        assert_eq!(
            snapshot.selection.cursor(),
            snapshot
                .display_map
                .visible_to_source_with_affinity(expected_cursor, SelectionAffinity::Downstream,)
                .source_offset
        );
        assert_eq!(snapshot.visible_selection.cursor(), expected_cursor);
    }

    #[gpui::test]
    fn pressing_down_twice_from_gap_enters_blockquote_content_start(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\n> Quote");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down down");
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        let blockquote = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Blockquote))
            .expect("blockquote block");
        let expected_cursor = blockquote.visible_range.start + 1;
        assert_eq!(
            snapshot.selection.cursor(),
            snapshot
                .display_map
                .visible_to_source_with_affinity(expected_cursor, SelectionAffinity::Downstream,)
                .source_offset
        );
        assert_eq!(snapshot.visible_selection.cursor(), expected_cursor);
    }

    #[gpui::test]
    fn pressing_down_twice_from_heading_gap_keeps_visual_and_source_carets_in_sync(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# A\n\n\n\n## AA\n\n123\n\n123\n\n123");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down down");
        cx.run_until_parked();

        let first_snapshot = snapshot(&view, cx);
        let heading_index = first_snapshot
            .display_map
            .blocks
            .iter()
            .position(|block| {
                matches!(block.kind, BlockKind::Heading { depth: 2 })
                    && rendered_text_for_block(block) == "AA"
            })
            .expect("second heading block");
        let heading = &first_snapshot.display_map.blocks[heading_index];
        let input_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        let expected_cursor = heading.visible_range.start + 1;

        assert_eq!(first_snapshot.visible_selection.cursor(), expected_cursor);
        assert_eq!(input_cursor, expected_cursor);
        assert_eq!(
            first_snapshot.selection.cursor(),
            first_snapshot
                .display_map
                .visible_to_source_with_affinity(expected_cursor, SelectionAffinity::Downstream,)
                .source_offset
        );
        assert_eq!(
            caret_visual_offset_for_block(
                &first_snapshot.display_map.blocks,
                heading_index.saturating_sub(1),
                first_snapshot.visible_selection.cursor(),
            ),
            None
        );
        assert_eq!(
            caret_visual_offset_for_block(
                &first_snapshot.display_map.blocks,
                heading_index,
                first_snapshot.visible_selection.cursor(),
            ),
            Some(1)
        );

        cx.simulate_keystrokes("x");
        cx.run_until_parked();

        let final_snapshot = snapshot(&view, cx);
        assert!(
            final_snapshot
                .document_text
                .starts_with("# A\n\n\n\n## AxA")
        );
    }

    #[gpui::test]
    fn pressing_down_across_gap_enters_list_content_start(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\n- item");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down down");
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        let list = snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::List))
            .expect("list block");
        let expected_cursor = list.visible_range.start + 1;
        assert_eq!(
            snapshot.selection.cursor(),
            snapshot
                .display_map
                .visible_to_source_with_affinity(expected_cursor, SelectionAffinity::Downstream,)
                .source_offset
        );
        assert_eq!(snapshot.visible_selection.cursor(), expected_cursor);
    }

    #[gpui::test]
    fn pressing_down_twice_from_gap_into_heading_keeps_visual_caret_and_input_in_sync(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# A\n\n\n\n## AA\n\n123\n\n123\n\n123\n");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        let gap_snapshot = snapshot(&view, cx);
        let gap_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        assert_eq!(gap_snapshot.selection.cursor(), 5);
        assert_eq!(gap_cursor, 2);

        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        let heading_snapshot = snapshot(&view, cx);
        let (heading_cursor, heading_text) = cx.update_window_entity(&input, |input, _, _| {
            (input.cursor(), input.text().to_string())
        });
        let heading_block = heading_snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| matches!(block.kind, BlockKind::Heading { depth: 2 }))
            .expect("heading block");
        let expected_cursor = heading_block.visible_range.start + 1;

        assert_eq!(
            heading_snapshot.selection.cursor(),
            heading_snapshot
                .display_map
                .visible_to_source_with_affinity(expected_cursor, SelectionAffinity::Downstream,)
                .source_offset
        );
        assert_eq!(heading_snapshot.visible_selection.cursor(), expected_cursor);
        assert_eq!(heading_cursor, heading_snapshot.visible_selection.cursor());
        assert_eq!(heading_text, heading_snapshot.display_map.visible_text);
        assert_eq!(
            caret_visual_offset_for_block(
                &heading_snapshot.display_map.blocks,
                1,
                heading_snapshot.visible_selection.cursor(),
            ),
            None
        );
        assert_eq!(
            caret_visual_offset_for_block(
                &heading_snapshot.display_map.blocks,
                2,
                heading_snapshot.visible_selection.cursor(),
            ),
            Some(1)
        );

        cx.simulate_keystrokes("x");
        cx.run_until_parked();

        let typed_snapshot = snapshot(&view, cx);
        assert_eq!(
            typed_snapshot.document_text,
            "# A\n\n\n\n## AxA\n\n123\n\n123\n\n123\n"
        );
        assert_eq!(
            typed_snapshot.visible_selection.cursor(),
            heading_block.visible_range.start + 2
        );
    }

    #[gpui::test]
    fn pressing_down_through_mixed_blocks_does_not_skip_to_last_line(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# 1\n\n## 2\n\n12\n\n34");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let initial_snapshot = snapshot(&view, cx);
        assert_eq!(initial_snapshot.display_map.blocks.len(), 4);

        let expected_blocks = [(1usize, "2"), (2usize, "12"), (3usize, "34")];

        for (expected_block_index, expected_text) in expected_blocks {
            cx.simulate_keystrokes("down");
            cx.run_until_parked();

            let step_snapshot = snapshot(&view, cx);
            let input_position =
                cx.update_window_entity(&input, |input, _, _| input.cursor_position());
            let caret_blocks = step_snapshot
                .display_map
                .blocks
                .iter()
                .enumerate()
                .filter_map(|(block_index, _)| {
                    caret_visual_offset_for_block(
                        &step_snapshot.display_map.blocks,
                        block_index,
                        step_snapshot.visible_selection.cursor(),
                    )
                    .map(|_| block_index)
                })
                .collect::<Vec<_>>();

            assert_eq!(
                caret_blocks,
                vec![expected_block_index],
                "input_position={input_position:?}, visible_cursor={}, source_cursor={}",
                step_snapshot.visible_selection.cursor(),
                step_snapshot.selection.cursor()
            );
            assert_eq!(
                rendered_text_for_block(&step_snapshot.display_map.blocks[expected_block_index]),
                expected_text
            );
        }

        let before_typing = snapshot(&view, cx);
        let input_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        let expected_source_cursor = before_typing
            .display_map
            .visible_to_source_with_affinity(input_cursor, SelectionAffinity::Downstream)
            .source_offset;
        assert_eq!(before_typing.selection.cursor(), expected_source_cursor);
    }

    #[gpui::test]
    fn pressing_up_through_mixed_blocks_does_not_skip_to_first_line(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# 1\n\n## 2\n\n12\n\n34");
        let input = document_input(&view, cx);
        cx.focus(&input);

        cx.update_window_entity(&view, |editor, window, cx| {
            let last_block = editor
                .snapshot
                .display_map
                .blocks
                .last()
                .expect("last block")
                .clone();
            let selection = SelectionState::collapsed(
                editor
                    .snapshot
                    .display_map
                    .visible_to_source_with_affinity(
                        last_block.visible_range.start,
                        SelectionAffinity::Downstream,
                    )
                    .source_offset,
            );
            let effects = editor
                .controller
                .dispatch(EditCommand::SetSelection { selection });
            editor.apply_effects(window, cx, effects);
        });
        cx.run_until_parked();

        let expected_blocks = [(2usize, "12"), (1usize, "2"), (0usize, "1")];

        for (expected_block_index, expected_text) in expected_blocks {
            cx.simulate_keystrokes("up");
            cx.run_until_parked();

            let step_snapshot = snapshot(&view, cx);
            let input_position =
                cx.update_window_entity(&input, |input, _, _| input.cursor_position());
            let caret_blocks = step_snapshot
                .display_map
                .blocks
                .iter()
                .enumerate()
                .filter_map(|(block_index, _)| {
                    caret_visual_offset_for_block(
                        &step_snapshot.display_map.blocks,
                        block_index,
                        step_snapshot.visible_selection.cursor(),
                    )
                    .map(|_| block_index)
                })
                .collect::<Vec<_>>();

            assert_eq!(
                caret_blocks,
                vec![expected_block_index],
                "input_position={input_position:?}, visible_cursor={}, source_cursor={}",
                step_snapshot.visible_selection.cursor(),
                step_snapshot.selection.cursor()
            );
            assert_eq!(
                rendered_text_for_block(&step_snapshot.display_map.blocks[expected_block_index]),
                expected_text
            );
        }

        let final_snapshot = snapshot(&view, cx);
        let input_cursor = cx.update_window_entity(&input, |input, _, _| input.cursor());
        let expected_source_cursor = final_snapshot
            .display_map
            .visible_to_source_with_affinity(input_cursor, SelectionAffinity::Downstream)
            .source_offset;
        assert_eq!(final_snapshot.selection.cursor(), expected_source_cursor);
    }

    #[gpui::test]
    fn pressing_down_preserves_nearest_column_across_blocks(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# 1234\n\n## 5678\n\n12\n\n3456");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 3,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let expected = [
            (1usize, "5678", 3usize),
            (2usize, "12", 2usize),
            (3usize, "3456", 3usize),
        ];

        for (expected_block_index, expected_text, expected_offset) in expected {
            cx.simulate_keystrokes("down");
            cx.run_until_parked();

            let step_snapshot = snapshot(&view, cx);
            let caret_offset = caret_visual_offset_for_block(
                &step_snapshot.display_map.blocks,
                expected_block_index,
                step_snapshot.visible_selection.cursor(),
            )
            .expect("caret should be in expected block");

            assert_eq!(
                rendered_text_for_block(&step_snapshot.display_map.blocks[expected_block_index]),
                expected_text
            );
            assert_eq!(caret_offset, expected_offset);
            assert_eq!(step_snapshot.selection.preferred_column, Some(3));
        }
    }

    #[gpui::test]
    fn pressing_up_preserves_nearest_column_across_blocks(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# 1234\n\n## 5678\n\n12\n\n3456");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&view, |editor, window, cx| {
            let last_block = editor
                .snapshot
                .display_map
                .blocks
                .last()
                .expect("last block")
                .clone();
            let selection = SelectionState::collapsed(
                editor
                    .snapshot
                    .display_map
                    .visible_to_source_with_affinity(
                        last_block.visible_range.start + 3,
                        SelectionAffinity::Downstream,
                    )
                    .source_offset,
            );
            let effects = editor
                .controller
                .dispatch(EditCommand::SetSelection { selection });
            editor.apply_effects(window, cx, effects);
        });
        cx.run_until_parked();

        let expected = [
            (2usize, "12", 2usize),
            (1usize, "5678", 3usize),
            (0usize, "1234", 3usize),
        ];

        for (expected_block_index, expected_text, expected_offset) in expected {
            cx.simulate_keystrokes("up");
            cx.run_until_parked();

            let step_snapshot = snapshot(&view, cx);
            let caret_offset = caret_visual_offset_for_block(
                &step_snapshot.display_map.blocks,
                expected_block_index,
                step_snapshot.visible_selection.cursor(),
            )
            .expect("caret should be in expected block");

            assert_eq!(
                rendered_text_for_block(&step_snapshot.display_map.blocks[expected_block_index]),
                expected_text
            );
            assert_eq!(caret_offset, expected_offset);
            assert_eq!(step_snapshot.selection.preferred_column, Some(3));
        }
    }

    #[gpui::test]
    fn pressing_up_from_line_start_enters_previous_line_start(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "12\n\n34");
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

        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        let input_position = cx.update_window_entity(&input, |input, _, _| input.cursor_position());

        assert_eq!(input_position.character, 0);
        assert_eq!(snapshot.visible_selection.cursor(), 0);
        assert_eq!(snapshot.selection.cursor(), 0);
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                0,
                snapshot.visible_selection.cursor()
            ),
            Some(0)
        );
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                1,
                snapshot.visible_selection.cursor()
            ),
            None
        );
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
        assert_visible_edit_round_trip(cx, "> quote", "quote", "changed", "> changed", "changed");
    }

    #[gpui::test]
    fn single_surface_preserves_bold_markup_when_editing_visible_text(cx: &mut TestAppContext) {
        assert_visible_edit_round_trip(
            cx,
            "Hello **world**",
            "Hello world",
            "Hello there",
            "Hello **there**",
            "Hello there",
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
            "Hello there",
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
    fn moving_cursor_into_link_reveals_full_markup(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "[官网](https://example.com/)");
        let input = document_input(&view, cx);
        cx.focus(&input);

        let collapsed_snapshot = snapshot(&view, cx);
        assert_eq!(collapsed_snapshot.display_map.visible_text, "官网");

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let revealed_snapshot = snapshot(&view, cx);
        let (visible_text, cursor_position) = cx.update_window_entity(&input, |input, _, _| {
            (input.text().to_string(), input.cursor_position())
        });

        assert_eq!(
            revealed_snapshot.display_map.visible_text,
            "[官网](https://example.com/)"
        );
        assert_eq!(revealed_snapshot.visible_selection.cursor(), 4);
        assert_eq!(visible_text, "[官网](https://example.com/)");
        assert_eq!(
            cursor_position,
            Position {
                line: 0,
                character: 2,
            }
        );
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
    fn repeated_enter_after_heading_creates_visible_empty_rows(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# A");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("enter enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "# A\n\n\n\n");
        assert_eq!(snapshot.selection.cursor(), 6);
        assert_eq!(snapshot.display_map.blocks.len(), 2);

        let line_count = cx.update_window_entity(&view, |editor, _, _| {
            rendered_empty_block_line_count(&editor.snapshot.display_map.blocks[1]).unwrap_or(0)
        });
        assert_eq!(line_count, 3);
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                1,
                snapshot.visible_selection.cursor(),
            ),
            Some(1)
        );
    }

    #[gpui::test]
    fn typing_after_single_enter_starts_next_paragraph_without_extra_blank_line(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        cx.simulate_keystrokes("x");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\n\nx");
    }

    #[gpui::test]
    fn enter_at_start_of_second_visible_line_moves_caret_to_third_line_start(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
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

        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\n\n\nB");
        assert_eq!(snapshot.selection.cursor(), 4);
        assert_eq!(snapshot.display_map.blocks.len(), 3);
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                1,
                snapshot.visible_selection.cursor(),
            ),
            None
        );
        assert_eq!(
            caret_visual_offset_for_block(
                &snapshot.display_map.blocks,
                2,
                snapshot.visible_selection.cursor(),
            ),
            Some(0)
        );
    }

    #[gpui::test]
    fn typing_after_enter_at_start_of_second_line_inserts_before_shifted_content(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
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

        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.simulate_keystrokes("x");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\n\n\nxB");
    }

    #[gpui::test]
    fn typing_after_double_enter_stays_on_current_empty_line(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("enter enter");
        cx.run_until_parked();

        cx.simulate_keystrokes("x");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\n\n\nx\n");
    }

    #[gpui::test]
    fn typing_into_collapsed_inter_block_gap_normalizes_to_single_paragraph_between_blocks(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
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
        let blank_snapshot = snapshot(&view, cx);
        assert_eq!(blank_snapshot.selection.cursor(), 3);

        cx.simulate_keystrokes("x");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\n\nx\n\nB");
    }

    #[gpui::test]
    fn backspace_on_collapsed_inter_block_gap_removes_visible_empty_paragraph(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
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

        cx.simulate_keystrokes("backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\nB");
        assert_eq!(snapshot.selection.cursor(), 1);
    }

    #[gpui::test]
    fn delete_on_collapsed_inter_block_gap_removes_visible_empty_paragraph(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
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

        cx.simulate_keystrokes("delete");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "A\nB");
        assert_eq!(snapshot.selection.cursor(), 1);
    }

    #[gpui::test]
    fn shift_selection_across_collapsed_inter_block_gap_survives_snapshot_sync(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\n\n\n\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("shift-down");
        let selected_snapshot = snapshot(&view, cx);
        let input_selection = cx.update_window_entity(&input, |input, window, cx| {
            input
                .selected_text_range(true, window, cx)
                .map(|selection| selection.range)
        });

        assert!(!selected_snapshot.selection.is_collapsed());
        assert!(!selected_snapshot.visible_selection.is_collapsed());
        assert_eq!(selected_snapshot.selection.range(), 1..3);
        assert_eq!(selected_snapshot.visible_selection.range(), 1..3);
        assert_eq!(input_selection, Some(1..3));
    }

    #[gpui::test]
    fn shift_selection_survives_snapshot_sync_on_plain_multiline_text(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "A\nB");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: 0,
                    character: 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        cx.simulate_keystrokes("shift-down");

        let selected_snapshot = snapshot(&view, cx);
        let input_selection = cx.update_window_entity(&input, |input, window, cx| {
            input
                .selected_text_range(true, window, cx)
                .map(|selection| selection.range)
        });

        assert!(!selected_snapshot.selection.is_collapsed());
        assert!(!selected_snapshot.visible_selection.is_collapsed());
        assert!(input_selection.is_some());
        assert!(!input_selection.unwrap_or_default().is_empty());
    }

    #[gpui::test]
    fn empty_document_double_enter_matches_typora_blank_line_count(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "");
        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.run_until_parked();

        cx.simulate_keystrokes("enter enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "\n\n\n\n");
        assert_eq!(snapshot.selection.cursor(), 2);
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
    fn backspace_at_start_of_empty_table_cell_is_consumed_without_mutation(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        let source = concat!(
            "| 1 | 2 | 3 | 4 |\n",
            "| --- | --- | --- | --- |\n",
            "|  |  |  |  |\n",
            "|  |  |  |  |"
        );
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 2, 0));

        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.run_until_parked();

        cx.simulate_keystrokes("backspace backspace");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, source);
        assert_eq!(snapshot.selection.cursor(), table_cell_cursor(source, 2, 0));
        assert_eq!(snapshot.visible_caret_position.line, 2);
        assert_eq!(snapshot.visible_caret_position.column, 0);
        assert_eq!(
            cx.update_window_entity(&input, |input, _, _| input.cursor_position()),
            Position {
                line: 2,
                character: 0,
            }
        );
    }

    #[gpui::test]
    fn ctrl_delete_in_table_removes_current_row(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = concat!(
            "| Name | Role |\n",
            "| --- | --- |\n",
            "| Ada | Eng |\n",
            "| Bob | PM |\n",
            "| Cat | QA |"
        );
        let expected = concat!(
            "| Name | Role |\n",
            "| --- | --- |\n",
            "| Ada | Eng |\n",
            "| Cat | QA |"
        );
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 2, 1));

        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.run_until_parked();

        cx.simulate_keystrokes("ctrl-delete");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(expected, 2, 1)
        );
        assert_eq!(
            cx.update_window_entity(&input, |input, _, _| input.cursor_position()),
            Position {
                line: 2,
                character: 7,
            }
        );
    }

    #[gpui::test]
    fn ctrl_delete_on_last_empty_table_row_removes_row_without_merging_cells(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        let source = concat!(
            "| 1 | 2 | 3 |\n",
            "| --- | --- | --- |\n",
            "| 1 | 2 | 3 |\n",
            "|  |  |  |"
        );
        let expected = concat!("| 1 | 2 | 3 |\n", "| --- | --- | --- |\n", "| 1 | 2 | 3 |");
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 2, 0));

        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.run_until_parked();

        cx.simulate_keystrokes("ctrl-delete");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(expected, 1, 0)
        );
        assert_eq!(
            snapshot
                .display_map
                .visible_text
                .lines()
                .collect::<Vec<_>>(),
            vec!["1   2   3", "1   2   3"]
        );
    }

    #[gpui::test]
    fn ctrl_delete_uses_latest_input_cursor_when_snapshot_selection_is_stale(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        let source = concat!(
            "| 1 | 2 | 3 |\n",
            "| --- | --- | --- |\n",
            "| 1 | 2 | 3 |\n",
            "|  |  |  |"
        );
        let expected = concat!("| 1 | 2 | 3 |\n", "| --- | --- | --- |\n", "| 1 | 2 | 3 |");
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 1, 1));

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

        let handled = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_delete_table_row(window, cx)
        });
        assert!(handled);
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(expected, 1, 0)
        );
    }

    #[gpui::test]
    fn ctrl_enter_exits_table_to_new_empty_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = concat!("| Name | Role |\n", "| --- | --- |\n", "| Ada | Eng |");
        let expected = format!("{source}\n\n");
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 1, 1));

        let input = document_input(&view, cx);
        cx.focus(&input);
        cx.run_until_parked();

        cx.simulate_keystrokes("ctrl-enter");

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(snapshot.selection.cursor(), expected.len());
        assert_eq!(
            cx.update_window_entity(&input, |input, _, _| input.cursor_position()),
            Position {
                line: 3,
                character: 0,
            }
        );
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

    #[gpui::test]
    fn shift_click_extends_selection_to_clicked_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = "# Heading\n\nParagraph";
        load_document(cx, &view, source);

        let (first_bounds, second_bounds) = cx.update_window_entity(&view, |editor, _, _| {
            let first = editor.snapshot.display_map.blocks[0].clone();
            let second = editor.snapshot.display_map.blocks[1].clone();
            let bounds = editor.block_bounds.borrow();
            (
                bounds
                    .get(&first.id)
                    .copied()
                    .expect("first block bounds should exist"),
                bounds
                    .get(&second.id)
                    .copied()
                    .expect("second block bounds should exist"),
            )
        });

        // 先在第一个 block 上按下鼠标，建立 anchor
        cx.simulate_mouse_down(
            point(first_bounds.left() + px(1.), first_bounds.top() + px(8.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();

        let anchor_snapshot = snapshot(&view, cx);
        let anchor_byte = anchor_snapshot.selection.anchor_byte;

        // 再在第二个 block 上按下鼠标，带 shift，应当以第一次 anchor 为起点扩展选区
        cx.simulate_mouse_down(
            point(second_bounds.left() + px(1.), second_bounds.top() + px(8.)),
            MouseButton::Left,
            Modifiers {
                shift: true,
                ..Modifiers::default()
            },
        );
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.selection.anchor_byte, anchor_byte);
        assert!(snapshot.selection.head_byte > anchor_byte);
        assert!(!snapshot.selection.is_collapsed());
    }

    #[gpui::test]
    fn double_click_selects_word_in_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "hello world");

        let target_bounds = cx.update_window_entity(&view, |editor, _, _| {
            let block = editor.snapshot.display_map.blocks[0].clone();
            editor
                .block_bounds
                .borrow()
                .get(&block.id)
                .copied()
                .expect("rendered block bounds should exist")
        });

        let click_position = point(target_bounds.left() + px(8.), target_bounds.top() + px(8.));
        cx.simulate_event(MouseDownEvent {
            position: click_position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position: click_position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 2,
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert!(!snapshot.selection.is_collapsed());
        // "hello" = bytes 0..5 in source ("hello world")
        assert_eq!(snapshot.selection.range(), 0..5);
        assert!(!snapshot.visible_selection.is_collapsed());
    }

    #[gpui::test]
    fn triple_click_selects_entire_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "hello world");

        let target_bounds = cx.update_window_entity(&view, |editor, _, _| {
            let block = editor.snapshot.display_map.blocks[0].clone();
            editor
                .block_bounds
                .borrow()
                .get(&block.id)
                .copied()
                .expect("rendered block bounds should exist")
        });

        let click_position = point(target_bounds.left() + px(8.), target_bounds.top() + px(8.));
        cx.simulate_event(MouseDownEvent {
            position: click_position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
            first_mouse: false,
        });
        cx.simulate_event(MouseUpEvent {
            position: click_position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count: 3,
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert!(!snapshot.selection.is_collapsed());
        assert_eq!(snapshot.selection.range(), 0..11);
        assert_eq!(snapshot.visible_selection.range(), 0..11);
    }

    #[gpui::test]
    fn dragging_across_blocks_updates_selection_range(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "# Heading\n\nParagraph");

        let (first_bounds, second_bounds) = cx.update_window_entity(&view, |editor, _, _| {
            let first = editor.snapshot.display_map.blocks[0].clone();
            let second = editor.snapshot.display_map.blocks[1].clone();
            let bounds = editor.block_bounds.borrow();
            (
                bounds
                    .get(&first.id)
                    .copied()
                    .expect("first block bounds should exist"),
                bounds
                    .get(&second.id)
                    .copied()
                    .expect("second block bounds should exist"),
            )
        });

        cx.simulate_mouse_down(
            point(first_bounds.left() + px(8.), first_bounds.top() + px(8.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        let first_snapshot = snapshot(&view, cx);

        cx.simulate_mouse_move(
            point(second_bounds.left() + px(24.), second_bounds.top() + px(8.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.run_until_parked();
        cx.simulate_mouse_up(
            point(second_bounds.left() + px(24.), second_bounds.top() + px(8.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert!(!snapshot.selection.is_collapsed());
        assert_eq!(snapshot.selection.anchor_byte, first_snapshot.selection.anchor_byte);
        assert!(snapshot.selection.head_byte > first_snapshot.selection.head_byte);
    }

    #[gpui::test]
    fn enter_on_pipe_row_builds_table_and_places_caret_in_first_empty_cell(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        let source = "| Name | Role |";
        let expected = "| Name | Role |\n| --- | --- |\n|  |  |";
        load_document(cx, &view, source);
        set_source_selection(cx, &view, source.len());

        let handled = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_enter(false, window, cx)
        });
        assert!(handled);
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(expected, 1, 0)
        );
        assert!(!snapshot.display_map.visible_text.contains('|'));
        assert!(!snapshot.display_map.visible_text.contains("---"));
    }

    #[gpui::test]
    fn tab_and_shift_tab_navigate_between_table_cells(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 1, 0));

        let moved_forward = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_indent(true, window, cx)
        });
        assert!(moved_forward);
        cx.run_until_parked();
        assert_eq!(
            snapshot(&view, cx).selection.cursor(),
            table_cell_cursor(source, 1, 1)
        );

        let moved_backward = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_indent(false, window, cx)
        });
        assert!(moved_backward);
        cx.run_until_parked();
        assert_eq!(
            snapshot(&view, cx).selection.cursor(),
            table_cell_cursor(source, 1, 0)
        );
    }

    #[gpui::test]
    fn tab_on_last_table_cell_appends_new_row(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        let expected = "| Name | Role |\n| --- | --- |\n| Ada | Eng |\n|  |  |";
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_end_cursor(source, 1, 1));

        let handled = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_indent(true, window, cx)
        });
        assert!(handled);
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(expected, 2, 0)
        );
    }

    #[gpui::test]
    fn enter_and_shift_enter_move_down_table_column_and_append_at_end(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |\n| Bob | CTO |";
        let appended = "| Name | Role |\n| --- | --- |\n| Ada | Eng |\n| Bob | CTO |\n|  |  |";
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_cursor(source, 1, 1));

        let enter_handled = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_enter(false, window, cx)
        });
        assert!(enter_handled);
        cx.run_until_parked();
        assert_eq!(snapshot(&view, cx).document_text, source);
        assert_eq!(
            snapshot(&view, cx).selection.cursor(),
            table_cell_cursor(source, 2, 1)
        );

        let shift_enter_handled = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_enter(true, window, cx)
        });
        assert!(shift_enter_handled);
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, appended);
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(appended, 3, 0)
        );
        let input = document_input(&view, cx);
        assert_eq!(
            cx.update_window_entity(&input, |input, _, _| input.cursor_position()),
            Position {
                line: 3,
                character: 0,
            }
        );
    }

    #[gpui::test]
    fn moving_right_from_table_cell_end_enters_next_cell(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = "| Name | Role |\n| --- | --- |\n| Ada | Eng |";
        load_document(cx, &view, source);
        let first_cell_end = table_cell_end_cursor(source, 1, 0);
        cx.update_window_entity(&view, |editor, window, cx| {
            let effects = editor.controller.dispatch(EditCommand::SetSelection {
                selection: SelectionState {
                    anchor_byte: first_cell_end,
                    head_byte: first_cell_end,
                    preferred_column: None,
                    affinity: SelectionAffinity::Upstream,
                },
            });
            editor.apply_effects(window, cx, effects);
        });
        cx.run_until_parked();

        let input = document_input(&view, cx);
        cx.focus(&input);
        let start_position = cx.update_window_entity(&input, |input, _, _| input.cursor_position());
        cx.update_window_entity(&input, |input, window, cx| {
            input.set_cursor_position(
                Position {
                    line: start_position.line,
                    character: start_position.character + 1,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.selection.cursor(), table_cell_cursor(source, 1, 1));
        assert_eq!(snapshot.selection.affinity, SelectionAffinity::Upstream);
    }

    #[gpui::test]
    fn enter_after_multiple_empty_table_rows_keeps_single_table_block(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        let source = concat!(
            "| 1 | 2 | 3 |\n",
            "| --- | --- | --- |\n",
            "| 1 | 2 | 3 |\n",
            "|  |  |  |\n",
            "|  |  |  |"
        );
        let expected = concat!(
            "| 1 | 2 | 3 |\n",
            "| --- | --- | --- |\n",
            "| 1 | 2 | 3 |\n",
            "|  |  |  |\n",
            "|  |  |  |\n",
            "|  |  |  |"
        );
        load_document(cx, &view, source);
        set_source_selection(cx, &view, table_cell_end_cursor(source, 3, 2));

        let handled = cx.update_window_entity(&view, |editor, window, cx| {
            editor.handle_enter(false, window, cx)
        });
        assert!(handled);
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, expected);
        assert_eq!(
            snapshot
                .display_map
                .blocks
                .iter()
                .filter(|block| block.kind == BlockKind::Table)
                .count(),
            1
        );
        assert_eq!(
            snapshot.selection.cursor(),
            table_cell_cursor(expected, 4, 0)
        );
    }

    fn assert_visible_edit_round_trip(
        cx: &mut TestAppContext,
        source_text: &str,
        expected_visible_text: &str,
        edited_visible_text: &str,
        expected_source_text: &str,
        expected_final_visible_text: &str,
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
        assert_eq!(
            snapshot.display_map.visible_text,
            expected_final_visible_text
        );
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

    fn set_source_selection(
        cx: &mut VisualTestContext,
        view: &Entity<MarkdownEditor>,
        cursor: usize,
    ) {
        cx.update_window_entity(view, |editor, window, cx| {
            let effects = editor.controller.dispatch(EditCommand::SetSelection {
                selection: SelectionState::collapsed(cursor),
            });
            editor.apply_effects(window, cx, effects);
        });
        cx.run_until_parked();
    }

    fn table_cell_cursor(source: &str, visible_row: usize, column: usize) -> usize {
        crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row,
                column,
            })
            .expect("table cell")
            .start
    }

    fn table_cell_end_cursor(source: &str, visible_row: usize, column: usize) -> usize {
        crate::core::table::TableModel::parse(source)
            .cell_source_range(crate::core::table::TableCellRef {
                visible_row,
                column,
            })
            .expect("table cell")
            .end
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
