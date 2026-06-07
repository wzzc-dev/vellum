use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use editor::{
    AlignTableColumnCenter, AlignTableColumnLeft, AlignTableColumnRight, BoldSelection,
    DeleteTableColumn, DeleteTableRow, DemoteBlock, EditorEvent, EditorSnapshot, ExitBlockEdit,
    FocusNextBlock, FocusPrevBlock, InsertCallout, InsertCodeFence, InsertFootnote,
    InsertFrontMatter, InsertHorizontalRule, InsertHtmlBlock, InsertImage, InsertInlineMath,
    InsertMathBlock, InsertMermaidDiagram, InsertTable, InsertTableColumn, InsertTableRow,
    InsertToc, ItalicSelection, LinkSelection, MarkdownEditor, PromoteBlock, RedoEdit,
    SecondaryEnter, ToggleBlockquote, ToggleBulletList, ToggleFocusHighlightMode, ToggleHeading1,
    ToggleHeading2, ToggleHeading3, ToggleHeading4, ToggleHeading5, ToggleHeading6,
    ToggleHighlight, ToggleInlineCode, ToggleOrderedList, ToggleParagraph, ToggleSourceMode,
    ToggleStrikethrough, ToggleSubscript, ToggleSuperscript, ToggleTaskList, ToggleTypewriterMode,
    UndoEdit, bind_keys as bind_editor_keys,
};
use gpui::Focusable;
use gpui::{
    App, AppContext, Application, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, Styled, Subscription, Timer, VisualContext, Window,
    WindowBounds, WindowHandle, WindowOptions, actions, div, px, size,
};
#[cfg(target_os = "macos")]
use gpui::{Menu, MenuItem, OsAction, SystemMenuType};
#[cfg(target_os = "macos")]
use gpui_component::input::{Copy, Cut, Paste, SelectAll};
use gpui_component::input::{InputEvent, InputState};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, TitleBar,
    button::{Button, ButtonVariants as _},
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    tree::TreeState,
};
use rfd::FileDialog;
use workspace::{
    QuickOpenItem, TreeSortMode, WorkspaceEvent, WorkspaceSearchOptions, WorkspaceSearchResult,
    WorkspaceState, is_markdown_path,
};

mod command_palette;
mod commands;
mod document_io;
mod export;
mod frame;
mod layout;
mod preferences;
mod render;

