use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::ActiveTheme;

use super::{
    BODY_FONT_SIZE, BODY_LINE_HEIGHT, MAX_EDITOR_WIDTH,
    component_ui::{Button, ButtonVariants as _, render_block_list, render_markdown_preview},
    layout::block_layout_metrics,
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
                                .child("Reload the disk version or keep your current in-memory changes."),
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
        let metrics = block_layout_metrics(&block.kind);

        let content_body = if is_active {
            let session = self.interaction.active_session().expect("active session");
            div()
                .w_full()
                .capture_key_down({
                    let view = view.clone();
                    move |event, window, cx| {
                        let handled =
                            view.update(cx, |this, cx| this.handle_active_navigation_key(event, window, cx));
                        if handled {
                            cx.stop_propagation();
                            window.prevent_default();
                        }
                    }
                })
                .child(session.input.render(&block.kind))
                .into_any_element()
        } else if self.snapshot.blocks.len() == 1 && block_is_empty {
            div()
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .text_color(cx.theme().muted_foreground)
                .child("Start writing...")
                .into_any_element()
        } else {
            div()
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .child(render_markdown_preview(block.id, block.text, window, cx))
                .into_any_element()
        };

        let placeholder_extra_padding = if !is_active && self.snapshot.blocks.len() == 1 && block_is_empty {
            6.
        } else {
            0.
        };
        let content = div()
            .px_1()
            .py(px(metrics.block_padding_y + placeholder_extra_padding))
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
            .py(px(metrics.row_spacing_y))
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

    pub(super) fn render_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
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

        div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
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
