use gpui::{StatefulInteractiveElement as _, prelude::FluentBuilder as _};
use gpui_component::{
    Selectable, Sizable as _, button::ButtonGroup, menu::{ContextMenuExt, DropdownMenu as _, PopupMenuItem},
};

use super::render::build_tab_context_menu;
use super::*;

impl VellumApp {
    fn hide_status_bar(&mut self, cx: &mut Context<Self>) {
        self.cancel_status_bar_hide();
        if self.status_bar_visible {
            self.status_bar_visible = false;
            cx.notify();
        }
    }

    fn cancel_status_bar_hide(&mut self) {
        self.status_bar_hide_generation = self.status_bar_hide_generation.wrapping_add(1);
    }

    fn reveal_status_bar(&mut self, cx: &mut Context<Self>) {
        self.cancel_status_bar_hide();
        if !self.status_bar_visible {
            self.status_bar_visible = true;
            cx.notify();
        }
    }

    fn reveal_status_bar_temporarily(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reveal_status_bar(cx);
        self.schedule_status_bar_hide(window, cx);
    }

    fn schedule_status_bar_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.status_bar_pinned || !self.status_bar_visible {
            return;
        }

        self.status_bar_hide_generation = self.status_bar_hide_generation.wrapping_add(1);
        let token = self.status_bar_hide_generation;
        let view = cx.entity();
        window
            .spawn(cx, async move |cx| {
                Timer::after(STATUS_BAR_HIDE_DELAY).await;
                let _ = cx.update_window_entity(&view, |this, _, cx| {
                    if this.status_bar_hide_generation == token && !this.status_bar_pinned {
                        this.status_bar_visible = false;
                        cx.notify();
                    }
                });
            })
            .detach();
    }

    pub(super) fn set_status_bar_pinned(
        &mut self,
        pinned: bool,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.status_bar_pinned = pinned;
        if pinned {
            self.reveal_status_bar(cx);
        } else {
            self.hide_status_bar(cx);
        }
    }

    pub(super) fn on_root_mouse_move(
        &mut self,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let reveal_threshold = window.viewport_size().height - px(STATUS_BAR_REVEAL_EDGE_HEIGHT);
        let near_bottom = event.position.y >= reveal_threshold;

        if self.status_bar_edge_hovered == near_bottom {
            return;
        }

        self.status_bar_edge_hovered = near_bottom;
        if near_bottom {
            if self.status_bar_pinned {
                self.reveal_status_bar(cx);
            } else {
                self.reveal_status_bar_temporarily(window, cx);
            }
        } else if !self.status_bar_hovered && !self.status_bar_pinned {
            self.schedule_status_bar_hide(window, cx);
        }
    }

    pub(super) fn on_root_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if *hovered {
            return;
        }

        self.status_bar_edge_hovered = false;
        self.status_bar_hovered = false;
        if !self.status_bar_pinned {
            self.hide_status_bar(cx);
        } else {
            self.schedule_status_bar_hide(window, cx);
        }
    }

    pub(super) fn on_status_bar_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.status_bar_hovered == *hovered {
            return;
        }

        self.status_bar_hovered = *hovered;
        if *hovered {
            if self.status_bar_pinned {
                self.reveal_status_bar(cx);
            } else {
                self.reveal_status_bar_temporarily(window, cx);
            }
        } else if !self.status_bar_edge_hovered && !self.status_bar_pinned {
            self.schedule_status_bar_hide(window, cx);
        }
    }

    pub(super) fn window_title(&self) -> String {
        let dirty_suffix = if self.editor_snapshot.dirty { " *" } else { "" };
        format!(
            "{}{dirty_suffix} - Vellum",
            self.editor_snapshot.display_name
        )
    }

    pub(super) fn current_document_dir(&self) -> Option<PathBuf> {
        self.editor_snapshot
            .path
            .as_ref()
            .or(self.editor_snapshot.suggested_path.as_ref())
            .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
            .or_else(|| self.app_state.workspace_root.clone())
    }

    pub(super) fn set_status(&mut self, status: impl Into<String>) {
        self.shell_status_message = status.into();
    }

    pub(super) fn clear_status(&mut self) {
        self.shell_status_message.clear();
    }

    pub(super) fn toggle_sidebar_visibility(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    pub(super) fn set_sidebar_view(&mut self, view: SidebarView, cx: &mut Context<Self>) {
        if self.sidebar_view == view {
            return;
        }

        self.sidebar_view = view;
        cx.notify();
    }

    pub(super) fn toggle_plugin(&mut self, plugin_id: String, cx: &mut Context<Self>) {
        if self.disabled_plugin_ids.contains(&plugin_id) {
            self.disabled_plugin_ids.retain(|id| id != &plugin_id);
            if let Some(plugin_dir) = dirs::data_dir().map(|d| d.join("dev.vellum").join("plugins")) {
                let wasm_path = plugin_dir.join(format!("{}.wasm", plugin_id));
                if wasm_path.exists() {
                    if let Err(e) = self.plugin_manager.load_plugin(&wasm_path) {
                        eprintln!("failed to reload plugin {}: {}", plugin_id, e);
                    }
                }
            }
        } else {
            self.disabled_plugin_ids.push(plugin_id.clone());
            self.webview_manager.remove_all();
            if let Err(e) = self.plugin_manager.unload_plugin(&plugin_id) {
                eprintln!("failed to unload plugin {}: {}", plugin_id, e);
            }
        }
        cx.notify();
    }

    pub(super) fn loaded_plugin_manifests(&self) -> Vec<vellum_plugin::PluginManifest> {
        self.plugin_manager.loaded_manifests()
    }

    pub(super) fn document_label(&self) -> String {
        let mut label = self.editor_snapshot.display_name.clone();
        if self.editor_snapshot.dirty {
            label.push_str(" *");
        }
        label
    }

    pub(super) fn document_word_count(&self) -> usize {
        self.editor_snapshot.word_count
    }

    fn status_message(&self) -> String {
        if self.shell_status_message.is_empty() {
            self.editor_snapshot.status_message.clone()
        } else {
            self.shell_status_message.clone()
        }
    }

    pub(super) fn render_app_menu(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();

        Button::new("app-menu")
            .icon(IconName::Menu)
            .ghost()
            .compact()
            .tooltip("File")
            .dropdown_menu(move |menu, _, _| {
                menu.min_w(px(220.))
                    .item(
                        PopupMenuItem::new("Open Folder")
                            .icon(IconName::FolderOpen)
                            .on_click({
                                let view = view.clone();
                                move |_, window, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.request_open_folder(window, cx);
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
                                    cx.notify();
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
                                    cx.notify();
                                }
                            });
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new("Toggle Source Mode").on_click({
                        let view = view.clone();
                        move |_, window, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.active_editor_entity().update(cx, |editor, cx| {
                                    editor.toggle_view_mode(window, cx);
                                });
                            });
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new("Plugins").icon(IconName::Settings).on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.sidebar_visible = true;
                                this.set_sidebar_view(SidebarView::Plugins, cx);
                            });
                        }
                    }))
            })
    }

    pub(super) fn render_plugins_menu(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let manifests = self.loaded_plugin_manifests();
        let disabled = self.disabled_plugin_ids.clone();

        Button::new("plugins-menu")
            .icon(IconName::Settings)
            .ghost()
            .compact()
            .tooltip("Plugins")
            .dropdown_menu(move |menu, _, _| {
                let view = view.clone();
                let mut menu = menu.min_w(px(220.));

                menu = menu.item(
                    PopupMenuItem::new("Manage Plugins")
                        .icon(IconName::LayoutDashboard)
                        .on_click({
                            let view = view.clone();
                            move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.sidebar_visible = true;
                                    this.set_sidebar_view(SidebarView::Plugins, cx);
                                });
                            }
                        }),
                );

                menu = menu.separator();

                if manifests.is_empty() && disabled.is_empty() {
                    menu = menu.item(
                        PopupMenuItem::new("No plugins loaded").disabled(true),
                    );
                } else {
                    for manifest in &manifests {
                        let is_enabled = !disabled.contains(&manifest.id);
                        let plugin_id = manifest.id.clone();
                        let label = if is_enabled {
                            format!("✓ {} — {}", manifest.name, manifest.version)
                        } else {
                            format!("  {} — {} (disabled)", manifest.name, manifest.version)
                        };
                        menu = menu.item(PopupMenuItem::new(label).on_click({
                            let view = view.clone();
                            let pid = plugin_id.clone();
                            move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.toggle_plugin(pid.clone(), cx);
                                });
                            }
                        }));
                    }

                    for plugin_id in &disabled {
                        if !manifests.iter().any(|m| &m.id == plugin_id) {
                            let label = format!("  {} (disabled)", plugin_id);
                            menu = menu.item(PopupMenuItem::new(label).on_click({
                                let view = view.clone();
                                let pid = plugin_id.clone();
                                move |_, _, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.toggle_plugin(pid.clone(), cx);
                                    });
                                }
                            }));
                        }
                    }
                }

                menu
            })
    }

    pub(super) fn render_sidebar_toggle(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let is_open = self.sidebar_visible;
        let border_color = if is_open {
            cx.theme().foreground.opacity(0.42)
        } else {
            cx.theme().muted_foreground.opacity(0.55)
        };
        let background_color = if is_open {
            cx.theme().foreground.opacity(0.06)
        } else {
            cx.theme().background
        };
        let hover_border = cx.theme().foreground.opacity(0.38);
        let hover_background = cx.theme().secondary.opacity(0.14);

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
        let view = cx.entity();
        let (doc_status, icon, color) = if self.editor_snapshot.has_conflict {
            ("Conflict", IconName::TriangleAlert, cx.theme().warning)
        } else if self.editor_snapshot.is_missing {
            ("Missing", IconName::TriangleAlert, cx.theme().warning)
        } else if self.editor_snapshot.saving {
            (
                "Saving",
                IconName::LoaderCircle,
                cx.theme().muted_foreground,
            )
        } else if self.editor_snapshot.dirty {
            ("Edited", IconName::Asterisk, cx.theme().muted_foreground)
        } else {
            ("Saved", IconName::CircleCheck, cx.theme().success)
        };
        let find_status = self.active_find_status();

        div()
            .id("status-bar")
            .flex()
            .items_center()
            .gap_4()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border.opacity(0.18))
            .bg(cx.theme().background)
            .text_sm()
            .on_hover(cx.listener(Self::on_status_bar_hover))
            .child(self.render_sidebar_toggle(cx))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_color(cx.theme().muted_foreground)
                    .truncate()
                    .child(self.status_message()),
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
                    .when_some(find_status, |this, find_status| {
                        this.child(
                            div()
                                .text_color(cx.theme().muted_foreground)
                                .child(find_status),
                        )
                    })
                    .child(
                        ButtonGroup::new("editor-view-mode-status")
                            .compact()
                            .ghost()
                            .child(Button::new("view-mode-preview").label("Preview").selected(
                                self.editor_snapshot.view_mode
                                    == editor::EditorViewMode::LivePreview,
                            ))
                            .child(Button::new("view-mode-source").label("Source").selected(
                                self.editor_snapshot.view_mode == editor::EditorViewMode::Source,
                            ))
                            .on_click(move |selected: &Vec<usize>, window, app| {
                                let target = if selected.contains(&1) {
                                    editor::EditorViewMode::Source
                                } else {
                                    editor::EditorViewMode::LivePreview
                                };
                                let _ = view.update(app, |this, cx| {
                                    this.active_editor_entity().update(cx, |editor, cx| {
                                        editor.set_view_mode(target, window, cx);
                                    });
                                });
                            }),
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

    pub(super) fn render_tab_bar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let active_index = self.active_tab_index;
        let tab_count = self.tabs.len();
        let border_color = cx.theme().border.opacity(0.18);
        let muted_foreground = cx.theme().muted_foreground;
        let foreground = cx.theme().foreground;
        let background = cx.theme().background;
        let active_bg = cx.theme().secondary.opacity(0.10);
        let hover_bg = cx.theme().secondary.opacity(0.06);

        div()
            .id("editor-tab-bar")
            .w_full()
            .h(px(36.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(border_color)
            .bg(background)
            .children(self.tabs.iter().enumerate().map(|(i, tab)| {
                let path = tab.editor.read(cx).document_path();
                let title = match path {
                    Some(p) => p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    None => "Untitled".to_string(),
                };
                let is_dirty = tab.editor.read(cx).snapshot().dirty;
                let label = if is_dirty {
                    format!("● {}", title)
                } else {
                    title
                };
                let is_active = i == active_index;
                let view_for_click = view.clone();
                let view_for_menu = view.clone();

                let tab_el = div()
                    .id(("tab", i))
                    .h_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .text_sm()
                    .cursor_pointer()
                    .border_b_2()
                    .border_color(if is_active {
                        cx.theme().primary
                    } else {
                        gpui::Hsla::transparent_black()
                    })
                    .bg(if is_active { active_bg } else { background })
                    .hover(move |style| {
                        if is_active {
                            style
                        } else {
                            style.bg(hover_bg)
                        }
                    })
                    .text_color(if is_active { foreground } else { muted_foreground })
                    .child(
                        div()
                            .max_w(px(160.))
                            .overflow_hidden()
                            .truncate()
                            .child(label),
                    )
                    .on_click(move |_, window, cx| {
                        if let Some(entity) = view_for_click.upgrade() {
                            let _ = entity.update(cx, |this, cx| {
                                this.switch_to_tab(i, window, cx);
                            });
                        }
                    });

                tab_el.context_menu({
                    move |menu, _, _| {
                        build_tab_context_menu(menu, view_for_menu.clone(), i, tab_count)
                    }
                })
            }))
    }
}