actions!(
    vellum,
    [
        OpenFile,
        OpenFolder,
        NewFile,
        SaveNow,
        SaveAs,
        ExportHtml,
        ExportPrintHtml,
        OpenPreferences,
        Quit,
        ToggleSidebar,
        ToggleStatusBar,
        ToggleFocusMode,
        OpenGotoLine,
        OpenFindPanel,
        CloseFindPanel,
        FindNextMatch,
        FindPreviousMatch,
        OpenFindReplacePanel,
        OpenQuickly,
        OpenGlobalSearch,
        RefreshFileTree,
        ReplaceOne,
        ReplaceAll,
        CloseTab,
        PreviousTab,
        NextTab,
        OpenCommandPalette,
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
    quick_open_visible: bool,
    quick_open_query: String,
    quick_open_items: Vec<QuickOpenItem>,
    quick_open_selected_index: usize,
    global_search_visible: bool,
    global_search_query: String,
    global_search_results: Vec<WorkspaceSearchResult>,
    global_search_selected_index: usize,
    file_filter: String,
    outline_filter: String,
    find_query_input: Entity<InputState>,
    replace_query_input: Entity<InputState>,
    quick_open_input: Entity<InputState>,
    global_search_input: Entity<InputState>,
    file_filter_input: Entity<InputState>,
    outline_filter_input: Entity<InputState>,
    goto_line_visible: bool,
    goto_line_query: String,
    goto_line_input: Entity<InputState>,
    preferences_visible: bool,
    preferences_asset_dir_input: Entity<InputState>,
    /// Kept alive to keep subscriptions active.
    #[allow(dead_code)]
    find_input_subscriptions: Vec<Subscription>,
    // --- file tree rename state ---
    renaming_path: Option<PathBuf>,
    rename_input: Option<Entity<InputState>>,
    // --- pending file opens from drag-drop ---
    pending_file_opens: Vec<PathBuf>,
    recent_files: Vec<PathBuf>,
    focus_mode: bool,
    command_palette: command_palette::CommandPaletteState,
    preferences: preferences::AppPreferences,
}

pub fn run() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        bind_keys(cx);
        bind_editor_keys(cx);

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1024.), px(768.)), cx)),
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
        KeyBinding::new("cmd-alt-e", ExportHtml, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-alt-p", ExportPrintHtml, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-p", OpenQuickly, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-f", OpenFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-alt-f", OpenFindReplacePanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-f", OpenGlobalSearch, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-g", FindNextMatch, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-g", FindPreviousMatch, Some(APP_CONTEXT)),
        KeyBinding::new("escape", CloseFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseTab, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-[", PreviousTab, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-shift-]", NextTab, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-alt-shift-f", ToggleFocusMode, Some(APP_CONTEXT)),
        KeyBinding::new("cmd-l", OpenGotoLine, None),
        KeyBinding::new("cmd-shift-p", OpenCommandPalette, Some(APP_CONTEXT)),
    ]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([
        KeyBinding::new("ctrl-o", OpenFile, None),
        KeyBinding::new("ctrl-shift-o", OpenFolder, None),
        KeyBinding::new("ctrl-n", NewFile, None),
        KeyBinding::new("ctrl-s", SaveNow, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-alt-e", ExportHtml, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-alt-p", ExportPrintHtml, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-p", OpenQuickly, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-f", OpenFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-h", OpenFindReplacePanel, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-f", OpenGlobalSearch, Some(APP_CONTEXT)),
        KeyBinding::new("f3", FindNextMatch, Some(APP_CONTEXT)),
        KeyBinding::new("shift-f3", FindPreviousMatch, Some(APP_CONTEXT)),
        KeyBinding::new("escape", CloseFindPanel, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-alt-shift-f", ToggleFocusMode, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-l", OpenGotoLine, None),
        KeyBinding::new("ctrl-shift-p", OpenCommandPalette, Some(APP_CONTEXT)),
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
    cx.on_action(move |_: &ExportHtml, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.export_html_dialog(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &ExportPrintHtml, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.export_print_html_dialog(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &OpenPreferences, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.open_preferences(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &OpenQuickly, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.open_quickly(window, cx);
        });
    });
    let window = main_window;
    cx.on_action(move |_: &OpenGlobalSearch, cx| {
        update_vellum_app_from_menu(window, cx, |this, window, cx| {
            this.open_global_search(window, cx);
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
    cx.set_menus(vec![
        Menu {
            name: "Vellum".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Preferences...", OpenPreferences),
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
                MenuItem::action("Export HTML...", ExportHtml),
                MenuItem::action("Export and Open for Print...", ExportPrintHtml),
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
                MenuItem::action("Task List", ToggleTaskList),
                MenuItem::separator(),
                MenuItem::action("Insert Horizontal Rule", InsertHorizontalRule),
                MenuItem::action("Insert Code Fence", InsertCodeFence),
                MenuItem::action("Insert Mermaid Diagram", InsertMermaidDiagram),
                MenuItem::action("Insert Inline Math", InsertInlineMath),
                MenuItem::action("Insert Math Block", InsertMathBlock),
                MenuItem::action("Insert HTML Block", InsertHtmlBlock),
                MenuItem::action("Insert Callout", InsertCallout),
                MenuItem::action("Insert Table of Contents", InsertToc),
                MenuItem::action("Insert Footnote", InsertFootnote),
                MenuItem::action("Insert Front Matter", InsertFrontMatter),
                MenuItem::action("Insert Image", InsertImage),
                MenuItem::action("Insert Table", InsertTable),
                MenuItem::action("Table: Insert Row", InsertTableRow),
                MenuItem::action("Table: Delete Row", DeleteTableRow),
                MenuItem::action("Table: Insert Column", InsertTableColumn),
                MenuItem::action("Table: Delete Column", DeleteTableColumn),
                MenuItem::action("Table: Align Column Left", AlignTableColumnLeft),
                MenuItem::action("Table: Align Column Center", AlignTableColumnCenter),
                MenuItem::action("Table: Align Column Right", AlignTableColumnRight),
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
                MenuItem::action("Inline Code", ToggleInlineCode),
                MenuItem::action("Strikethrough", ToggleStrikethrough),
                MenuItem::action("Highlight", ToggleHighlight),
                MenuItem::action("Superscript", ToggleSuperscript),
                MenuItem::action("Subscript", ToggleSubscript),
                MenuItem::separator(),
                MenuItem::action("Insert Link", LinkSelection),
            ],
        },
        Menu {
            name: "Find".into(),
            items: vec![
                MenuItem::action("Open Quickly", OpenQuickly),
                MenuItem::action("Global Search", OpenGlobalSearch),
                MenuItem::separator(),
                MenuItem::action("Find", OpenFindPanel),
                MenuItem::action("Find and Replace", OpenFindReplacePanel),
                MenuItem::action("Find Next", FindNextMatch),
                MenuItem::action("Find Previous", FindPreviousMatch),
                MenuItem::separator(),
                MenuItem::action("Go to Line", OpenGotoLine),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Source Mode", ToggleSourceMode),
                MenuItem::separator(),
                MenuItem::action("Toggle Focus Mode", ToggleFocusMode),
                MenuItem::action("Toggle Typewriter Mode", ToggleTypewriterMode),
                MenuItem::action("Toggle Focus Highlight", ToggleFocusHighlightMode),
                MenuItem::separator(),
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle Status Bar", ToggleStatusBar),
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
        let mut preferences = preferences::load_preferences();
        editor::set_syntax_theme(preferences.syntax_theme);
        preferences.font_size = editor::set_body_font_size(preferences.font_size);
        let editor = cx.new(|cx| MarkdownEditor::new(window, cx));
        editor.update(cx, |editor, cx| {
            editor.set_image_asset_dir(preferences.image_asset_dir.clone());
            editor.set_typewriter_mode(preferences.typewriter_mode, window, cx);
            editor.set_focus_highlight_mode(preferences.focus_highlight_mode, cx);
            editor.set_view_mode(preferences.view_mode, window, cx);
        });
        let focus_handle = cx.focus_handle();
        let editor_snapshot = editor.read(cx).snapshot();
        let find_query_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find"));
        let replace_query_input = cx.new(|cx| InputState::new(window, cx).placeholder("Replace"));
        let quick_open_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Open quickly..."));
        let global_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search workspace..."));
        let file_filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Filter files"));
        let outline_filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Filter outline"));
        let goto_line_input = cx.new(|cx| InputState::new(window, cx).placeholder("Go to line"));
        let preferences_asset_dir_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("assets"));

        let editor_subscription =
            cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| match event {
                EditorEvent::Changed(snapshot) => {
                    this.editor_snapshot = snapshot.clone();
                    this.remember_view_mode_preference(snapshot.view_mode);
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
                EditorEvent::ViewSettingsChanged {
                    typewriter_mode,
                    focus_highlight_mode,
                } => {
                    this.set_editor_view_mode_preferences(
                        *typewriter_mode,
                        *focus_highlight_mode,
                        cx,
                    );
                }
            });

        let find_input_subscription = cx.subscribe(
            &find_query_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = this.find_query_input.read(cx).value();
                    this.set_find_query(value);
                    cx.notify();
                }
            },
        );

        let replace_input_subscription = cx.subscribe(
            &replace_query_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.replace_query = this.replace_query_input.read(cx).value().to_string();
                }
            },
        );

        let quick_open_input_subscription = cx.subscribe(
            &quick_open_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = this.quick_open_input.read(cx).value();
                    this.set_quick_open_query(value);
                    cx.notify();
                }
            },
        );

        let global_search_input_subscription = cx.subscribe(
            &global_search_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = this.global_search_input.read(cx).value();
                    this.set_global_search_query(value);
                    cx.notify();
                }
            },
        );

        let outline_input_subscription = cx.subscribe(
            &outline_filter_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = this.outline_filter_input.read(cx).value();
                    this.set_outline_filter(value);
                    cx.notify();
                }
            },
        );

        let file_input_subscription = cx.subscribe(
            &file_filter_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let value = this.file_filter_input.read(cx).value();
                    this.set_file_filter(value);
                    cx.notify();
                }
            },
        );

        let goto_line_input_subscription = cx.subscribe(
            &goto_line_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    this.goto_line_query = this.goto_line_input.read(cx).value().to_string();
                    cx.notify();
                }
            },
        );

        let preferences_asset_dir_subscription = cx.subscribe(
            &preferences_asset_dir_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    if this.preferences_visible {
                        let value = this.preferences_asset_dir_input.read(cx).value();
                        this.set_image_asset_dir_preference(value, cx);
                        cx.notify();
                    }
                }
            },
        );

        let palette_state = command_palette::CommandPaletteState::new(
            cx.new(|cx| InputState::new(window, cx).placeholder("Search commands...")),
        );

        let mut this = Self {
            app_state: AppState::default(),
            workspace: WorkspaceState::new(),
            tree_state,
            tabs: vec![EditorTab { editor }],
            active_tab_index: 0,
            focus_handle,
            editor_snapshot,
            sidebar_visible: preferences.sidebar_visible,
            sidebar_view: SidebarView::Files,
            status_bar_pinned: preferences.status_bar_pinned,
            status_bar_visible: preferences.status_bar_pinned,
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
            quick_open_visible: false,
            quick_open_query: String::new(),
            quick_open_items: Vec::new(),
            quick_open_selected_index: 0,
            global_search_visible: false,
            global_search_query: String::new(),
            global_search_results: Vec::new(),
            global_search_selected_index: 0,
            file_filter: String::new(),
            outline_filter: String::new(),
            find_query_input,
            replace_query_input,
            quick_open_input,
            global_search_input,
            file_filter_input,
            outline_filter_input,
            goto_line_visible: false,
            goto_line_query: String::new(),
            goto_line_input,
            preferences_visible: false,
            preferences_asset_dir_input,
            find_input_subscriptions: vec![
                editor_subscription,
                find_input_subscription,
                replace_input_subscription,
                quick_open_input_subscription,
                global_search_input_subscription,
                outline_input_subscription,
                file_input_subscription,
                goto_line_input_subscription,
                preferences_asset_dir_subscription,
                cx.subscribe(
                    &palette_state.input,
                    |this: &mut Self, _: Entity<InputState>, event: &InputEvent, cx| {
                        if let InputEvent::Change = event {
                            let value = this.command_palette.input.read(cx).value();
                            this.command_palette.update_filter(&value);
                            cx.notify();
                        }
                    },
                ),
            ],
            renaming_path: None,
            rename_input: None,
            pending_file_opens: Vec::new(),
            recent_files: crate::path::read_recent_files(),
            focus_mode: preferences.focus_mode,
            command_palette: palette_state,
            preferences,
        };
        window.focus(&this.focus_handle);

        this.restore_last_opened_document(window, cx);

        this
    }

    fn active_editor(&self) -> Option<&Entity<MarkdownEditor>> {
        self.tabs.get(self.active_tab_index).map(|tab| &tab.editor)
    }

    fn active_editor_entity(&self) -> Entity<MarkdownEditor> {
        self.tabs[self.active_tab_index].editor.clone()
    }

    fn new_configured_editor(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<MarkdownEditor> {
        let editor = cx.new(|cx| MarkdownEditor::new(window, cx));
        self.apply_editor_preferences(&editor, window, cx);
        editor
    }

    fn apply_editor_preferences(
        &self,
        editor: &Entity<MarkdownEditor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        editor.update(cx, |editor, cx| {
            editor.set_image_asset_dir(self.preferences.image_asset_dir.clone());
            editor.set_typewriter_mode(self.preferences.typewriter_mode, window, cx);
            editor.set_focus_highlight_mode(self.preferences.focus_highlight_mode, cx);
            editor.set_view_mode(self.preferences.view_mode, window, cx);
        });
    }

    fn active_editor_mut(&mut self) -> Option<&mut Entity<MarkdownEditor>> {
        self.tabs
            .get_mut(self.active_tab_index)
            .map(|tab| &mut tab.editor)
    }

    fn open_editor_tab(
        &mut self,
        editor: Entity<MarkdownEditor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
        let keep = self
            .tabs
            .remove(self.active_tab_index.min(self.tabs.len() - 1));
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
            let subscription =
                cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| match event {
                    EditorEvent::Changed(snapshot) => {
                        this.editor_snapshot = snapshot.clone();
                        this.remember_view_mode_preference(snapshot.view_mode);
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
                    EditorEvent::ViewSettingsChanged {
                        typewriter_mode,
                        focus_highlight_mode,
                    } => {
                        this.set_editor_view_mode_preferences(
                            *typewriter_mode,
                            *focus_highlight_mode,
                            cx,
                        );
                    }
                });
            self.find_input_subscriptions.push(subscription);
        }
    }

    fn save_preferences(&mut self) {
        if let Err(err) = preferences::save_preferences(&self.preferences) {
            self.set_status(format!("Failed to save preferences: {err}"));
        }
    }

    fn open_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences_visible = true;
        self.preferences_asset_dir_input.update(cx, |input, cx| {
            input.set_value(self.preferences.image_asset_dir.clone(), window, cx);
        });
        cx.notify();
    }

    fn close_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences_visible = false;
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn set_image_asset_dir_preference(&mut self, value: impl AsRef<str>, cx: &mut Context<Self>) {
        let normalized = preferences::normalize_image_asset_dir(value.as_ref());
        if self.preferences.image_asset_dir == normalized {
            return;
        }

        self.preferences.image_asset_dir = normalized.clone();
        for tab in &self.tabs {
            tab.editor.update(cx, |editor, _| {
                editor.set_image_asset_dir(normalized.clone());
            });
        }
        self.save_preferences();
    }

    fn set_editor_view_mode_preferences(
        &mut self,
        typewriter_mode: bool,
        focus_highlight_mode: bool,
        cx: &mut Context<Self>,
    ) {
        let changed = self.preferences.typewriter_mode != typewriter_mode
            || self.preferences.focus_highlight_mode != focus_highlight_mode;
        self.preferences.typewriter_mode = typewriter_mode;
        self.preferences.focus_highlight_mode = focus_highlight_mode;

        for tab in &self.tabs {
            tab.editor.update(cx, |editor, cx| {
                editor.set_typewriter_mode_without_window(typewriter_mode, cx);
                editor.set_focus_highlight_mode(focus_highlight_mode, cx);
            });
        }

        if changed {
            self.save_preferences();
        }
        cx.notify();
    }

    fn remember_view_mode_preference(&mut self, view_mode: editor::EditorViewMode) {
        if self.preferences.view_mode == view_mode {
            return;
        }
        self.preferences.view_mode = view_mode;
        self.save_preferences();
    }

    fn set_view_mode_preference(
        &mut self,
        view_mode: editor::EditorViewMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.remember_view_mode_preference(view_mode);
        self.active_editor_entity().update(cx, |editor, cx| {
            editor.set_view_mode(view_mode, window, cx);
        });
        cx.notify();
    }

    fn set_font_size_preference(&mut self, size: u16, cx: &mut Context<Self>) {
        let normalized = editor::set_body_font_size(size);
        if self.preferences.font_size == normalized {
            return;
        }

        self.preferences.font_size = normalized;
        for tab in &self.tabs {
            tab.editor.update(cx, |_, cx| cx.notify());
        }
        self.save_preferences();
        cx.notify();
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

    fn set_quick_open_query(&mut self, query: impl Into<String>) {
        let query = query.into();
        if self.quick_open_query == query {
            return;
        }
        self.quick_open_query = query;
        self.refresh_quick_open_items();
    }

    fn set_global_search_query(&mut self, query: impl Into<String>) {
        let query = query.into();
        if self.global_search_query == query {
            return;
        }
        self.global_search_query = query;
        self.refresh_global_search_results();
    }

    fn set_outline_filter(&mut self, filter: impl Into<String>) {
        self.outline_filter = filter.into();
    }

    fn set_file_filter(&mut self, filter: impl Into<String>) {
        self.file_filter = filter.into();
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

    fn open_quickly(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.command_palette.hide();
        self.preferences_visible = false;
        self.global_search_visible = false;
        self.goto_line_visible = false;
        self.quick_open_visible = true;
        self.quick_open_query.clear();
        self.quick_open_input.update(cx, |input, cx| {
            input.set_value(String::new(), window, cx);
        });
        self.refresh_quick_open_items();
        let focus = self.quick_open_input.focus_handle(cx);
        window.focus(&focus);
        cx.notify();
    }

    fn close_quick_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.quick_open_visible = false;
        self.quick_open_query.clear();
        self.quick_open_items.clear();
        self.quick_open_selected_index = 0;
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn refresh_quick_open_items(&mut self) {
        match self.workspace.quick_open_items(&self.quick_open_query, 80) {
            Ok(items) => {
                self.quick_open_items = items;
                if self.quick_open_selected_index >= self.quick_open_items.len() {
                    self.quick_open_selected_index = 0;
                }
            }
            Err(err) => {
                self.quick_open_items.clear();
                self.quick_open_selected_index = 0;
                self.set_status(format!("Open Quickly failed: {err}"));
            }
        }
    }

    fn select_next_quick_open_item(&mut self) {
        if self.quick_open_items.is_empty() {
            self.quick_open_selected_index = 0;
        } else {
            self.quick_open_selected_index =
                (self.quick_open_selected_index + 1) % self.quick_open_items.len();
        }
    }

    fn select_previous_quick_open_item(&mut self) {
        if self.quick_open_items.is_empty() {
            self.quick_open_selected_index = 0;
        } else if self.quick_open_selected_index == 0 {
            self.quick_open_selected_index = self.quick_open_items.len() - 1;
        } else {
            self.quick_open_selected_index -= 1;
        }
    }

    fn apply_quick_open_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self
            .quick_open_items
            .get(self.quick_open_selected_index)
            .cloned()
        else {
            self.set_status("No quick-open match".to_string());
            cx.notify();
            return;
        };
        let offset = item
            .heading
            .as_ref()
            .map(|heading| heading.source_offset)
            .unwrap_or(0);
        self.quick_open_visible = false;
        self.quick_open_query.clear();
        self.open_file_at_source_range(item.path, offset..offset, window, cx);
    }

    fn open_global_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.command_palette.hide();
        self.preferences_visible = false;
        self.quick_open_visible = false;
        self.goto_line_visible = false;
        self.global_search_visible = true;
        self.global_search_query.clear();
        self.global_search_input.update(cx, |input, cx| {
            input.set_value(String::new(), window, cx);
        });
        self.refresh_global_search_results();
        let focus = self.global_search_input.focus_handle(cx);
        window.focus(&focus);
        cx.notify();
    }

    fn close_global_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.global_search_visible = false;
        self.global_search_query.clear();
        self.global_search_results.clear();
        self.global_search_selected_index = 0;
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn refresh_global_search_results(&mut self) {
        let options = self.workspace_search_options();
        match self
            .workspace
            .search_markdown(&self.global_search_query, options)
        {
            Ok(results) => {
                self.global_search_results = results;
                if self.global_search_selected_index >= self.global_search_match_count() {
                    self.global_search_selected_index = 0;
                }
            }
            Err(err) => {
                self.global_search_results.clear();
                self.global_search_selected_index = 0;
                self.set_status(format!("Global search failed: {err}"));
            }
        }
    }

    fn workspace_search_options(&self) -> WorkspaceSearchOptions {
        WorkspaceSearchOptions {
            case_sensitive: self.find_case_sensitive,
            whole_word: self.find_whole_word,
            use_regex: self.find_regex,
        }
    }

    fn set_find_case_sensitive(&mut self, value: bool) {
        self.find_case_sensitive = value;
        self.refresh_find_matches();
        self.refresh_global_search_results();
    }

    fn set_find_whole_word(&mut self, value: bool) {
        self.find_whole_word = value;
        self.refresh_find_matches();
        self.refresh_global_search_results();
    }

    fn set_find_regex(&mut self, value: bool) {
        self.find_regex = value;
        self.refresh_find_matches();
        self.refresh_global_search_results();
    }

    fn global_search_match_count(&self) -> usize {
        self.global_search_results
            .iter()
            .map(|result| result.matches.len())
            .sum()
    }

    fn selected_global_search_match(&self) -> Option<(usize, usize)> {
        let mut remaining = self.global_search_selected_index;
        for (result_index, result) in self.global_search_results.iter().enumerate() {
            if remaining < result.matches.len() {
                return Some((result_index, remaining));
            }
            remaining -= result.matches.len();
        }
        None
    }

    fn select_next_global_search_match(&mut self) {
        let count = self.global_search_match_count();
        if count == 0 {
            self.global_search_selected_index = 0;
        } else {
            self.global_search_selected_index = (self.global_search_selected_index + 1) % count;
        }
    }

    fn select_previous_global_search_match(&mut self) {
        let count = self.global_search_match_count();
        if count == 0 {
            self.global_search_selected_index = 0;
        } else if self.global_search_selected_index == 0 {
            self.global_search_selected_index = count - 1;
        } else {
            self.global_search_selected_index -= 1;
        }
    }

    fn apply_global_search_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((result_index, match_index)) = self.selected_global_search_match() else {
            self.set_status("No global-search match".to_string());
            cx.notify();
            return;
        };
        let result = self.global_search_results[result_index].clone();
        let text_match = result.matches[match_index].clone();
        self.open_file_at_source_range(result.path, text_match.range, window, cx);
    }

    fn open_file_at_source_range(
        &mut self,
        path: PathBuf,
        range: std::ops::Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_file(path.clone(), window, cx);
        self.active_editor_entity().update(cx, |editor, cx| {
            editor.select_source_range(range.start, range.end, window, cx);
        });
        self.editor_snapshot = self.active_editor_entity().read(cx).snapshot();
        self.workspace.select_file(path);
        self.refresh_tree(cx);
        cx.notify();
    }

    fn set_tree_sort_mode(&mut self, mode: TreeSortMode, cx: &mut Context<Self>) {
        let mut sort = self.workspace.tree_sort();
        sort.mode = mode;
        self.workspace.set_tree_sort(sort);
        self.refresh_tree(cx);
    }

    fn toggle_tree_directories_first(&mut self, cx: &mut Context<Self>) {
        let mut sort = self.workspace.tree_sort();
        sort.directories_first = !sort.directories_first;
        self.workspace.set_tree_sort(sort);
        self.refresh_tree(cx);
    }

    fn refresh_workspace_navigation(&mut self, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
        if self.quick_open_visible {
            self.refresh_quick_open_items();
        }
        if self.global_search_visible {
            self.refresh_global_search_results();
        }
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
                    .position(|item| {
                        item.range.start <= current_cursor && current_cursor <= item.range.end
                    })
                    .or(Some(0))
            };
        }

        self.editor_snapshot.find_matches =
            self.find_matches.iter().map(|m| m.range.clone()).collect();
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
            Ok(re) => re.find_iter(haystack).map(|m| m.start()..m.end()).collect(),
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
