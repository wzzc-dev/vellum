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
        ToggleSidebar
    ]
);

const APP_CONTEXT: &str = "VellumApp";
const WATCH_POLL_DELAY: Duration = Duration::from_millis(250);

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
    shell_status_message: String,
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
    cx.bind_keys([
        KeyBinding::new("ctrl-o", OpenFile, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-o", OpenFolder, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-n", NewFile, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-s", SaveNow, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(APP_CONTEXT)),
    ]);
}

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
