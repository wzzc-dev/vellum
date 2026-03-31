use super::layout::{block_layout_metrics, markdown_preview_style, style_active_input_for_block};
use super::*;

impl VellumApp {
    pub(super) fn clear_session(&mut self) {
        self.input_subscription = None;
        self.active_session = None;
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let selected_path = self.workspace.selected_file.clone();
        let foreground = cx.theme().foreground;

        div()
            .size_full()
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border.opacity(0.35))
            .p_3()
            .child(
                tree(&self.tree_state, move |ix, entry, selected, _, _| {
                    let path = PathBuf::from(entry.item().id.as_ref());
                    let label = if entry.is_folder() {
                        if entry.is_expanded() {
                            format!("v {}", entry.item().label)
                        } else {
                            format!("> {}", entry.item().label)
                        }
                    } else {
                        entry.item().label.to_string()
                    };

                    let is_selected_file = selected_path.as_ref() == Some(&path);
                    ListItem::new(ix)
                        .selected(selected || is_selected_file)
                        .rounded(px(8.))
                        .text_sm()
                        .pl(px(8. + entry.depth() as f32 * 14.))
                        .child(div().text_color(foreground).child(label))
                        .on_click({
                            let view = view.clone();
                            move |_, window, cx| {
                                if path.is_file() {
                                    let _ = view.update(cx, |this, cx| {
                                        this.open_file(path.clone(), window, cx);
                                    });
                                }
                            }
                        })
                })
                .size_full(),
            )
    }

    fn render_conflict_banner(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if !matches!(self.document.conflict, ConflictState::Conflict { .. }) {
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

    fn render_empty_state(&self, cx: &Context<Self>) -> impl IntoElement {
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

    fn render_block_row(
        &mut self,
        block_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let block = self.document.blocks[block_ix].clone();
        let block_text = self.document.block_text(&block);
        let is_active = self
            .active_session
            .as_ref()
            .map(|session| session.block_id == block.id)
            .unwrap_or(false);
        let view = cx.entity();
        let metrics = block_layout_metrics(&block.kind);

        let content = if is_active {
            let session = self.active_session.as_ref().expect("active session");
            let input = style_active_input_for_block(
                Input::new(&session.input)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false),
                &block.kind,
            );
            div()
                .px_1()
                .py(px(metrics.block_padding_y))
                .child(input)
                .into_any_element()
        } else if self.document.is_empty() && block_text.is_empty() {
            div()
                .px_1()
                .py(px(metrics.block_padding_y + 6.))
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .text_color(cx.theme().muted_foreground)
                .child("Start writing...")
                .into_any_element()
        } else {
            div()
                .px_1()
                .py(px(metrics.block_padding_y))
                .text_size(px(BODY_FONT_SIZE))
                .line_height(px(BODY_LINE_HEIGHT))
                .child(
                    TextView::markdown(("preview", block.id), block_text, window, cx)
                        .style(markdown_preview_style()),
                )
                .into_any_element()
        };

        div()
            .id(("block-row", block.id))
            .w_full()
            .py(px(metrics.row_spacing_y))
            .child(
                div()
                    .id(("activate-block", block.id))
                    .w_full()
                    .on_click(move |_, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.activate_block(block_ix, window, cx);
                        });
                    })
                    .child(content),
            )
            .into_any_element()
    }

    fn block_item_sizes(&self) -> Rc<Vec<gpui::Size<gpui::Pixels>>> {
        Rc::new(
            self.document
                .blocks
                .iter()
                .map(|block| {
                    let text = self.document.block_text(block);
                    let line_count = cmp::max(text.lines().count(), 1);
                    let metrics = block_layout_metrics(&block.kind);
                    size(
                        px(1.),
                        px(metrics.block_padding_y * 2.
                            + metrics.row_spacing_y * 2.
                            + metrics.line_height * line_count as f32
                            + metrics.extra_height),
                    )
                })
                .collect(),
        )
    }

    fn render_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let sizes = self.block_item_sizes();
        let conflict_banner = self
            .render_conflict_banner(cx)
            .map(|banner| banner.into_any_element());
        let content = if self.document.blocks.is_empty() {
            self.render_empty_state(cx).into_any_element()
        } else {
            v_virtual_list(
                view,
                "document-blocks",
                sizes,
                |this, range: Range<usize>, window, cx| {
                    range
                        .map(|ix| this.render_block_row(ix, window, cx))
                        .collect::<Vec<_>>()
                },
            )
            .size_full()
            .into_any_element()
        };

        div()
            .size_full()
            .bg(cx.theme().background)
            .overflow_hidden()
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .px_8()
                    .pt(px(28.))
                    .pb(px(44.))
                    .when_some(conflict_banner, |this, banner| this.child(banner))
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .mx_auto()
                            .max_w(px(MAX_EDITOR_WIDTH))
                            .w_full()
                            .child(content),
                    ),
            )
            .into_any_element()
    }
}

impl Render for VellumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(&self.window_title());

        let body = if self.sidebar_visible {
            h_resizable("vellum-layout")
                .child(
                    resizable_panel()
                        .size(px(240.))
                        .size_range(px(180.)..px(360.))
                        .child(self.render_sidebar(cx)),
                )
                .child(resizable_panel().child(self.render_editor(window, cx)))
                .into_any_element()
        } else {
            div()
                .size_full()
                .child(self.render_editor(window, cx))
                .into_any_element()
        };

        div()
            .id("vellum-app")
            .key_context(APP_CONTEXT)
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_open_file))
            .on_action(cx.listener(Self::on_open_folder))
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_save_now))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_bold_selection))
            .on_action(cx.listener(Self::on_italic_selection))
            .on_action(cx.listener(Self::on_link_selection))
            .on_action(cx.listener(Self::on_promote_block))
            .on_action(cx.listener(Self::on_demote_block))
            .on_action(cx.listener(Self::on_exit_block_edit))
            .on_action(cx.listener(Self::on_focus_prev_block))
            .on_action(cx.listener(Self::on_focus_next_block))
            .child(
                TitleBar::new()
                    .bg(cx.theme().background)
                    .border_color(cx.theme().background)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .w_full()
                            .px_2()
                            .pr_3()
                            .child(self.render_app_menu(cx))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.document_label()),
                            )
                            .child(div().flex_1()),
                    ),
            )
            .child(div().flex_1().min_h(px(0.)).child(body))
            .child(self.render_status_bar(cx))
    }
}
