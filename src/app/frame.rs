use super::layout::count_document_words;
use super::*;

impl VellumApp {
    pub(super) fn window_title(&self) -> String {
        let mut title = format!("{} - Vellum", self.document.display_name());
        if self.document.dirty {
            title.push_str(" *");
        }
        title
    }

    pub(super) fn current_document_dir(&self) -> Option<PathBuf> {
        self.document
            .path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| self.app_state.workspace_root.clone())
    }

    pub(super) fn set_status(&mut self, status: impl Into<SharedString>) {
        self.status_message = status.into();
    }

    pub(super) fn toggle_sidebar_visibility(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    pub(super) fn document_label(&self) -> String {
        let mut label = self.document.display_name();
        if self.document.dirty {
            label.push_str(" *");
        }
        label
    }

    pub(super) fn document_word_count(&self) -> usize {
        count_document_words(&self.document.text())
    }

    pub(super) fn render_app_menu(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();

        Button::new("app-menu")
            .icon(IconName::Menu)
            .ghost()
            .compact()
            .tooltip("Menu")
            .dropdown_menu(move |menu, _, _| {
                menu.min_w(px(220.))
                    .item(
                        PopupMenuItem::new("Open Folder")
                            .icon(IconName::FolderOpen)
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.open_folder_dialog(window, cx);
                                    });
                                }
                            }),
                    )
                    .item(
                        PopupMenuItem::new("Open File")
                            .icon(IconName::File)
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.open_file_dialog(window, cx);
                                    });
                                }
                            }),
                    )
                    .item(
                        PopupMenuItem::new("New File")
                            .icon(IconName::Plus)
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.create_new_file(window, cx);
                                    });
                                }
                            }),
                    )
                    .separator()
                    .item(PopupMenuItem::new("Save").on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                if let Err(err) = this.save_document(window, cx) {
                                    this.set_status(format!("Save failed: {err}"));
                                }
                            });
                        }
                    }))
                    .item(PopupMenuItem::new("Save As").on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                if let Err(err) = this.save_document_as(window, cx) {
                                    this.set_status(format!("Save As failed: {err}"));
                                }
                            });
                        }
                    }))
            })
    }

    pub(super) fn render_sidebar_toggle(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let is_open = self.sidebar_visible;
        let border_color = if is_open {
            cx.theme().foreground.opacity(0.45)
        } else {
            cx.theme().muted_foreground.opacity(0.55)
        };
        let background_color = if is_open {
            cx.theme().foreground.opacity(0.08)
        } else {
            cx.theme().background
        };
        let hover_border = cx.theme().foreground.opacity(0.38);
        let hover_background = cx.theme().secondary.opacity(0.18);

        div()
            .id("sidebar-toggle")
            .size(px(18.))
            .rounded_full()
            .border_1()
            .border_color(border_color)
            .bg(background_color)
            .hover(move |style| style.border_color(hover_border).bg(hover_background))
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.toggle_sidebar_visibility(cx);
                });
            })
    }

    pub(super) fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let (doc_status, icon, color) =
            if matches!(self.document.conflict, ConflictState::Conflict { .. }) {
                ("Conflict", IconName::TriangleAlert, cx.theme().warning)
            } else if self.document.saving {
                (
                    "Saving",
                    IconName::LoaderCircle,
                    cx.theme().muted_foreground,
                )
            } else if self.document.dirty {
                ("Edited", IconName::Asterisk, cx.theme().muted_foreground)
            } else {
                ("Saved", IconName::CircleCheck, cx.theme().success)
            };

        div()
            .flex()
            .justify_between()
            .items_center()
            .gap_4()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border.opacity(0.35))
            .bg(cx.theme().background)
            .text_sm()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(self.render_sidebar_toggle(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_color(cx.theme().muted_foreground)
                            .child(self.status_message.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Words {}", self.document_word_count())),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_color(color)
                            .child(Icon::new(icon).small())
                            .child(doc_status),
                    ),
            )
    }
}
