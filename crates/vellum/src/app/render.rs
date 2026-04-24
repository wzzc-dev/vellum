use gpui::{AnyElement, StatefulInteractiveElement as _, prelude::FluentBuilder as _};
use gpui_component::{Selectable, button::ButtonGroup, scroll::ScrollableElement};

use super::*;

impl VellumApp {
    fn render_file_tree(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let selected_path = self.workspace.selected_file.clone();
        let foreground = cx.theme().foreground;

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
        .size_full()
        .into_any_element()
    }

    fn render_outline(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let active_block_id = self
            .editor_snapshot
            .display_map
            .blocks
            .iter()
            .find(|block| {
                let cursor = self.editor_snapshot.selection.cursor();
                cursor >= block.source_range.start && cursor <= block.source_range.end
            })
            .map(|block| block.id);

        if self.editor_snapshot.outline.is_empty() {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("No headings yet")
                .into_any_element();
        }

        let mut items = div().w_full().flex().flex_col().gap_1();
        for (ix, item) in self.editor_snapshot.outline.iter().enumerate() {
            let block_id = item.block_id;
            let title = item.title.clone();
            let depth = item.depth.saturating_sub(1) as f32;
            items = items.child(
                ListItem::new(("outline-item", ix))
                    .selected(active_block_id == Some(block_id))
                    .rounded(px(6.))
                    .text_sm()
                    .w_full()
                    .pr_2()
                    .pl(px(8. + depth * 14.))
                    .child(
                        div()
                            .w_full()
                            .min_w(px(0.))
                            .overflow_hidden()
                            .text_color(cx.theme().foreground)
                            .truncate()
                            .child(title),
                    )
                    .on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.editor.update(cx, |editor, cx| {
                                    editor.select_block_start(block_id, window, cx);
                                });
                            });
                        }
                    }),
            );
        }

        div()
            .size_full()
            .overflow_y_scrollbar()
            .child(items)
            .into_any_element()
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let body = match self.sidebar_view {
            SidebarView::Files => self.render_file_tree(cx),
            SidebarView::Outline => self.render_outline(cx),
        };

        let tabs = ButtonGroup::new("workspace-sidebar-tabs")
            .compact()
            .ghost()
            .child(
                Button::new("sidebar-files")
                    .label("Files")
                    .selected(self.sidebar_view == SidebarView::Files),
            )
            .child(
                Button::new("sidebar-outline")
                    .label("Outline")
                    .selected(self.sidebar_view == SidebarView::Outline),
            )
            .on_click(move |selected: &Vec<usize>, _, app| {
                let target = if selected.contains(&1) {
                    SidebarView::Outline
                } else {
                    SidebarView::Files
                };
                let _ = view.update(app, |this, cx| {
                    this.set_sidebar_view(target, cx);
                });
            });

        div()
            .id("workspace-sidebar")
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .border_r_1()
            .border_color(cx.theme().border.opacity(0.18))
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(tabs)
            .child(div().flex_1().min_h(px(0.)).child(body))
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
            .track_focus(&self.focus_handle)
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
            .on_action(cx.listener(Self::on_toggle_status_bar))
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
