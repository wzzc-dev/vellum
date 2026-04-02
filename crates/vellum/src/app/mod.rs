use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use editor::{EditorEvent, EditorSnapshot, MarkdownEditor, bind_keys as bind_editor_keys};
use gpui::{
    App, AppContext, Application, Context, Entity, InteractiveElement, IntoElement, KeyBinding,
    ParentElement, Render, Styled, Timer, VisualContext, Window, WindowOptions, actions, div, px,
};
#[cfg(target_os = "macos")]
use gpui::{Menu, MenuItem, SystemMenuType}; 
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, TitleBar,
    button::{Button, ButtonVariants as _},
    list::ListItem,
    menu::PopupMenuItem,
    resizable::{h_resizable, resizable_panel},
    tree::{TreeState, tree},
};
use rfd::FileDialog;
use workspace::{WorkspaceEvent, WorkspaceState, is_markdown_path};

mod commands;
mod document_io;
mod frame;
mod layout;
mod render;

actions!(
    vellum,
    [
        OpenFile,
        OpenFolder,
        NewFile,
        SaveNow,
        SaveAs,
        Quit,
        ToggleSidebar
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

struct VellumApp {
    app_state: AppState,
    workspace: WorkspaceState,
    tree_state: Entity<TreeState>,
    editor: Entity<MarkdownEditor>,
    editor_snapshot: EditorSnapshot,
    sidebar_visible: bool,
    status_bar_visible: bool,
    status_bar_hovered: bool,
    status_bar_edge_hovered: bool,
    status_bar_hide_generation: u64,
    shell_status_message: String,
}

pub fn run() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        bind_keys(cx);
        bind_editor_keys(cx);
        install_app_menus(cx);

        let options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            window.set_window_title("Vellum");
            let view = cx.new(|cx| VellumApp::new(window, cx));
            VellumApp::start_background_tasks(&view, window, cx);
            cx.new(|cx| Root::new(view, window, cx))
        })
        .expect("failed to open main window");

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
        KeyBinding::new("cmd-q", Quit, None)
    ]);

    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([
        KeyBinding::new("ctrl-o", OpenFile, None),
        KeyBinding::new("ctrl-shift-o", OpenFolder, None),
        KeyBinding::new("ctrl-n", NewFile, None),
        KeyBinding::new("ctrl-s", SaveNow, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(APP_CONTEXT)),
    ]);

}

#[cfg(target_os = "macos")]
fn install_app_menus(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &NewFile, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |root, window, cx| {
                let Ok(root) = root.downcast::<Root>() else {
                    return;
                };
                if let Ok(app) = root.read(cx).view().clone().downcast::<VellumApp>() {
                    let _ = app.update(cx, |this, cx| {
                        this.create_new_file(window, cx);
                    });
                }
            });
        }
    });
    cx.on_action(|_: &OpenFile, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |root, window, cx| {
                let Ok(root) = root.downcast::<Root>() else {
                    return;
                };
                if let Ok(app) = root.read(cx).view().clone().downcast::<VellumApp>() {
                    let _ = app.update(cx, |this, cx| {
                        this.open_file_dialog(window, cx);
                    });
                }
            });
        }
    });
    cx.on_action(|_: &OpenFolder, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |root, window, cx| {
                let Ok(root) = root.downcast::<Root>() else {
                    return;
                };
                if let Ok(app) = root.read(cx).view().clone().downcast::<VellumApp>() {
                    let _ = app.update(cx, |this, cx| {
                        this.request_open_folder(window, cx);
                    });
                }
            });
        }
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
            ],
        },
    ]);
}

#[cfg(not(target_os = "macos"))]
fn install_app_menus(_: &mut App) {}

impl VellumApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let editor = cx.new(|cx| MarkdownEditor::new(window, cx));
        let editor_snapshot = editor.read(cx).snapshot();

        cx.subscribe(&editor, |this, _, event: &EditorEvent, cx| {
            let EditorEvent::Changed(snapshot) = event;
            this.editor_snapshot = snapshot.clone();
            if !snapshot.status_message.is_empty() {
                this.shell_status_message.clear();
            }
            cx.notify();
        })
        .detach();

        let mut this = Self {
            app_state: AppState::default(),
            workspace: WorkspaceState::new(),
            tree_state,
            editor,
            editor_snapshot,
            sidebar_visible: false,
            status_bar_visible: false,
            status_bar_hovered: false,
            status_bar_edge_hovered: false,
            status_bar_hide_generation: 0,
            shell_status_message: String::new(),
        };
        this.restore_last_opened_document(window, cx);
        this
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
}
