use std::rc::Rc;

use gpui::{
    AnyElement, ElementId, StatefulInteractiveElement as _, prelude::FluentBuilder as _,
    uniform_list,
};
use gpui_component::{
    Selectable,
    button::ButtonGroup,
    input::Input,
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    scroll::ScrollableElement,
};

use super::*;

#[derive(Debug, Clone)]
struct FileTreeEntry {
    path: PathBuf,
    label: String,
    depth: usize,
    is_folder: bool,
    is_expanded: bool,
}

fn flatten_tree_items(
    items: &[gpui_component::tree::TreeItem],
    depth: usize,
) -> Vec<FileTreeEntry> {
    let mut result = Vec::new();
    for item in items {
        let path = PathBuf::from(item.id.as_ref());
        let is_folder = !item.children.is_empty();
        let is_expanded = item.is_expanded();
        result.push(FileTreeEntry {
            path: path.clone(),
            label: item.label.to_string(),
            depth,
            is_folder,
            is_expanded,
        });
        if is_folder && is_expanded {
            result.extend(flatten_tree_items(&item.children, depth + 1));
        }
    }
    result
}

impl VellumApp {
    fn render_file_tree(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity().downgrade();
        let selected_path = self.workspace.selected_file.clone();
        let foreground = cx.theme().foreground;
        let renaming_path = self.renaming_path.clone();
        let rename_input = self.rename_input.clone();

        let entries = match self.workspace.tree_items() {
            Ok(items) => flatten_tree_items(&items, 0),
            Err(_) => Vec::new(),
        };
        let entries = Rc::new(entries);

        div()
            .size_full()
            .child(
                uniform_list("file-tree", entries.len(), {
                    let entries = entries.clone();
                    move |visible_range, _window, _cx| {
                        let mut items = Vec::with_capacity(visible_range.len());
                        for ix in visible_range {
                            let entry = &entries[ix];
                            let path = entry.path.clone();
                            let is_selected_file = selected_path.as_ref() == Some(&path);
                            let is_folder = entry.is_folder;
                            let is_renaming = renaming_path.as_ref() == Some(&path);

                            let label = if is_folder {
                                if entry.is_expanded {
                                    format!("v {}", entry.label)
                                } else {
                                    format!("> {}", entry.label)
                                }
                            } else {
                                entry.label.clone()
                            };

                            let content = if is_renaming {
                                if let Some(input) = rename_input.as_ref() {
                                    div()
                                        .w_full()
                                        .on_key_down({
                                            let view = view.clone();
                                            move |event: &gpui::KeyDownEvent, _, cx| {
                                                if event.keystroke.key.as_str() == "escape" {
                                                    if let Some(entity) = view.upgrade() {
                                                        let _ = entity.update(cx, |this, cx| {
                                                            this.cancel_rename(cx);
                                                        });
                                                    }
                                                }
                                            }
                                        })
                                        .child(Input::new(input).w_full().text_sm())
                                        .into_any_element()
                                } else {
                                    div()
                                        .w_full()
                                        .min_w(px(0.))
                                        .overflow_hidden()
                                        .text_color(foreground)
                                        .truncate()
                                        .child(label)
                                        .into_any_element()
                                }
                            } else {
                                div()
                                    .w_full()
                                    .min_w(px(0.))
                                    .overflow_hidden()
                                    .text_color(foreground)
                                    .truncate()
                                    .child(label)
                                    .into_any_element()
                            };

                            let item = ListItem::new(ix)
                                .selected(is_selected_file)
                                .rounded(px(6.))
                                .text_sm()
                                .w_full()
                                .pr_2()
                                .pl(px(8. + entry.depth as f32 * 14.))
                                .child(content)
                                .when(!is_renaming, |this| {
                                    this.on_click({
                                        let view = view.clone();
                                        let path = path.clone();
                                        move |_, window, cx| {
                                            if path.is_file() {
                                                if let Some(entity) = view.upgrade() {
                                                    let _ = entity.update(cx, |this, cx| {
                                                        this.open_file_in_current_tab(
                                                            path.clone(),
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            }
                                        }
                                    })
                                });

                            let menu_item = item.context_menu({
                                let view = view.clone();
                                let path = path.clone();
                                move |menu, _, _| {
                                    build_file_tree_context_menu(
                                        menu,
                                        view.clone(),
                                        path.clone(),
                                        is_folder,
                                    )
                                }
                            });

                            items.push(menu_item.into_any_element());
                        }
                        items
                    }
                })
                .size_full(),
            )
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

        // Outline filter input
        let filter_text = self.outline_filter.clone();
        let filter_view = view.clone();
        let filter_input = self.outline_filter_input.clone();

        let filtered_items: Vec<_> = self
            .editor_snapshot
            .outline
            .iter()
            .filter(|item| {
                filter_text.is_empty()
                    || item
                        .title
                        .to_lowercase()
                        .contains(&filter_text.to_lowercase())
            })
            .cloned()
            .collect();

        let filter_bar = div().w_full().child(Input::new(&filter_input));

        if filtered_items.is_empty() {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(filter_bar)
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(if self.editor_snapshot.outline.is_empty() {
                            "No headings yet"
                        } else {
                            "No matches"
                        }),
                )
                .into_any_element();
        }

        let mut items = div().w_full().flex().flex_col().gap_1();
        for (ix, item) in filtered_items.iter().enumerate() {
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
                        let view = filter_view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.active_editor_entity().update(cx, |editor, cx| {
                                    editor.select_block_start(block_id, window, cx);
                                });
                            });
                        }
                    }),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(filter_bar)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scrollbar()
                    .child(items),
            )
            .into_any_element()
    }

    pub(super) fn render_find_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let find_input = self.find_query_input.clone();
        let replace_visible = self.replace_visible;

        let has_matches = !self.find_matches.is_empty();
        let nav_color = if has_matches {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        let match_count_label = if !self.find_query.is_empty() {
            let current = self.active_find_index.map(|i| i + 1).unwrap_or(0);
            let total = self.find_matches.len();
            format!("{current}/{total}")
        } else {
            String::new()
        };

        let toggle_icon = if replace_visible { "▾" } else { "▸" };

        let find_row = div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("find-toggle-replace-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.replace_visible = !this.replace_visible;
                                cx.notify();
                            });
                        }
                    })
                    .child(toggle_icon),
            )
            .child(Input::new(&find_input).flex_1())
            .child(
                div()
                    .id("find-case-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_sm()
                    .when(self.find_case_sensitive, |this| {
                        this.bg(cx.theme().primary.opacity(0.15))
                            .text_color(cx.theme().primary)
                    })
                    .when(!self.find_case_sensitive, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.find_case_sensitive = !this.find_case_sensitive;
                                this.refresh_find_matches();
                                cx.notify();
                            });
                        }
                    })
                    .child("Aa"),
            )
            .child(
                div()
                    .id("find-whole-word-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_sm()
                    .when(self.find_whole_word, |this| {
                        this.bg(cx.theme().primary.opacity(0.15))
                            .text_color(cx.theme().primary)
                    })
                    .when(!self.find_whole_word, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.find_whole_word = !this.find_whole_word;
                                this.refresh_find_matches();
                                cx.notify();
                            });
                        }
                    })
                    .child("W"),
            )
            .child(
                div()
                    .id("find-regex-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_sm()
                    .when(self.find_regex, |this| {
                        this.bg(cx.theme().primary.opacity(0.15))
                            .text_color(cx.theme().primary)
                    })
                    .when(!self.find_regex, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.find_regex = !this.find_regex;
                                this.refresh_find_matches();
                                cx.notify();
                            });
                        }
                    })
                    .child(".*"),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(match_count_label),
            )
            .child(
                div()
                    .id("find-prev-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(nav_color)
                    .text_sm()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.on_find_previous_match(&FindPreviousMatch, window, cx);
                            });
                        }
                    })
                    .child("↑"),
            )
            .child(
                div()
                    .id("find-next-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(nav_color)
                    .text_sm()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.on_find_next_match(&FindNextMatch, window, cx);
                            });
                        }
                    })
                    .child("↓"),
            )
            .child(
                div()
                    .id("find-close-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.close_find_panel();
                                cx.notify();
                            });
                        }
                    })
                    .child("✕"),
            );

        let replace_input = self.replace_query_input.clone();
        let replace_row = div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .child(div().size(px(20.)))
            .child(Input::new(&replace_input).flex_1())
            .child(
                div()
                    .id("replace-one-btn")
                    .px_2()
                    .py(px(2.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(cx.theme().foreground)
                    .text_xs()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.on_replace_one(&ReplaceOne, window, cx);
                            });
                        }
                    })
                    .child("Replace"),
            )
            .child(
                div()
                    .id("replace-all-btn")
                    .px_2()
                    .py(px(2.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(cx.theme().foreground)
                    .text_xs()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.on_replace_all(&ReplaceAll, window, cx);
                            });
                        }
                    })
                    .child("All"),
            );

        div()
            .w_full()
            .flex()
            .flex_col()
            .px_3()
            .py_1()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.18))
            .bg(cx.theme().background)
            .child(find_row)
            .when(replace_visible, |this| this.child(replace_row))
            .into_any_element()
    }

    pub(super) fn render_goto_line_bar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let input = self.goto_line_input.clone();
        let line_count = self.editor_snapshot.document_text.lines().count().max(1);

        div()
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.18))
            .bg(cx.theme().background)
            .on_action(cx.listener(
                |this, _: &gpui_component::input::Enter, window, cx| {
                    this.apply_goto_line(window, cx);
                },
            ))
            .child(
                div()
                    .flex_shrink_0()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Go to line"),
            )
            .child(Input::new(&input).w(px(160.)))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("1-{line_count}")),
            )
            .child(
                div()
                    .id("goto-line-submit-btn")
                    .px_2()
                    .py(px(2.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(cx.theme().foreground)
                    .text_xs()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.apply_goto_line(window, cx);
                            });
                        }
                    })
                    .child("Go"),
            )
            .child(
                div()
                    .id("goto-line-close-btn")
                    .size(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(cx.theme().muted_foreground)
                    .text_sm()
                    .hover(|style| style.bg(cx.theme().secondary.opacity(0.12)))
                    .on_click(move |_, window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.close_goto_line(window, cx);
                        });
                    })
                    .child("✕"),
            )
            .into_any_element()
    }

    fn render_sidebar(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .on_click(move |selected: &Vec<usize>, _, cx| {
                let target = if selected.contains(&1) {
                    SidebarView::Outline
                } else {
                    SidebarView::Files
                };
                let _ = view.update(cx, |this, cx| {
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

        let pending = std::mem::take(&mut self.pending_file_opens);
        for path in pending {
            self.open_file(path, window, cx);
        }

        let show_sidebar = self.sidebar_visible && !self.focus_mode;
        let show_tabs = self.tabs.len() > 1 && !self.focus_mode;
        let show_status_bar = self.status_bar_visible && !self.focus_mode;

        let editor_panel = div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(cx.theme().background)
            .flex()
            .flex_col()
            .when(self.find_panel_visible, |this| {
                this.child(self.render_find_bar(cx))
            })
            .when(self.goto_line_visible, |this| {
                this.child(self.render_goto_line_bar(cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .child(self.active_editor_entity().clone()),
            )
            .into_any_element();

        let mut layout = h_resizable("vellum-layout");
        if show_sidebar {
            layout = layout.child(
                resizable_panel()
                    .size(px(240.))
                    .size_range(px(180.)..px(360.))
                    .child(self.render_sidebar(window, cx)),
            );
        }
        layout = layout.child(resizable_panel().child(editor_panel));
        let body: AnyElement = div()
            .size_full()
            .min_w(px(0.))
            .min_h(px(0.))
            .child(layout)
            .into_any_element();

        let status_bar = if show_status_bar {
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
            .on_action(cx.listener(Self::on_export_html))
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_toggle_status_bar))
            .on_action(cx.listener(Self::on_toggle_focus_mode))
            .on_action(cx.listener(Self::on_open_goto_line))
            .on_action(cx.listener(Self::on_open_find_panel))
            .on_action(cx.listener(Self::on_close_find_panel))
            .on_action(cx.listener(Self::on_find_next_match))
            .on_action(cx.listener(Self::on_find_previous_match))
            .on_action(cx.listener(Self::on_open_find_replace_panel))
            .on_action(cx.listener(Self::on_replace_one))
            .on_action(cx.listener(Self::on_replace_all))
            .on_action(cx.listener(Self::on_close_tab))
            .on_action(cx.listener(Self::on_previous_tab))
            .on_action(cx.listener(Self::on_next_tab))
            .on_action(cx.listener(Self::on_open_command_palette))
            .child(
                TitleBar::new()
                    .bg(cx.theme().background)
                    .border_color(cx.theme().background)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .w_full()
                            .min_w(px(0.))
                            .h_full()
                            .when(show_sidebar, |this| {
                                this.child(
                                    div()
                                        .w(px(240.))
                                        .flex_shrink_0()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .px_2()
                                        .when(!cfg!(target_os = "macos"), |this| {
                                            this.child(self.render_app_menu(cx))
                                        })
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .max_w(px(180.))
                                                .truncate()
                                                .child(self.document_label()),
                                        ),
                                )
                            })
                            .when(!show_sidebar, |this| {
                                this.child(
                                    div()
                                        .flex_shrink_0()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .px_2()
                                        .when(!cfg!(target_os = "macos"), |this| {
                                            this.child(self.render_app_menu(cx))
                                        })
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .max_w(px(220.))
                                                .truncate()
                                                .child(self.document_label()),
                                        ),
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .h_full()
                                    .when(show_tabs, |this| {
                                        this.child(self.render_tab_bar(window, cx))
                                    }),
                            )
                    ),
            )
            .child(div().flex_1().min_w(px(0.)).min_h(px(0.)).child(body))
            .child(self.render_command_palette(cx))
            .when_some(status_bar, |this, status_bar| this.child(status_bar))
    }
}

impl VellumApp {
    fn render_command_palette(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if !self.command_palette.is_visible() {
            return div().into_any_element();
        }

        let theme = cx.theme();
        let selected = self.command_palette.selected_index;
        let commands = &crate::app::command_palette::ALL_COMMANDS;
        let filtered = &self.command_palette.filtered_indices;

        let command_items: Vec<_> = filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let item = &commands[idx];
                let is_selected = i == selected;
                div()
                    .id(ElementId::Name(format!("cmd-item-{}", idx).into()))
                    .w_full()
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .rounded(px(4.))
                    .when(is_selected, |this| this.bg(theme.list_active))
                    .when(!is_selected, |this| {
                        this.hover(|this| this.bg(theme.list_hover))
                    })
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(item.label),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(item.description),
                            ),
                    )
                    .into_any_element()
            })
            .collect();

        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.command_palette.hide();
                    window.focus(&this.focus_handle);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .pt(px(80.0))
                    .w_full()
                    .child(
                        div()
                            .w(px(500.0))
                            .max_h(px(420.0))
                            .bg(theme.popover)
                            .border_1()
                            .border_color(theme.border)
                            .rounded_lg()
                            .shadow_xl()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_action(cx.listener(Self::on_palette_enter))
                            .on_action(cx.listener(Self::on_palette_move_up))
                            .on_action(cx.listener(Self::on_palette_move_down))
                            .child(
                                div()
                                    .p_3()
                                    .border_b_1()
                                    .border_color(theme.border)
                                    .child(Input::new(&self.command_palette.input).w_full()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_y_scrollbar()
                                    .p_2()
                                    .children(command_items),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn build_file_tree_context_menu(
    menu: PopupMenu,
    view: gpui::WeakEntity<VellumApp>,
    path: PathBuf,
    is_folder: bool,
) -> PopupMenu {
    if is_folder {
        menu.item(PopupMenuItem::new("New File").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.create_new_file_in_folder(path.clone(), window, cx);
                    });
                }
            }
        }))
        .item(PopupMenuItem::new("New Folder").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.create_new_folder(path.clone(), window, cx);
                    });
                }
            }
        }))
        .separator()
        .item(PopupMenuItem::new("Copy Path").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.copy_path_to_clipboard(&path, window, cx);
                    });
                }
            }
        }))
        .item(PopupMenuItem::new("Reveal in Finder").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, _, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, _cx| {
                        this.reveal_in_finder(&path);
                    });
                }
            }
        }))
        .separator()
        .item(PopupMenuItem::new("Rename").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.start_rename(path.clone(), window, cx);
                    });
                }
            }
        }))
        .item(PopupMenuItem::new("Delete").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.delete_file(path.clone(), window, cx);
                    });
                }
            }
        }))
    } else {
        menu.item(PopupMenuItem::new("Open in New Tab").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.open_file(path.clone(), window, cx);
                    });
                }
            }
        }))
        .separator()
        .item(PopupMenuItem::new("Copy Path").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.copy_path_to_clipboard(&path, window, cx);
                    });
                }
            }
        }))
        .item(PopupMenuItem::new("Reveal in Finder").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, _, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, _cx| {
                        this.reveal_in_finder(&path);
                    });
                }
            }
        }))
        .separator()
        .item(PopupMenuItem::new("Rename").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.start_rename(path.clone(), window, cx);
                    });
                }
            }
        }))
        .item(PopupMenuItem::new("Delete").on_click({
            let view = view.clone();
            let path = path.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.delete_file(path.clone(), window, cx);
                    });
                }
            }
        }))
    }
}

pub(super) fn build_tab_context_menu(
    menu: PopupMenu,
    view: gpui::WeakEntity<VellumApp>,
    tab_index: usize,
    tab_count: usize,
) -> PopupMenu {
    let mut menu = menu.item(PopupMenuItem::new("Close").on_click({
        let view = view.clone();
        move |_, window, cx| {
            if let Some(entity) = view.upgrade() {
                let _ = entity.update(cx, |this, cx| {
                    this.close_tab(tab_index, window, cx);
                });
            }
        }
    }));

    if tab_count > 1 {
        menu = menu.item(PopupMenuItem::new("Close Others").on_click({
            let view = view.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.close_other_tabs(tab_index, window, cx);
                    });
                }
            }
        }));
        menu = menu.item(PopupMenuItem::new("Close All").on_click({
            let view = view.clone();
            move |_, window, cx| {
                if let Some(entity) = view.upgrade() {
                    let _ = entity.update(cx, |this, cx| {
                        this.close_all_tabs(window, cx);
                    });
                }
            }
        }));
    }

    menu
}
