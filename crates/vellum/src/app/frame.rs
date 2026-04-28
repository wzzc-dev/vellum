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

    pub(super) fn toggle_right_panel_visibility(&mut self, cx: &mut Context<Self>) {
        self.right_panel_visible = !self.right_panel_visible;
        cx.notify();
    }

    pub(super) fn set_right_panel_view(&mut self, view: RightPanelView, cx: &mut Context<Self>) {
        if self.right_panel_view == view && self.right_panel_visible {
            return;
        }

        self.right_panel_visible = true;
        self.right_panel_view = view;
        cx.notify();
    }

    pub(super) fn open_right_panel(&mut self, view: RightPanelView, cx: &mut Context<Self>) {
        self.set_right_panel_view(view, cx);
    }

    fn reveal_right_panel_toggle(&mut self, cx: &mut Context<Self>) {
        self.right_panel_toggle_hide_generation = self.right_panel_toggle_hide_generation.wrapping_add(1);
        if !self.right_panel_toggle_visible {
            self.right_panel_toggle_visible = true;
            cx.notify();
        }
    }

    fn hide_right_panel_toggle(&mut self, cx: &mut Context<Self>) {
        self.right_panel_toggle_hide_generation = self.right_panel_toggle_hide_generation.wrapping_add(1);
        if self.right_panel_toggle_visible && !self.right_panel_visible {
            self.right_panel_toggle_visible = false;
            cx.notify();
        }
    }

    fn schedule_right_panel_toggle_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.right_panel_visible || self.right_panel_toggle_hovered {
            return;
        }

        self.right_panel_toggle_hide_generation = self.right_panel_toggle_hide_generation.wrapping_add(1);
        let generation = self.right_panel_toggle_hide_generation;
        let view = cx.entity();
        window.spawn(cx, async move |cx| {
            Timer::after(Duration::from_secs(3)).await;
            let _ = view.update(cx, |this, cx| {
                if this.right_panel_toggle_hide_generation == generation
                    && !this.right_panel_visible
                    && !this.right_panel_toggle_hovered
                {
                    this.right_panel_toggle_visible = false;
                    cx.notify();
                }
            });
        }).detach();
    }

    pub(super) fn on_right_panel_toggle_hover(&mut self, hovered: &bool, window: &mut Window, cx: &mut Context<Self>) {
        self.right_panel_toggle_hovered = *hovered;
        if *hovered {
            self.reveal_right_panel_toggle(cx);
        } else {
            self.schedule_right_panel_toggle_hide(window, cx);
        }
    }

    pub(super) fn on_right_panel_hit_area_hover(&mut self, hovered: &bool, window: &mut Window, cx: &mut Context<Self>) {
        if *hovered {
            self.reveal_right_panel_toggle(cx);
        } else if !self.right_panel_toggle_hovered {
            self.schedule_right_panel_toggle_hide(window, cx);
        }
    }

    pub(super) fn toggle_extension(&mut self, extension_id: String, cx: &mut Context<Self>) {
        if self.extension_host.registry().is_disabled(&extension_id) {
            self.extension_host.registry_mut().enable(&extension_id);
            // Re-activate the extension
            if let Err(e) = self.extension_host.activate_discovered() {
                eprintln!("failed to re-activate extension {}: {}", extension_id, e);
            }
        } else {
            self.extension_host.registry_mut().disable(&extension_id);
            self.webview_manager.remove_all();
            if let Err(e) = self.extension_host.unload_extension(&extension_id) {
                eprintln!("failed to unload extension {}: {}", extension_id, e);
            }
        }
        cx.notify();
    }

    pub(super) fn loaded_extension_manifests(&self) -> Vec<vellum_extension::manifest::ExtensionManifest> {
        self.extension_host.loaded_manifests()
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
        let recent_files: Vec<PathBuf> = self.recent_files.iter().take(10).cloned().collect();

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
                    .when(!recent_files.is_empty(), |menu| {
                        let recent_files = recent_files.clone();
                        let mut menu = menu.separator();
                        for (i, path) in recent_files.iter().enumerate() {
                            let name = path
                                .file_name()
                                .and_then(|n: &std::ffi::OsStr| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            let label = if i == 0 {
                                format!("① {}", name)
                            } else if i == 1 {
                                format!("② {}", name)
                            } else if i == 2 {
                                format!("③ {}", name)
                            } else if i == 3 {
                                format!("④ {}", name)
                            } else if i == 4 {
                                format!("⑤ {}", name)
                            } else {
                                format!("{}", name)
                            };
                            let path = path.clone();
                            let view = view.clone();
                            menu = menu.item(
                                PopupMenuItem::new(label).on_click(move |_, window, cx| {
                                    let path = path.clone();
                                    let _ = view.update(cx, |this, cx| {
                                        this.open_file(path, window, cx);
                                    });
                                })
                            );
                        }
                        menu.separator()
                    })
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
                                this.open_right_panel(RightPanelView::Plugins, cx);
                            });
                        }
                    }))
            })
    }

    pub(super) fn render_extensions_menu(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let manifests = self.loaded_extension_manifests();
        let disabled_ids: Vec<String> = self.extension_host.registry().disabled_ids().to_vec();

        Button::new("extensions-menu")
            .icon(IconName::Settings)
            .ghost()
            .compact()
            .tooltip("Extensions")
            .dropdown_menu(move |menu, _, _| {
                let view = view.clone();
                let mut menu = menu.min_w(px(220.));

                menu = menu.item(
                    PopupMenuItem::new("Manage Extensions")
                        .icon(IconName::LayoutDashboard)
                        .on_click({
                            let view = view.clone();
                            move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.open_right_panel(RightPanelView::Plugins, cx);
                                });
                            }
                        }),
                );

                menu = menu.separator();

                if manifests.is_empty() && disabled_ids.is_empty() {
                    menu = menu.item(
                        PopupMenuItem::new("No extensions loaded").disabled(true),
                    );
                } else {
                    for manifest in &manifests {
                        let is_enabled = !disabled_ids.contains(&manifest.id);
                        let ext_id = manifest.id.clone();
                        let label = if is_enabled {
                            format!("✓ {} — {}", manifest.name, manifest.version)
                        } else {
                            format!("  {} — {} (disabled)", manifest.name, manifest.version)
                        };
                        menu = menu.item(PopupMenuItem::new(label).on_click({
                            let view = view.clone();
                            let eid = ext_id.clone();
                            move |_, _, cx| {
                                let _ = view.update(cx, |this, cx| {
                                    this.toggle_extension(eid.clone(), cx);
                                });
                            }
                        }));
                    }

                    for ext_id in &disabled_ids {
                        if !manifests.iter().any(|m| &m.id == ext_id) {
                            let label = format!("  {} (disabled)", ext_id);
                            menu = menu.item(PopupMenuItem::new(label).on_click({
                                let view = view.clone();
                                let eid = ext_id.clone();
                                move |_, _, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.toggle_extension(eid.clone(), cx);
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

    pub(super) fn render_right_panel_toggle(&self, cx: &Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let is_open = self.right_panel_visible;
        let should_show = self.right_panel_toggle_visible || self.right_panel_visible;

        let left_color = if is_open {
            cx.theme().primary
        } else {
            cx.theme().foreground
        };
        let right_color = if is_open {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground.opacity(0.4)
        };

        div()
            .id("right-panel-toggle")
            .flex()
            .items_center()
            .justify_center()
            .w(px(28.))
            .h_full()
            .cursor_pointer()
            .when(should_show, |this| {
                this.child(
                    div()
                        .flex()
                        .rounded(px(2.))
                        .overflow_hidden()
                        .child(div().w(px(7.)).h(px(10.)).bg(left_color))
                        .child(div().w(px(7.)).h(px(10.)).bg(right_color)),
                )
            })
            .on_hover(cx.listener(Self::on_right_panel_toggle_hover))
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.toggle_right_panel_visibility(cx);
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
                            .child(Button::new("view-mode-live-preview").label("LivePreview").selected(
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
