use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use editor::{
    BoldSelection, DemoteBlock, EditorEvent, EditorSnapshot, ExitBlockEdit, FocusNextBlock,
    FocusPrevBlock, InsertCodeFence, InsertHorizontalRule, InsertTable, ItalicSelection,
    LinkSelection, MarkdownEditor, PromoteBlock, RedoEdit, SecondaryEnter, ToggleBlockquote,
    ToggleBulletList, ToggleHeading1, ToggleHeading2, ToggleHeading3, ToggleHeading4, ToggleHeading5,
    ToggleHeading6, ToggleOrderedList, ToggleParagraph, ToggleSourceMode, UndoEdit,
    bind_keys as bind_editor_keys,
};
use gpui::{
    App, AppContext, Application, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, Styled, Subscription, Timer, VisualContext, Window,
    WindowHandle, WindowOptions, actions, div, px,
};
use gpui_component::input::{InputEvent, InputState};
#[cfg(target_os = "macos")]
use gpui::{Menu, MenuItem, OsAction, SystemMenuType};
#[cfg(target_os = "macos")]
use gpui_component::input::{Copy, Cut, Paste, SelectAll};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, TitleBar,
    button::{Button, ButtonVariants as _},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    tree::TreeState,
};
use rfd::FileDialog;
use workspace::{WorkspaceEvent, WorkspaceState, is_markdown_path};
use vellum_extension::ExtensionHost;

use webview::WebViewManager;

mod commands;
mod document_io;
mod frame;
mod layout;
mod render;
mod webview;

actions!(
    vellum,
    [
        OpenFile,
        OpenFolder,
        NewFile,
        SaveNow,
        SaveAs,
        Quit,
        ToggleSidebar,
        ToggleRightPanel,
        ToggleStatusBar,
        ToggleFocusMode,
        OpenFindPanel,
        CloseFindPanel,
        FindNextMatch,
        FindPreviousMatch,
        OpenFindReplacePanel,
        ReplaceOne,
        ReplaceAll,
        CloseTab,
        PreviousTab,
        NextTab,
        ManagePlugins,
    ]
);

const APP_CONTEXT: &str = "VellumApp";
const WATCH_POLL_DELAY: Duration = Duration::from_millis(250);
const STATUS_BAR_HIDE_DELAY: Duration = Duration::from_secs(3);
const STATUS_BAR_REVEAL_EDGE_HEIGHT: f32 = 12.;

