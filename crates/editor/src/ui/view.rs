use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AppContext, Bounds, ClickEvent, Context, Entity, EntityInputHandler as _,
    EventEmitter, FontStyle, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    PaintQuad, Render, SharedString, StatefulInteractiveElement, StrikethroughStyle, Styled,
    StyledText, Subscription, TextStyle, UnderlineStyle, VisualContext, WhiteSpace, Window,
    canvas, div, fill, point, px, size,
};
use gpui_component::{
    ActiveTheme,
    input::{
        Backspace, Delete, Enter, IndentInline, Input, InputEvent, InputState, OutdentInline,
        Position,
    },
    scroll::ScrollableElement,
};

use crate::{
    BlockKind, EditCommand, RenderBlock, RenderSpan, RenderSpanKind, SelectionAffinity,
    SelectionState,
    core::controller::{DocumentSource, EditorController, EditorSnapshot, SyncPolicy},
};

use super::{
    BODY_LINE_HEIGHT, BODY_FONT_SIZE, EDITOR_CONTEXT, MAX_EDITOR_WIDTH, layout::block_presentation,
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
    syncing_input: bool,
    input_focused: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
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

    pub(super) fn sync_input_from_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = self.snapshot.clone();
        self.syncing_input = true;
        self.document_input.update(cx, |input, cx| {
            let current_text = input.text().to_string();
            if current_text != snapshot.display_map.visible_text {
                input.set_value(snapshot.display_map.visible_text.clone(), window, cx);
            }

            if input.cursor() != snapshot.visible_selection.cursor() {
                input.set_cursor_position(
                    Position {
                        line: snapshot.visible_caret_position.line as u32,
                        character: snapshot.visible_caret_position.column as u32,
                    },
                    window,
                    cx,
                );
            }
        });
        self.syncing_input = false;
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
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let effects = self.controller.dispatch(EditCommand::ToggleInlineMarkup {
            before: before.to_string(),
            after: after.to_string(),
            placeholder: placeholder.to_string(),
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
            selection: crate::SelectionState::collapsed(self.snapshot.selection.cursor()),
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

    fn handle_input_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.syncing_input {
            return;
        }

        match event {
            InputEvent::Change => self.sync_from_input(window, cx, true),
            InputEvent::Focus => {
                self.input_focused = true;
                self.sync_from_input(window, cx, false);
                cx.notify();
            }
            InputEvent::Blur => {
                self.input_focused = false;
                self.sync_from_input(window, cx, false);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => {}
        }
    }

    fn handle_observed_input_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.syncing_input {
            return;
        }

        self.sync_from_input(window, cx, false);
    }

    fn sync_from_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        allow_autosave: bool,
    ) {
        let Some((visible_text, visible_selection)) = self.read_input_state(window, cx) else {
            return;
        };

        let effects = if visible_text != self.snapshot.display_map.visible_text {
            let Some((text, selection)) =
                reconcile_visible_input_change(&self.snapshot, &visible_text)
            else {
                return;
            };
            self.controller
                .dispatch(EditCommand::SyncDocumentState { text, selection })
        } else if visible_selection != self.snapshot.visible_selection {
            let selection = self
                .snapshot
                .display_map
                .visible_selection_to_source(&visible_selection);
            self.controller
                .dispatch(EditCommand::SetSelection { selection })
        } else {
            return;
        };

        if allow_autosave && effects.changed {
            self.schedule_autosave(window, cx);
        }
        self.apply_effects(window, cx, effects);
    }

    fn read_input_state(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(String, SelectionState)> {
        self.document_input.update(cx, |input, cx| {
            if input.marked_text_range(window, cx).is_some() {
                return None;
            }

            let text = input.text().to_string();
            let cursor = input.cursor();
            let preferred_column = Some(input.cursor_position().character as usize);
            let selection = selection_from_input(
                &text,
                input
                    .selected_text_range(true, window, cx)
                    .map(|selection| selection.range),
                cursor,
                preferred_column,
            );
            Some((text, selection))
        })
    }

    fn input_has_marked_text(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.document_input.update(cx, |input, cx| {
            input.marked_text_range(window, cx).is_some()
        })
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

    fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.document_input.update(cx, |input, cx| {
            input.focus(window, cx);
        });
        self.input_focused = true;
    }

    fn handle_surface_click(
        &mut self,
        block_id: u64,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = self.block_bounds.borrow().get(&block_id).copied() else {
            self.focus_input(window, cx);
            return;
        };
        let Some(block) = self
            .snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| block.id == block_id)
            .cloned()
        else {
            self.focus_input(window, cx);
            return;
        };

        let local_visible_offset =
            visible_byte_offset_for_click_position(&block, event.position(), bounds, window);
        let visible_offset = clamp_to_char_boundary(
            &self.snapshot.display_map.visible_text,
            block.visible_range.start + local_visible_offset,
        );
        let selection = SelectionState::collapsed(
            self.snapshot
                .display_map
                .visible_to_source(visible_offset)
                .source_offset,
        );
        let effects = self.controller.dispatch(EditCommand::SetSelection { selection });
        self.apply_effects(window, cx, effects);
        self.focus_input(window, cx);
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
                move |action: &Enter, window, app| {
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
                move |_: &Backspace, window, app| {
                    let handled =
                        view.update(app, |this, cx| this.handle_delete_backward(window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &Delete, window, app| {
                    let handled =
                        view.update(app, |this, cx| this.handle_delete_forward(window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &IndentInline, window, app| {
                    let handled = view.update(app, |this, cx| this.handle_indent(true, window, cx));
                    if handled {
                        app.stop_propagation();
                    }
                }
            })
            .capture_action({
                let view = cx.entity();
                move |_: &OutdentInline, window, app| {
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

fn build_document_input(
    text: &str,
    window: &mut Window,
    cx: &mut Context<InputState>,
) -> InputState {
    let mut state = InputState::new(window, cx)
        .auto_grow(1, 4096)
        .soft_wrap(true)
        .placeholder("Start writing...");
    state.set_value(text.to_string(), window, cx);
    state
}

#[derive(Debug, Clone, Copy)]
struct RenderPalette {
    text_color: Hsla,
    muted_text_color: Hsla,
    selection_color: Hsla,
    caret_color: Hsla,
    code_background: Hsla,
    border_color: Hsla,
    blockquote_bar: Hsla,
    code_surface_background: Hsla,
}

#[derive(Clone)]
struct BlockOverlay {
    selection_quads: Vec<PaintQuad>,
    caret_quad: Option<PaintQuad>,
}

fn rendered_spans(block: &RenderBlock) -> impl Iterator<Item = &RenderSpan> {
    block
        .spans
        .iter()
        .filter(move |span| is_rendered_span(block, span))
}

fn is_rendered_span(block: &RenderBlock, span: &RenderSpan) -> bool {
    span.source_range.start < block.content_range.end
        && !(span.kind == RenderSpanKind::LineBreak
            && span.source_range.end == block.content_range.end)
}

fn rendered_visible_end(block: &RenderBlock) -> usize {
    rendered_spans(block)
        .map(|span| span.visible_range.end)
        .max()
        .unwrap_or(block.visible_range.start)
}

fn rendered_visible_len(block: &RenderBlock) -> usize {
    rendered_visible_end(block).saturating_sub(block.visible_range.start)
}

fn has_rendered_text(block: &RenderBlock) -> bool {
    rendered_visible_len(block) > 0
}

fn rendered_text_for_block(block: &RenderBlock) -> String {
    rendered_spans(block)
        .filter(|span| !span.visible_text.is_empty())
        .map(|span| span.visible_text.as_str())
        .collect()
}

fn render_document_surface(
    view: &Entity<MarkdownEditor>,
    snapshot: &EditorSnapshot,
    input_focused: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
    window: &mut Window,
    cx: &mut Context<MarkdownEditor>,
) -> AnyElement {
    let display_blocks = Rc::new(snapshot.display_map.blocks.clone());
    let palette = RenderPalette {
        text_color: cx.theme().foreground,
        muted_text_color: cx.theme().muted_foreground,
        selection_color: cx.theme().foreground.opacity(0.14),
        caret_color: cx.theme().foreground,
        code_background: cx.theme().foreground.opacity(0.08),
        border_color: cx.theme().foreground.opacity(0.12),
        blockquote_bar: cx.theme().foreground.opacity(0.18),
        code_surface_background: cx.theme().foreground.opacity(0.04),
    };

    if snapshot.display_map.blocks.is_empty() {
        let empty_view = view.clone();
        return div()
            .id("empty-surface")
            .w_full()
            .min_h(px(BODY_LINE_HEIGHT))
            .text_size(px(BODY_FONT_SIZE))
            .line_height(px(BODY_LINE_HEIGHT))
            .text_color(cx.theme().muted_foreground)
            .on_click(move |_, window, cx| {
                let _ = empty_view.update(cx, |this, cx| {
                    let effects = this.controller.dispatch(EditCommand::SetSelection {
                        selection: SelectionState::collapsed(0),
                    });
                    this.apply_effects(window, cx, effects);
                    this.focus_input(window, cx);
                });
            })
            .child("Start writing...")
            .into_any_element();
    }

    let mut document = div().w_full().flex().flex_col();
    for (block_index, block) in snapshot.display_map.blocks.iter().cloned().enumerate() {
        document = document.child(render_display_block(
            view,
            snapshot,
            display_blocks.clone(),
            block_index,
            &block,
            input_focused,
            block_bounds.clone(),
            palette,
            window,
            cx,
        ));
    }
    document.into_any_element()
}

fn render_display_block(
    view: &Entity<MarkdownEditor>,
    snapshot: &EditorSnapshot,
    display_blocks: Rc<Vec<RenderBlock>>,
    block_index: usize,
    block: &RenderBlock,
    input_focused: bool,
    block_bounds: Rc<RefCell<HashMap<u64, Bounds<gpui::Pixels>>>>,
    palette: RenderPalette,
    window: &mut Window,
    _cx: &mut Context<MarkdownEditor>,
) -> AnyElement {
    let presentation = block_presentation(&block.kind);
    let show_placeholder = snapshot.display_map.blocks.len() == 1
        && snapshot.display_map.visible_text.is_empty()
        && !input_focused;
    let visible_selection = snapshot.visible_selection.clone();
    let block_id = block.id;
    let block_clone = block.clone();
    let overlay_block = block.clone();
    let block_view = view.clone();

    let text_area = div()
        .relative()
        .min_w(px(0.))
        .w_full()
        .min_h(px(presentation.line_height))
        .child(
            div()
                .w_full()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .when(
                    matches!(block.kind, BlockKind::CodeFence { .. }),
                    |this| this.font_family("Consolas"),
                )
                .when(show_placeholder, |this| {
                    this.text_color(palette.muted_text_color)
                        .child("Start writing...")
                })
                .when(!show_placeholder, |this| {
                    this.child(styled_text_for_block(&block_clone, palette, window))
                }),
        )
        .child(
            canvas(
                move |bounds, window, _| {
                    block_bounds.borrow_mut().insert(overlay_block.id, bounds);
                    build_block_overlay(
                        &display_blocks,
                        block_index,
                        &overlay_block,
                        &visible_selection,
                        input_focused,
                        bounds,
                        palette,
                        window,
                    )
                },
                move |_, overlay, window, _| {
                    for quad in overlay.selection_quads {
                        window.paint_quad(quad);
                    }
                    if let Some(caret) = overlay.caret_quad {
                        window.paint_quad(caret);
                    }
                },
            )
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .right(px(0.))
            .bottom(px(0.)),
        )
        .into_any_element();

    let content = match &block.kind {
        BlockKind::Blockquote => div()
            .w_full()
            .flex()
            .gap_3()
            .items_start()
            .child(
                div()
                    .w(px(3.))
                    .h_full()
                    .min_h(px(presentation.line_height))
                    .rounded(px(999.))
                    .bg(palette.blockquote_bar),
            )
            .child(text_area)
            .into_any_element(),
        BlockKind::List => {
            if let Some(marker) = list_decoration_text(block) {
                div()
                    .w_full()
                    .flex()
                    .gap_3()
                    .items_start()
                    .child(
                        div()
                            .min_w(px(28.))
                            .text_color(palette.muted_text_color)
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(px(BODY_FONT_SIZE))
                            .line_height(px(BODY_LINE_HEIGHT))
                            .child(marker),
                    )
                    .child(text_area)
                    .into_any_element()
            } else {
                text_area
            }
        }
        BlockKind::CodeFence { language } => {
            let mut code_surface = div()
                .w_full()
                .rounded(px(8.))
                .border_1()
                .border_color(palette.border_color)
                .bg(palette.code_surface_background)
                .px_3()
                .py_2()
                .child(text_area);

            if let Some(language) = language.as_ref().filter(|language| !language.is_empty()) {
                code_surface = div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(palette.muted_text_color)
                            .child(language.clone()),
                    )
                    .child(code_surface);
            }

            code_surface.into_any_element()
        }
        _ => text_area,
    };

    div()
        .id(("display-block", block.id))
        .w_full()
        .py(px(presentation.row_spacing_y))
        .child(
            div()
                .id(("surface-hit-target", block_id))
                .w_full()
                .px_1()
                .py(px(presentation.block_padding_y))
                .on_click(move |event, window, cx| {
                    let _ = block_view.update(cx, |this, cx| {
                        this.handle_surface_click(block_id, event, window, cx);
                    });
                })
                .child(content),
        )
        .into_any_element()
}

fn build_block_overlay(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    selection: &SelectionState,
    input_focused: bool,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> BlockOverlay {
    let selection_range = selection_range_in_block(block, selection);
    let selection_quads = selection_range
        .filter(|range| !range.is_empty())
        .map(|range| selection_quads_for_block(block, range, bounds, palette, window))
        .unwrap_or_default();
    let caret_quad = if input_focused && selection.is_collapsed() {
        caret_quad_for_block(blocks, block_index, block, selection.cursor(), bounds, palette, window)
    } else {
        None
    };

    BlockOverlay {
        selection_quads,
        caret_quad,
    }
}

fn selection_range_in_block(
    block: &RenderBlock,
    selection: &SelectionState,
) -> Option<std::ops::Range<usize>> {
    let selection = selection.range();
    let start = selection.start.max(block.visible_range.start);
    let end = selection.end.min(rendered_visible_end(block));
    (start < end).then(|| {
        start.saturating_sub(block.visible_range.start)..end.saturating_sub(block.visible_range.start)
    })
}

fn selection_quads_for_block(
    block: &RenderBlock,
    selection_range: std::ops::Range<usize>,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> Vec<PaintQuad> {
    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let lines = shape_block_lines(block, bounds.size.width, window);
    if lines.is_empty() {
        return Vec::new();
    }

    let mut quads = Vec::new();
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);
    for (line_ix, line) in lines.iter().enumerate() {
        let line_len = line.len();
        let line_range = byte_offset..byte_offset + line_len;
        if selection_range.start < line_range.end && line_range.start < selection_range.end {
            let local_start = selection_range.start.saturating_sub(line_range.start);
            let local_end = (selection_range.end.min(line_range.end)).saturating_sub(line_range.start);
            let mut wrap_start = 0usize;
            let mut wrap_y = px(0.);

            for wrap_end in line
                .wrap_boundaries()
                .iter()
                .map(|boundary| wrap_boundary_index(line, boundary))
                .chain(std::iter::once(line.len()))
            {
                let start = local_start.max(wrap_start);
                let end = local_end.min(wrap_end);
                if start < end {
                    let start_position = line
                        .position_for_index(start, line_height)
                        .unwrap_or_else(|| point(px(0.), wrap_y));
                    let end_position = line
                        .position_for_index(end, line_height)
                        .unwrap_or_else(|| point(px(0.), wrap_y));
                    quads.push(fill(
                        Bounds::new(
                            point(
                                bounds.left() + start_position.x,
                                bounds.top() + y_offset + start_position.y,
                            ),
                            size(
                                (end_position.x - start_position.x).max(px(1.)),
                                line_height,
                            ),
                        ),
                        palette.selection_color,
                    ));
                }
                wrap_start = wrap_end;
                wrap_y += line_height;
            }
        }

        y_offset += line.size(line_height).height;
        byte_offset += line_len;
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    quads
}

fn caret_visual_offset_for_block(
    blocks: &[RenderBlock],
    block_index: usize,
    cursor: usize,
) -> Option<usize> {
    let block = blocks.get(block_index)?;
    let block_start = block.visible_range.start;
    let block_end = rendered_visible_end(block);
    let previous_end = block_index
        .checked_sub(1)
        .and_then(|index| blocks.get(index))
        .map(rendered_visible_end)
        .unwrap_or(block_start);

    if !has_rendered_text(block) {
        return (cursor == block_start || (cursor > previous_end && cursor < block_start))
            .then_some(0);
    }

    if cursor < block_start {
        return (cursor > previous_end && cursor <= block_start).then_some(0);
    }

    if cursor < block_end {
        return Some(cursor.saturating_sub(block_start));
    }

    if cursor == block_end {
        if blocks
            .get(block_index + 1)
            .map(|next| next.visible_range.start == cursor)
            .unwrap_or(false)
        {
            return None;
        }

        return Some(block_end.saturating_sub(block_start));
    }

    None
}

fn caret_quad_for_block(
    blocks: &[RenderBlock],
    block_index: usize,
    block: &RenderBlock,
    cursor: usize,
    bounds: Bounds<gpui::Pixels>,
    palette: RenderPalette,
    window: &mut Window,
) -> Option<PaintQuad> {
    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let local_cursor = caret_visual_offset_for_block(blocks, block_index, cursor)?;

    if !has_rendered_text(block) {
        return Some(fill(
            Bounds::new(bounds.origin, size(px(2.), line_height)),
            palette.caret_color,
        ));
    }

    let lines = shape_block_lines(block, bounds.size.width, window);
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);

    for (line_ix, line) in lines.iter().enumerate() {
        let line_len = line.len();
        if local_cursor <= byte_offset + line_len {
            let local = local_cursor.saturating_sub(byte_offset);
            let position = line
                .position_for_index(local, line_height)
                .unwrap_or_else(|| point(px(0.), px(0.)));
            return Some(fill(
                Bounds::new(
                    point(
                        bounds.left() + position.x,
                        bounds.top() + y_offset + position.y,
                    ),
                    size(px(2.), line_height),
                ),
                palette.caret_color,
            ));
        }

        let line_height_span = line.size(line_height).height;
        if line_ix + 1 < lines.len() && local_cursor == byte_offset + line_len + 1 {
            return Some(fill(
                Bounds::new(
                    point(bounds.left(), bounds.top() + y_offset + line_height_span),
                    size(px(2.), line_height),
                ),
                palette.caret_color,
            ));
        }

        y_offset += line_height_span;
        byte_offset += line_len;
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    lines.last().and_then(|line| {
        line.position_for_index(line.len(), line_height).map(|position| {
            fill(
                Bounds::new(
                    point(
                        bounds.left() + position.x,
                        bounds.top() + y_offset - line.size(line_height).height + position.y,
                    ),
                    size(px(2.), line_height),
                ),
                palette.caret_color,
            )
        })
    })
}

fn styled_text_for_block(
    block: &RenderBlock,
    palette: RenderPalette,
    window: &Window,
) -> StyledText {
    let text = rendered_text_for_block(block);
    if text.is_empty() {
        return StyledText::new(String::new());
    }

    let mut runs = Vec::new();
    let base_style = base_text_style_for_block(block, palette.text_color, window);

    for span in rendered_spans(block).filter(|span| !span.visible_text.is_empty()) {
        let mut style = base_style.clone();
        apply_span_style(&mut style, span, palette);
        runs.push(style.to_run(span.visible_text.len()));
    }

    StyledText::new(text).with_runs(runs)
}

fn apply_span_style(
    style: &mut TextStyle,
    span: &RenderSpan,
    palette: RenderPalette,
) {
    if matches!(span.kind, RenderSpanKind::HiddenSyntax | RenderSpanKind::ListMarker) {
        style.color = palette.muted_text_color;
    }
    if matches!(span.kind, RenderSpanKind::TaskMarker) {
        style.font_weight = FontWeight::MEDIUM;
        style.color = palette.muted_text_color;
    }

    if span.style.strong {
        style.font_weight = FontWeight::BOLD;
    }
    if span.style.emphasis {
        style.font_style = FontStyle::Italic;
    }
    if span.style.strikethrough {
        style.strikethrough = Some(StrikethroughStyle {
            thickness: px(1.),
            color: Some(palette.text_color.opacity(0.68)),
        });
    }
    if span.style.link {
        style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            color: Some(palette.text_color.opacity(0.7)),
            wavy: false,
        });
    }
    if span.style.code {
        if !span.style.strong {
            style.font_weight = FontWeight::MEDIUM;
        }
        style.font_family = SharedString::from("Consolas");
        style.background_color = Some(palette.code_background);
    }
}

fn base_text_style_for_block(block: &RenderBlock, text_color: Hsla, window: &Window) -> TextStyle {
    let presentation = block_presentation(&block.kind);
    let mut style = window.text_style().clone();
    style.color = text_color;
    style.font_size = px(presentation.font_size).into();
    style.line_height = px(presentation.line_height).into();
    style.font_weight = match block.kind {
        BlockKind::Heading { depth } if depth <= 2 => FontWeight::BOLD,
        BlockKind::Heading { .. } => FontWeight::SEMIBOLD,
        _ => FontWeight::NORMAL,
    };
    style.font_style = FontStyle::Normal;
    style.white_space = WhiteSpace::Normal;
    if matches!(block.kind, BlockKind::CodeFence { .. }) {
        style.font_family = SharedString::from("Consolas");
    }
    style
}

fn list_decoration_text(block: &RenderBlock) -> Option<String> {
    let marker = block
        .spans
        .iter()
        .find(|span| {
            span.kind == RenderSpanKind::ListMarker
                && span.hidden
                && span.visible_text.is_empty()
                && !span.source_text.trim().is_empty()
        })?
        .source_text
        .trim()
        .to_string();

    if marker.chars().all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ')')) {
        Some(marker)
    } else {
        Some("•".to_string())
    }
}

fn visible_byte_offset_for_click_position(
    block: &RenderBlock,
    click_position: gpui::Point<gpui::Pixels>,
    bounds: Bounds<gpui::Pixels>,
    window: &Window,
) -> usize {
    if !has_rendered_text(block) {
        return 0;
    }

    if bounds.size.width <= px(0.) {
        return rendered_visible_len(block);
    }

    let presentation = block_presentation(&block.kind);
    let line_height = px(presentation.line_height);
    let mut local = click_position - bounds.origin;
    local.x = local.x.max(px(0.));
    local.y = local.y.max(px(0.));

    let lines = shape_block_lines(block, bounds.size.width, window);
    let mut byte_offset = 0usize;
    let mut y_offset = px(0.);
    for (line_ix, line) in lines.iter().enumerate() {
        let line_height_span = line.size(line_height).height;
        if local.y <= y_offset + line_height_span {
            let position = point(local.x, (local.y - y_offset).max(px(0.)));
            let local_offset = match line.closest_index_for_position(position, line_height) {
                Ok(offset) | Err(offset) => offset,
            };
            return (byte_offset + local_offset).min(rendered_visible_len(block));
        }

        y_offset += line_height_span;
        byte_offset += line.len();
        if line_ix + 1 < lines.len() {
            byte_offset += 1;
        }
    }

    rendered_visible_len(block)
}

fn shape_block_lines(
    block: &RenderBlock,
    width: gpui::Pixels,
    window: &Window,
) -> Vec<gpui::WrappedLine> {
    if !has_rendered_text(block) {
        return Vec::new();
    }

    let base_style = base_text_style_for_block(block, window.text_style().color, window);
    let rendered_text = rendered_text_for_block(block);
    let runs = rendered_spans(block)
        .filter(|span| !span.visible_text.is_empty())
        .map(|span| {
            let mut style = base_style.clone();
            if span.style.strong {
                style.font_weight = FontWeight::BOLD;
            }
            if span.style.emphasis {
                style.font_style = FontStyle::Italic;
            }
            if span.style.code {
                style.font_family = SharedString::from("Consolas");
                if !span.style.strong {
                    style.font_weight = FontWeight::MEDIUM;
                }
            }
            style.to_run(span.visible_text.len())
        })
        .collect::<Vec<_>>();

    window
        .text_system()
        .shape_text(
            rendered_text.into(),
            base_style.font_size.to_pixels(window.rem_size()),
            &runs,
            Some(width),
            None,
        )
        .unwrap_or_default()
        .to_vec()
}

fn wrap_boundary_index(line: &gpui::WrappedLine, boundary: &gpui::WrapBoundary) -> usize {
    let run = &line.runs()[boundary.run_ix];
    let glyph = &run.glyphs[boundary.glyph_ix];
    glyph.index
}

fn selection_from_input(
    text: &str,
    selection_utf16: Option<std::ops::Range<usize>>,
    cursor_byte: usize,
    preferred_column: Option<usize>,
) -> SelectionState {
    let cursor_byte = clamp_to_char_boundary(text, cursor_byte);
    let Some(selection_utf16) = selection_utf16 else {
        let mut selection = SelectionState::collapsed(cursor_byte);
        selection.preferred_column = preferred_column;
        return selection;
    };

    let range = utf16_range_to_byte_range(text, &selection_utf16);
    if range.is_empty() {
        let mut selection = SelectionState::collapsed(cursor_byte);
        selection.preferred_column = preferred_column;
        return selection;
    }

    SelectionState {
        anchor_byte: if cursor_byte == range.start {
            range.end
        } else {
            range.start
        },
        head_byte: cursor_byte,
        preferred_column,
        affinity: if cursor_byte == range.start {
            SelectionAffinity::Upstream
        } else {
            SelectionAffinity::Downstream
        },
    }
}

fn utf16_range_to_byte_range(text: &str, range: &std::ops::Range<usize>) -> std::ops::Range<usize> {
    utf16_offset_to_byte_offset(text, range.start)..utf16_offset_to_byte_offset(text, range.end)
}

fn utf16_offset_to_byte_offset(text: &str, target: usize) -> usize {
    if target == 0 {
        return 0;
    }

    let mut utf16_offset = 0usize;
    for (byte_offset, ch) in text.char_indices() {
        if utf16_offset >= target {
            return byte_offset;
        }
        utf16_offset += ch.len_utf16();
        if utf16_offset >= target {
            return byte_offset + ch.len_utf8();
        }
    }

    text.len()
}

fn clamp_to_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn reconcile_visible_input_change(
    snapshot: &EditorSnapshot,
    visible_text: &str,
) -> Option<(String, SelectionState)> {
    let (visible_range, replacement) =
        compute_document_diff(&snapshot.display_map.visible_text, visible_text)?;
    let source_range = snapshot
        .display_map
        .visible_selection_to_source(&SelectionState {
            anchor_byte: visible_range.start,
            head_byte: visible_range.end,
            preferred_column: None,
            affinity: SelectionAffinity::Downstream,
        })
        .range();

    let mut source_text = snapshot.document_text.clone();
    source_text.replace_range(source_range.clone(), &replacement);

    Some((
        source_text,
        SelectionState::collapsed(source_range.start + replacement.len()),
    ))
}

fn compute_document_diff(old: &str, new: &str) -> Option<(std::ops::Range<usize>, String)> {
    if old == new {
        return None;
    }

    let mut prefix = common_prefix_len(old.as_bytes(), new.as_bytes());
    while prefix > 0 && (!old.is_char_boundary(prefix) || !new.is_char_boundary(prefix)) {
        prefix -= 1;
    }

    let old_remaining = &old.as_bytes()[prefix..];
    let new_remaining = &new.as_bytes()[prefix..];
    let mut suffix = common_suffix_len(old_remaining, new_remaining);
    while suffix > 0 {
        let old_start = old.len().saturating_sub(suffix);
        let new_start = new.len().saturating_sub(suffix);
        if old.is_char_boundary(old_start) && new.is_char_boundary(new_start) {
            break;
        }
        suffix -= 1;
    }

    let old_suffix_start = old.len().saturating_sub(suffix);
    let new_suffix_start = new.len().saturating_sub(suffix);
    Some((prefix..old_suffix_start, new[prefix..new_suffix_start].to_string()))
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

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::PathBuf, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext, VisualContext, VisualTestContext};
    use gpui_component::Root;

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
    fn list_enter_shows_single_caret_in_empty_following_block() {
        let text = "- item\n\n";
        let snapshot = snapshot_for_text_with_selection(text, text.len());
        let blocks = &snapshot.display_map.blocks;

        assert_eq!(blocks.len(), 2);
        assert_eq!(rendered_text_for_block(&blocks[0]), "item");
        assert_eq!(rendered_text_for_block(&blocks[1]), "");
        assert_eq!(snapshot.visible_selection.cursor(), 6);
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
    fn blockquote_enter_shows_single_caret_in_empty_following_block() {
        let text = "> Quote\n\n";
        let snapshot = snapshot_for_text_with_selection(text, text.len());
        let blocks = &snapshot.display_map.blocks;

        assert_eq!(blocks.len(), 2);
        assert_eq!(rendered_text_for_block(&blocks[0]), "Quote");
        assert_eq!(rendered_text_for_block(&blocks[1]), "");
        assert_eq!(snapshot.visible_selection.cursor(), 7);
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
    fn caret_at_block_boundary_is_painted_by_only_one_block() {
        let snapshot = snapshot_for_text_with_selection("First\n\n", 7);
        let blocks = &snapshot.display_map.blocks;
        let owners = blocks
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                caret_visual_offset_for_block(blocks, *index, snapshot.visible_selection.cursor())
                    .is_some()
            })
            .count();

        assert_eq!(owners, 1);
        assert_eq!(rendered_text_for_block(&blocks[0]), "First");
    }

    #[test]
    fn line_break_spans_remain_mapped_but_not_rendered_for_heading_block() {
        let snapshot = snapshot_for_text_with_selection("# H1\n\n## H2", 6);
        let heading = &snapshot.display_map.blocks[0];

        assert!(heading
            .spans
            .iter()
            .any(|span| span.kind == RenderSpanKind::LineBreak));
        assert_eq!(rendered_text_for_block(heading), "H1");
        assert_eq!(rendered_visible_end(heading), 2);
    }

    #[test]
    fn line_break_spans_remain_mapped_but_not_rendered_for_list_block() {
        let snapshot = snapshot_for_text_with_selection("- item\n\nNext", 8);
        let list = &snapshot.display_map.blocks[0];

        assert!(list
            .spans
            .iter()
            .any(|span| span.kind == RenderSpanKind::LineBreak));
        assert_eq!(rendered_text_for_block(list), "item");
        assert_eq!(rendered_visible_end(list), 4);
    }

    #[test]
    fn line_break_spans_remain_mapped_but_not_rendered_for_blockquote_block() {
        let snapshot = snapshot_for_text_with_selection("> Quote\n\nNext", 9);
        let blockquote = &snapshot.display_map.blocks[0];

        assert!(blockquote
            .spans
            .iter()
            .any(|span| span.kind == RenderSpanKind::LineBreak));
        assert_eq!(rendered_text_for_block(blockquote), "Quote");
        assert_eq!(rendered_visible_end(blockquote), 5);
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

        let visible_text =
            cx.update_window_entity(&input, |input, _, _| input.text().to_string());
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
    fn single_surface_preserves_inline_markup_when_editing_visible_text(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "Hello **world**");
        let input = document_input(&view, cx);

        let visible_text =
            cx.update_window_entity(&input, |input, _, _| input.text().to_string());
        assert_eq!(visible_text, "Hello world");

        cx.update_window_entity(&input, |input, window, cx| {
            input.set_value("Hello there".to_string(), window, cx);
        });
        cx.run_until_parked();

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.document_text, "Hello **there**");
        assert_eq!(snapshot.display_map.visible_text, "Hello there");
    }

    #[gpui::test]
    fn single_surface_hides_list_prefix_but_preserves_source_markup(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "- item");
        let input = document_input(&view, cx);

        assert_eq!(snapshot(&view, cx).display_map.visible_text, "item");

        let visible_text =
            cx.update_window_entity(&input, |input, _, _| input.text().to_string());
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
    fn single_surface_hides_blockquote_prefix_but_preserves_source_markup(
        cx: &mut TestAppContext,
    ) {
        assert_visible_edit_round_trip(cx, "> quote", "quote", "changed", "> changed");
    }

    #[gpui::test]
    fn single_surface_renders_task_marker_as_checkbox_and_preserves_markdown(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "- [ ] task");
        let input = document_input(&view, cx);

        assert_eq!(snapshot(&view, cx).display_map.visible_text, "\u{2610} task");
        let visible_text =
            cx.update_window_entity(&input, |input, _, _| input.text().to_string());
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
                    line: 0,
                    character: 0,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let (visible_text, cursor) = cx.update_window_entity(&input, |input, _, _| {
            (input.text().to_string(), input.cursor())
        });
        assert_eq!(visible_text, "# Title");
        assert_eq!(cursor, 2);

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.selection.cursor(), 2);
        assert_eq!(snapshot.visible_selection.cursor(), 2);
    }

    #[gpui::test]
    fn moving_to_start_of_hidden_blockquote_reveals_marker_boundary(cx: &mut TestAppContext) {
        let (view, cx) = build_editor_window(cx);
        load_document(cx, &view, "> Quote");
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
                    line: 0,
                    character: 0,
                },
                window,
                cx,
            );
        });
        cx.run_until_parked();

        let (visible_text, cursor) = cx.update_window_entity(&input, |input, _, _| {
            (input.text().to_string(), input.cursor())
        });
        assert_eq!(visible_text, "> Quote");
        assert_eq!(cursor, 2);

        let snapshot = snapshot(&view, cx);
        assert_eq!(snapshot.selection.cursor(), 2);
        assert_eq!(snapshot.visible_selection.cursor(), 2);
    }

    #[gpui::test]
    fn single_surface_preserves_emphasis_markup_when_editing_visible_text(
        cx: &mut TestAppContext,
    ) {
        assert_visible_edit_round_trip(
            cx,
            "Hello *world*",
            "Hello world",
            "Hello there",
            "Hello *there*",
        );
    }

    #[gpui::test]
    fn single_surface_preserves_strikethrough_markup_when_editing_visible_text(
        cx: &mut TestAppContext,
    ) {
        assert_visible_edit_round_trip(
            cx,
            "Hello ~~world~~",
            "Hello world",
            "Hello there",
            "Hello ~~there~~",
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
    fn single_surface_preserves_link_markup_when_editing_visible_text(
        cx: &mut TestAppContext,
    ) {
        assert_visible_edit_round_trip(
            cx,
            "Hello [world](https://example.com)",
            "Hello world",
            "Hello there",
            "Hello [there](https://example.com)",
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

        let visible_text =
            cx.update_window_entity(&input, |input, _, _| input.text().to_string());
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
