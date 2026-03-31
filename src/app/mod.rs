use std::{
    cmp,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use anyhow::Result;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext, Application, Context, Entity, EntityInputHandler as _,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Timer, VisualContext, Window, WindowOptions,
    actions, div, px, rems, size,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Root, Sizable as _, TitleBar,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    list::ListItem,
    menu::{DropdownMenu, PopupMenuItem},
    resizable::{h_resizable, resizable_panel},
    text::{TextView, TextViewStyle},
    tree::{TreeState, tree},
    v_virtual_list,
};
use rfd::FileDialog;

use crate::{
    editor::{ActiveBlockSession, BlockKind, ConflictState, DocumentState},
    workspace::{WorkspaceEvent, WorkspaceState, is_markdown_path},
};

mod commands;
mod document_io;
mod editing;
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
        ToggleSidebar,
        BoldSelection,
        ItalicSelection,
        LinkSelection,
        PromoteBlock,
        DemoteBlock,
        ExitBlockEdit,
        FocusPrevBlock,
        FocusNextBlock,
    ]
);

const APP_CONTEXT: &str = "VellumApp";
const INPUT_CONTEXT: &str = "Input";
const FLUSH_DELAY: Duration = Duration::from_millis(120);
const AUTOSAVE_DELAY: Duration = Duration::from_millis(700);
const WATCH_POLL_DELAY: Duration = Duration::from_millis(250);
const MAX_EDITOR_WIDTH: f32 = 780.;
const BODY_FONT_SIZE: f32 = 17.;
const BODY_LINE_HEIGHT: f32 = 28.;
const CODE_FONT_SIZE: f32 = 15.;
const CODE_LINE_HEIGHT: f32 = 24.;

pub fn run() -> Result<()> {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        bind_keys(cx);

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
        KeyBinding::new("ctrl-b", BoldSelection, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-i", ItalicSelection, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-k", LinkSelection, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-[", PromoteBlock, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-]", DemoteBlock, Some(APP_CONTEXT)),
        KeyBinding::new("escape", ExitBlockEdit, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(APP_CONTEXT)),
        KeyBinding::new("ctrl-s", SaveNow, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-b", BoldSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-i", ItalicSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-k", LinkSelection, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-[", PromoteBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-]", DemoteBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-up", FocusPrevBlock, Some(INPUT_CONTEXT)),
        KeyBinding::new("ctrl-down", FocusNextBlock, Some(INPUT_CONTEXT)),
    ]);
}

#[derive(Default)]
struct AppState {
    workspace_root: Option<PathBuf>,
}

struct VellumApp {
    app_state: AppState,
    workspace: WorkspaceState,
    tree_state: Entity<TreeState>,
    document: DocumentState,
    active_session: Option<ActiveBlockSession>,
    input_subscription: Option<Subscription>,
    sidebar_visible: bool,
    status_message: SharedString,
    flush_generation: u64,
    autosave_generation: u64,
}

impl VellumApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|cx| TreeState::new(cx));

        let mut this = Self {
            app_state: AppState {
                workspace_root: None,
            },
            workspace: WorkspaceState::new(),
            tree_state,
            document: DocumentState::new_empty(None, None),
            active_session: None,
            input_subscription: None,
            sidebar_visible: false,
            status_message: SharedString::from(""),
            flush_generation: 0,
            autosave_generation: 0,
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