#[derive(Default)]
struct AppState {
    workspace_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SidebarView {
    #[default]
    Files,
    Outline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RightPanelView {
    #[default]
    Plugins,
    Plugin(u32),
}

/// A single find match: byte offset range in the source document.
#[derive(Debug, Clone)]
pub(super) struct FindMatch {
    pub(super) range: std::ops::Range<usize>,
}

struct EditorTab {
    editor: Entity<MarkdownEditor>,
}

struct VellumApp {
    app_state: AppState,
    workspace: WorkspaceState,
    tree_state: Entity<TreeState>,
    tabs: Vec<EditorTab>,
    active_tab_index: usize,
    focus_handle: FocusHandle,
    editor_snapshot: EditorSnapshot,
    sidebar_visible: bool,
    sidebar_view: SidebarView,
    right_panel_visible: bool,
    right_panel_view: RightPanelView,
    right_panel_toggle_visible: bool,
    right_panel_toggle_hovered: bool,
    right_panel_toggle_hide_generation: u64,
    status_bar_pinned: bool,
    status_bar_visible: bool,
    status_bar_hovered: bool,
    status_bar_edge_hovered: bool,
    status_bar_hide_generation: u64,
    shell_status_message: String,
    // --- find panel state ---
    find_panel_visible: bool,
    find_query: String,
    find_matches: Vec<FindMatch>,
    active_find_index: Option<usize>,
    find_case_sensitive: bool,
    find_whole_word: bool,
    find_regex: bool,
    replace_visible: bool,
    replace_query: String,
    outline_filter: String,
    find_query_input: Entity<InputState>,
    replace_query_input: Entity<InputState>,
    outline_filter_input: Entity<InputState>,
    /// Kept alive to keep subscriptions active.
    #[allow(dead_code)]
    find_input_subscriptions: Vec<Subscription>,
    // --- file tree rename state ---
    renaming_path: Option<PathBuf>,
    rename_input: Option<Entity<InputState>>,
    // --- extension system ---
    extension_host: ExtensionHost,
    // --- pending file opens from drag-drop ---
    pending_file_opens: Vec<PathBuf>,
    recent_files: Vec<PathBuf>,
    disclosure_state: HashMap<String, bool>,
    webview_manager: WebViewManager,
    focus_mode: bool,
}

pub fn run() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        bind_keys(cx);
        bind_editor_keys(cx);

        let options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        let main_window = cx
            .open_window(options, |window, cx| {
                window.set_window_title("Vellum");
                let view = cx.new(|cx| VellumApp::new(window, cx));
                VellumApp::start_background_tasks(&view, window, cx);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open main window");
        install_app_menus(cx, main_window);

        cx.activate(true);
    });

    Ok(())
}

fn bind_keys(cx: &mut App) {
    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("cmd-o", OpenFile, None),
        KeyBinding::new("cmd-shift-o", OpenFolder, None),
        KeyBinding::new("cmd-n", NewFile, None),
        KeyBinding::new("cmd-s", SaveNow, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-s", SaveAs, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-f", OpenFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-alt-f", OpenFindReplacePanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-g", FindNextMatch, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-g", FindPreviousMatch, Some(APP_CONTEXT)),
        KeyBinding::new("escape", CloseFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseTab, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-[", PreviousTab, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-]", NextTab, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-alt-\\", ToggleRightPanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-f", ToggleFocusMode, Some(APP_CONTEXT)),
    ]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([
        KeyBinding::new("ctrl-o", OpenFile, None),
        KeyBinding::new("ctrl-shift-o", OpenFolder, None),
        KeyBinding::new("ctrl-n", NewFile, None),
        KeyBinding::new("ctrl-s", SaveNow, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-f", OpenFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-h", OpenFindReplacePanel, Some(APP_CONTEXT)),
        KeyBinding::new("f3", FindNextMatch, Some(APP_CONTEXT)),
        KeyBinding::new("shift-f3", FindPreviousMatch, Some(APP_CONTEXT)),
        KeyBinding::new("escape", CloseFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-f", ToggleFocusMode, Some(APP_CONTEXT)),
    ]);
}

#[cfg(target_os = "macos")]
fn install_app_menus(cx: &mut App, main_window: WindowHandle<Root>) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    let window = main_window;
    cx.on_action(move |_: &NewFile, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.create_new_file(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &OpenFile, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.open_file_dialog(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &OpenFolder, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.request_open_folder(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &CloseTab, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.on_close_tab(&CloseTab, window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &PreviousTab, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.on_previous_tab(&PreviousTab, window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &NextTab, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.on_next_tab(&NextTab, window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &ManagePlugins, cx| {
        update_vellum_app_from_menu(window, cx, |this, _window, cx| {
            this.open_right_panel(RightPanelView::Plugins, cx);
        });
    });
    cx.set_menus(vec![
        Menu {
            name: "Vellum".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit Vellum", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New File", NewFile),
                MenuItem::separator(),
                MenuItem::action("Open File...", OpenFile),
                MenuItem::action("Open Folder...", OpenFolder),
                MenuItem::separator(),
                MenuItem::action("Save", SaveNow),
                MenuItem::action("Save As...", SaveAs),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", UndoEdit, OsAction::Undo),
                MenuItem::os_action("Redo", RedoEdit, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::separator(),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "Paragraph".into(),
            items: vec![
                MenuItem::action("Paragraph", ToggleParagraph),
                MenuItem::action("Heading 1", ToggleHeading1),
                MenuItem::action("Heading 2", ToggleHeading2),
                MenuItem::action("Heading 3", ToggleHeading3),
                MenuItem::action("Heading 4", ToggleHeading4),
                MenuItem::action("Heading 5", ToggleHeading5),
                MenuItem::action("Heading 6", ToggleHeading6),
                MenuItem::separator(),
                MenuItem::action("Blockquote", ToggleBlockquote),
                MenuItem::action("Bullet List", ToggleBulletList),
                MenuItem::action("Ordered List", ToggleOrderedList),
                MenuItem::separator(),
                MenuItem::action("Insert Horizontal Rule", InsertHorizontalRule),
                MenuItem::action("Insert Code Fence", InsertCodeFence),
                MenuItem::action("Insert Table", InsertTable),
                MenuItem::separator(),
                MenuItem::action("Insert Line Break", SecondaryEnter),
                MenuItem::separator(),
                MenuItem::action("Indent Paragraph", DemoteBlock),
                MenuItem::action("Outdent Paragraph", PromoteBlock),
                MenuItem::separator(),
                MenuItem::action("Move to Previous Block", FocusPrevBlock),
                MenuItem::action("Move to Next Block", FocusNextBlock),
                MenuItem::separator(),
                MenuItem::action("Exit Current Block", ExitBlockEdit),
            ],
        },
        Menu {
            name: "Format".into(),
            items: vec![
                MenuItem::action("Bold", BoldSelection),
                MenuItem::action("Italic", ItalicSelection),
                MenuItem::separator(),
                MenuItem::action("Insert Link", LinkSelection),
            ],
        },
        Menu {
            name: "Find".into(),
            items: vec![
                MenuItem::action("Find", OpenFindPanel),
                MenuItem::action("Find and Replace", OpenFindReplacePanel),
                MenuItem::action("Find Next", FindNextMatch),
                MenuItem::action("Find Previous", FindPreviousMatch),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Source Mode", ToggleSourceMode),
                MenuItem::separator(),
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle Right Panel", ToggleRightPanel),
                MenuItem::action("Toggle Status Bar", ToggleStatusBar),
            ],
        },
        Menu {
            name: "Plugins".into(),
            items: vec![
                MenuItem::action("Manage Plugins...", ManagePlugins),
            ],
        },
    ]);
}

#[cfg(target_os = "macos")]
fn update_vellum_app_from_menu(
    window_handle: WindowHandle<Root>,
    cx: &mut App,
    update: impl FnOnce(&mut VellumApp, &mut Window, &mut Context<VellumApp>),
) {
    let _ = window_handle.update(cx, |root, window, cx| {
        if let Ok(app) = root.view().clone().downcast::<VellumApp>() {
            let _ = app.update(cx, |this, cx| {
                update(this, window, cx);
            });
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn install_app_menus(_: &mut App, _: WindowHandle<Root>) {}

impl VellumApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let editor = cx.new(|cx| MarkdownEditor::new(window, cx));
        let focus_handle = cx.focus_handle();
        let editor_snapshot = editor.read(cx).snapshot();
        let find_query_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find"));
        let replace_query_input = cx.new(|cx| InputState::new(window, cx).placeholder("Replace"));
        let outline_filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Filter outline"));

        let editor_subscription = cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| {
            match event {
                EditorEvent::Changed(snapshot) => {
                    this.editor_snapshot = snapshot.clone();
                    if !snapshot.status_message.is_empty() {
                        this.shell_status_message.clear();
                    }
                    this.refresh_find_matches();
                    cx.notify();
                }
                EditorEvent::OpenFile(path) => {
                    this.pending_file_opens.push(path.clone());
                    cx.notify();
                }
            }
        });

        let find_input_subscription = cx.subscribe(&find_query_input, |this: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let value = this.find_query_input.read(cx).value();
                this.set_find_query(value);
                cx.notify();
            }
        });

        let replace_input_subscription = cx.subscribe(&replace_query_input, |this: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                this.replace_query = this.replace_query_input.read(cx).value().to_string();
            }
        });

        let outline_input_subscription = cx.subscribe(&outline_filter_input, |this: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let value = this.outline_filter_input.read(cx).value();
                this.set_outline_filter(value);
                cx.notify();
            }
        });

        let mut this = Self {
            app_state: AppState::default(),
            workspace: WorkspaceState::new(),
            tree_state,
            tabs: vec![EditorTab { editor }],
            active_tab_index: 0,
            focus_handle,
            editor_snapshot,
            sidebar_visible: true,
            sidebar_view: SidebarView::Files,
            right_panel_visible: false,
            right_panel_view: RightPanelView::Plugins,
            right_panel_toggle_visible: false,
            right_panel_toggle_hovered: false,
            right_panel_toggle_hide_generation: 0,
            status_bar_pinned: true,
            status_bar_visible: true,
            status_bar_hovered: false,
            status_bar_edge_hovered: false,
            status_bar_hide_generation: 0,
            shell_status_message: String::new(),
            find_panel_visible: false,
            find_query: String::new(),
            find_matches: Vec::new(),
            active_find_index: None,
            find_case_sensitive: false,
            find_whole_word: false,
            find_regex: false,
            replace_visible: false,
            replace_query: String::new(),
            outline_filter: String::new(),
            find_query_input,
            replace_query_input,
            outline_filter_input,
            find_input_subscriptions: vec![editor_subscription, find_input_subscription, replace_input_subscription, outline_input_subscription],
            renaming_path: None,
            rename_input: None,
            extension_host: ExtensionHost::new().unwrap_or_else(|e| {
                tracing::error!("failed to initialize extension host: {}", e);
                ExtensionHost::new().unwrap()
            }),
            pending_file_opens: Vec::new(),
            recent_files: crate::path::read_recent_files(),
            disclosure_state: HashMap::new(),
            webview_manager: WebViewManager::new(),
            focus_mode: false,
        };
        window.focus(&this.focus_handle);
        this.restore_last_opened_document(window, cx);

        // Discover extensions in ~/.vellum/extensions/
        if let Some(ext_dir) = dirs::home_dir().map(|d| d.join(".vellum").join("extensions")) {
            if let Ok(discovered) = this.extension_host.discover_in_dir(&ext_dir) {
                for id in &discovered {
                    eprintln!("discovered extension: {}", id);
                }
            }
        }

        // Discover extensions in app bundle directory
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let local_ext_dir = exe_dir.join("extensions");
                if local_ext_dir.exists() {
                    if let Ok(discovered) = this.extension_host.discover_in_dir(&local_ext_dir) {
                        for id in &discovered {
                            eprintln!("discovered extension: {}", id);
                        }
                    }
                }
            }
        }

        // Activate all discovered extensions
        if let Ok(activated) = this.extension_host.activate_discovered() {
            for id in &activated {
                eprintln!("activated extension: {}", id);
            }
        }

        this
    }

    fn active_editor(&self) -> Option<&Entity<MarkdownEditor>> {
        self.tabs.get(self.active_tab_index).map(|tab| &tab.editor)
    }

    fn active_editor_entity(&self) -> Entity<MarkdownEditor> {
        self.tabs[self.active_tab_index].editor.clone()
    }

    fn active_editor_mut(&mut self) -> Option<&mut Entity<MarkdownEditor>> {
        self.tabs.get_mut(self.active_tab_index).map(|tab| &mut tab.editor)
    }

    fn open_editor_tab(&mut self, editor: Entity<MarkdownEditor>, window: &mut Window, cx: &mut Context<Self>) {
        let new_tab = EditorTab { editor };
        self.tabs.push(new_tab);
        self.active_tab_index = self.tabs.len() - 1;
        self.subscribe_active_editor(window, cx);
        cx.notify();
    }

    fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.tabs.remove(self.active_tab_index);
        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        }
        self.subscribe_active_editor(window, cx);
        cx.notify();
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.active_tab_index > index {
            self.active_tab_index -= 1;
        } else if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        }
        self.subscribe_active_editor(window, cx);
        cx.notify();
    }

    fn close_other_tabs(&mut self, keep_index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if keep_index >= self.tabs.len() {
            return;
        }
        let keep = self.tabs.remove(keep_index);
        self.tabs.clear();
        self.tabs.push(keep);
        self.active_tab_index = 0;
        self.subscribe_active_editor(window, cx);
        cx.notify();
    }

    fn close_all_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            return;
        }
        let keep = self.tabs.remove(self.active_tab_index.min(self.tabs.len() - 1));
        self.tabs.clear();
        self.tabs.push(keep);
        self.active_tab_index = 0;
        self.subscribe_active_editor(window, cx);
        cx.notify();
    }

    fn switch_to_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.tabs.len() && index != self.active_tab_index {
            self.active_tab_index = index;
            self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
            self.subscribe_active_editor(window, cx);
            cx.notify();
        }
    }

    fn subscribe_active_editor(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = self.active_editor() {
            let editor = editor.clone();
            let subscription = cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| {
                match event {
                    EditorEvent::Changed(snapshot) => {
                        this.editor_snapshot = snapshot.clone();
                        if !snapshot.status_message.is_empty() {
                            this.shell_status_message.clear();
                        }
                        this.refresh_find_matches();

                        let text = snapshot.document_text.clone();
                        let path = snapshot.path.as_ref().map(|p| p.to_string_lossy().to_string());
                        this.extension_host.update_document(text.clone(), path.clone());
                        this.extension_host.dispatch_event(
                            "document.changed",
                            snapshot.path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default().as_str(),
                            &text,
                            path.as_deref(),
                        );

                        cx.notify();
                    }
                    EditorEvent::OpenFile(path) => {
                         this.pending_file_opens.push(path.clone());
                         cx.notify();
                     }
                }
            });
            self.find_input_subscriptions.push(subscription);
        }
    }

    fn start_background_tasks(view: &Entity<Self>, window: &mut Window, cx: &mut App) {
        let view = view.clone();
        window
            .spawn(cx, async move |cx| {
                loop {
                    Timer::after(WATCH_POLL_DELAY).await;
                    if cx
                        .update_window_entity(&view, |this, window, cx| {
                            this.poll_workspace(window, cx);
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
    }
    fn set_find_query(&mut self, query: impl Into<String>) {
        let query = query.into();
        if self.find_query == query {
            return;
        }
        self.find_query = query;
        self.refresh_find_matches();
    }

    fn set_outline_filter(&mut self, filter: impl Into<String>) {
        self.outline_filter = filter.into();
    }

    fn open_find_panel(&mut self) {
        self.find_panel_visible = true;
        self.refresh_find_matches();
    }

    fn open_find_replace_panel(&mut self) {
        self.find_panel_visible = true;
        self.replace_visible = true;
        self.refresh_find_matches();
    }

    fn close_find_panel(&mut self) {
        self.find_panel_visible = false;
        self.replace_visible = false;
    }

    fn refresh_find_matches(&mut self) {
        if self.find_query.is_empty() {
            self.find_matches.clear();
            self.active_find_index = None;
        } else {
            self.find_matches = find_matches_ext(
                &self.editor_snapshot.document_text,
                &self.find_query,
                self.find_case_sensitive,
                self.find_whole_word,
                self.find_regex,
            )
            .into_iter()
            .map(|range| FindMatch { range })
            .collect();

            self.active_find_index = if self.find_matches.is_empty() {
                None
            } else {
                let current_cursor = self.editor_snapshot.selection.cursor();
                self.find_matches
                    .iter()
                    .position(|item| item.range.start <= current_cursor && current_cursor <= item.range.end)
                    .or(Some(0))
            };
        }

        self.editor_snapshot.find_matches = self.find_matches.iter().map(|m| m.range.clone()).collect();
        self.editor_snapshot.active_find_index = self.active_find_index;
    }

    fn navigate_find_match(&mut self, backwards: bool) -> Option<usize> {
        if self.find_matches.is_empty() {
            self.active_find_index = None;
            return None;
        }

        let len = self.find_matches.len();
        let next_index = match self.active_find_index {
            Some(current) if backwards => (current + len - 1) % len,
            Some(current) => (current + 1) % len,
            None if backwards => len - 1,
            None => 0,
        };
        self.active_find_index = Some(next_index);
        Some(self.find_matches[next_index].range.start)
    }

    fn active_find_status(&self) -> Option<String> {
        if !self.find_panel_visible {
            return None;
        }
        if self.find_query.is_empty() {
            return Some("Find".to_string());
        }
        if self.find_matches.is_empty() {
            return Some("No matches".to_string());
        }
        let current = self.active_find_index.unwrap_or(0) + 1;
        Some(format!("Find {current}/{}", self.find_matches.len()))
    }

    fn replace_current_match(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active_find_index else {
            return;
        };
        let Some(find_match) = self.find_matches.get(index) else {
            return;
        };
        let range = find_match.range.clone();
        let replacement = self.replace_query.clone();
        self.active_editor_entity().update(cx, |editor, cx| {
            editor.replace_source_range(range, replacement, window, cx);
        });
    }

    fn replace_all_matches(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.find_matches.is_empty() {
            return;
        }
        let replacement = self.replace_query.clone();
        for find_match in self.find_matches.iter().rev() {
            let range = find_match.range.clone();
            let replacement = replacement.clone();
            self.active_editor_entity().update(cx, |editor, cx| {
                editor.replace_source_range(range, replacement, window, cx);
            });
        }
    }
}

fn is_whole_word(text: &str, start: usize, end: usize) -> bool {
    let before_is_boundary = start == 0
        || !text.as_bytes()[start - 1].is_ascii_alphanumeric()
            && text.as_bytes()[start - 1] != b'_';
    let after_is_boundary = end >= text.len()
        || !text.as_bytes()[end].is_ascii_alphanumeric() && text.as_bytes()[end] != b'_';
    before_is_boundary && after_is_boundary
}

fn find_matches_ext(
    haystack: &str,
    needle: &str,
    case_sensitive: bool,
    whole_word: bool,
    use_regex: bool,
) -> Vec<std::ops::Range<usize>> {
    if needle.is_empty() {
        return Vec::new();
    }

    if use_regex {
        let pattern = if whole_word {
            format!(r"\b(?:{})\b", needle)
        } else {
            needle.to_string()
        };
        let re = regex::RegexBuilder::new(&pattern)
            .case_insensitive(!case_sensitive)
            .build();
        match re {
            Ok(re) => re
                .find_iter(haystack)
                .map(|m| m.start()..m.end())
                .collect(),
            Err(_) => Vec::new(),
        }
    } else if !case_sensitive {
        let needle_lower = needle.to_lowercase();
        let haystack_lower = haystack.to_lowercase();
        let mut results = Vec::new();
        let mut start = 0;
        while let Some(pos) = haystack_lower[start..].find(&needle_lower) {
            let abs_start = start + pos;
            let abs_end = abs_start + needle.len();
            if whole_word && !is_whole_word(haystack, abs_start, abs_end) {
                start = abs_start + 1;
                continue;
            }
            results.push(abs_start..abs_end);
            start = abs_start + 1;
        }
        results
    } else {
        let mut results = Vec::new();
        for (start, matched) in haystack.match_indices(needle) {
            let end = start + matched.len();
            if whole_word && !is_whole_word(haystack, start, end) {
                continue;
            }
            results.push(start..end);
        }
        results
    }
}
