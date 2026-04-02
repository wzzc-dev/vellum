use gpui::{AnyElement, StatefulInteractiveElement as _, prelude::FluentBuilder as _};

use super::*;

impl VellumApp {
    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let selected_path = self.workspace.selected_file.clone();
        let foreground = cx.theme().foreground;

        div()
            .id("workspace-sidebar")
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border.opacity(0.18))
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
                        .rounded(px(6.))
                        .text_sm()
                        .w_full()
                        .pr_2()
                        .pl(px(8. + entry.depth() as f32 * 14.))
                        .child(
                            div()
                                .w_full()
                                .min_w(px(0.))
                                .overflow_hidden()
                                .text_color(foreground)
                                .truncate()
                                .child(label),
                        )
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
}

impl Render for VellumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(&self.window_title());

        let editor_panel = div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .child(self.editor.clone())
            .into_any_element();

        let body: AnyElement = if self.sidebar_visible {
            div()
                .size_full()
                .min_w(px(0.))
                .min_h(px(0.))
                .child(
                    h_resizable("vellum-layout")
                        .child(
                            resizable_panel()
                                .size(px(240.))
                                .size_range(px(180.)..px(360.))
                                .child(self.render_sidebar(cx)),
                        )
                        .child(resizable_panel().child(editor_panel)),
                )
                .into_any_element()
        } else {
            editor_panel
        };

        let status_bar = if self.status_bar_visible {
            Some(self.render_status_bar(cx).into_any_element())
        } else {
            None
        };

        div()
            .id("vellum-app")
            .key_context(APP_CONTEXT)
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .on_hover(cx.listener(Self::on_root_hover))
            .on_mouse_move(cx.listener(Self::on_root_mouse_move))
            .on_action(cx.listener(Self::on_open_file))
            .on_action(cx.listener(Self::on_open_folder))
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_save_now))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_toggle_sidebar))
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
                            .when(!cfg!(target_os = "macos"), |this| {
                                this.child(self.render_app_menu(cx))
                            })
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.document_label()),
                            )
                            .child(div().flex_1()),
                    ),
            )
            .child(div().flex_1().min_w(px(0.)).min_h(px(0.)).child(body))
            .when_some(status_bar, |this, status_bar| this.child(status_bar))
    }
}
