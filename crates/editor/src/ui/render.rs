use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme,
    input::{Backspace, Enter, MoveDown, MoveUp},
};

use super::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, MAX_EDITOR_WIDTH,
    component_ui::{Button, ButtonVariants as _, render_block_list, render_markdown_preview},
    layout::block_presentation,
    view::MarkdownEditor,
};

impl MarkdownEditor {
    pub(super) fn render_conflict_banner(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if !self.snapshot.has_conflict {
            return None;
        }

        let view = cx.entity();
        Some(
            div()
                .flex()
                .justify_between()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .mb_4()
                .rounded(px(8.))
                .bg(cx.theme().warning.opacity(0.08))
                .border_1()
                .border_color(cx.theme().warning.opacity(0.22))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child("External file changes detected")
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(
                                "Reload the disk version or keep your current in-memory changes.",
                            ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("reload-disk")
                                .label("Reload Disk Version")
                                .warning()
                                .compact()
                                .on_click({
                                    let view = view.clone();
                                    move |_, window, cx| {
                                        let _ = view.update(cx, |this, cx| {
                                            this.reload_conflict_from_disk(window, cx);
                                        });
                                    }
                                }),
                        )
                        .child(
                            Button::new("keep-current")
                                .label("Keep Current Changes")
                                .ghost()
                                .compact()
                                .on_click(move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.keep_current_conflicted_version(window, cx);
                                    });
                                }),
                        ),
                ),
        )
    }

    pub(super) fn render_empty_state(&self, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt(px(56.))
            .text_size(px(BODY_FONT_SIZE))
            .line_height(px(BODY_LINE_HEIGHT))
            .text_color(cx.theme().muted_foreground)
            .child("Open a Markdown file or press Ctrl+N to start writing.")
            .child(
                div()
                    .text_sm()
                    .child("Vellum keeps editing in a single quiet writing column."),
            )
    }

    pub(super) fn render_block_row(
        &mut self,
        block_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let block = self.snapshot.blocks[block_ix].clone();
        let block_is_empty = block.text.is_empty();
        let is_active = self.interaction.is_block_active(block.id);
        let view = cx.entity();
        let presentation = block_presentation(&block.kind);

        let content_body = if is_active {
            let session = self.interaction.active_session().expect("active session");
            div()
                .w_full()
                .child(session.input.render(&block.kind))
                .into_any_element()
        } else if self.snapshot.blocks.len() == 1 && block_is_empty {
            div()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .text_color(cx.theme().muted_foreground)
                .child("Start writing...")
                .into_any_element()
        } else {
            div()
                .text_size(px(presentation.font_size))
                .line_height(px(presentation.line_height))
                .child(render_markdown_preview(
                    block.id,
                    &block.kind,
                    block.text,
                    window,
                    cx,
                ))
                .into_any_element()
        };

        let content = div()
            .px_1()
            .py(px(presentation.block_padding_y))
            .child(
                div()
                    .relative()
                    .w_full()
                    .child(self.interaction.capture_block_bounds(block.id))
                    .child(content_body),
            )
            .into_any_element();

        div()
            .id(("block-row", block.id))
            .w_full()
            .py(px(presentation.row_spacing_y))
            .child(
                div()
                    .id(("activate-block", block.id))
                    .w_full()
                    .on_click(move |event, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.activate_block_from_click(block_ix, event, window, cx);
                        });
                    })
                    .child(content),
            )
            .into_any_element()
    }

    pub(super) fn render_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.interaction.clear_block_bounds();
        let conflict_banner = self
            .render_conflict_banner(cx)
            .map(|banner| banner.into_any_element());
        let content = if self.snapshot.blocks.is_empty() {
            self.render_empty_state(cx).into_any_element()
        } else {
            render_block_list(
                (0..self.snapshot.blocks.len())
                    .map(|ix| self.render_block_row(ix, window, cx))
                    .collect::<Vec<_>>(),
            )
        };
        let view = cx.entity();

        div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .when(self.interaction.active_session().is_some(), |this| {
                let backspace_view = view.clone();
                let enter_view = view.clone();
                let keydown_view = view.clone();
                let up_view = view.clone();
                let down_view = view.clone();
                this.capture_key_down(move |event, window, app| {
                    let _ = keydown_view.update(app, |this, cx| {
                        this.record_active_enter_keydown(event, window, cx);
                    });
                })
                .capture_action(move |_: &Backspace, window, app| {
                    let handled = backspace_view.update(app, |this, cx| {
                        this.handle_active_boundary_backspace_action(window, cx)
                    });
                    if handled {
                        app.stop_propagation();
                    }
                })
                .capture_action(move |action: &Enter, window, app| {
                    let handled = enter_view.update(app, |this, cx| {
                        this.handle_active_semantic_enter_action(action.secondary, window, cx)
                    });
                    if handled {
                        app.stop_propagation();
                    }
                })
                .capture_action(move |_: &MoveUp, window, app| {
                    let handled = up_view.update(app, |this, cx| {
                        this.handle_active_navigation_action(-1, window, cx)
                    });
                    if handled {
                        app.stop_propagation();
                    }
                })
                .capture_action(move |_: &MoveDown, window, app| {
                    let handled = down_view.update(app, |this, cx| {
                        this.handle_active_navigation_action(1, window, cx)
                    });
                    if handled {
                        app.stop_propagation();
                    }
                })
            })
            .child(
                div()
                    .size_full()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("editor-scroll-area")
                            .flex_1()
                            .min_w(px(0.))
                            .min_h(px(0.))
                            .overflow_y_scroll()
                            .child(
                                div().w_full().px_8().pt(px(28.)).pb(px(44.)).child(
                                    div()
                                        .mx_auto()
                                        .max_w(px(MAX_EDITOR_WIDTH))
                                        .w_full()
                                        .flex()
                                        .flex_col()
                                        .when_some(conflict_banner, |this, banner| {
                                            this.child(banner)
                                        })
                                        .child(content),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }
}
