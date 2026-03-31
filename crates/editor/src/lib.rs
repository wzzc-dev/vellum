use std::{
    cmp,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext, Context, EntityInputHandler as _, EventEmitter,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Timer, VisualContext, Window, actions, div,
    px, rems, size,
};
use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariants as _},
    input::{Input, InputEvent, InputState},
    text::{TextView, TextViewStyle},
    v_virtual_list,
};

mod commands;
mod document;
mod layout;
mod session;
mod view;

actions!(
    vellum_editor,
    [
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

pub use commands::bind_keys;
pub use document::{BlockKind, BlockSpan, ConflictState, CursorAnchorPolicy, DocumentState};
pub use session::ActiveBlockSession;
pub use view::{EditorEvent, EditorSnapshot, MarkdownEditor};

const EDITOR_CONTEXT: &str = "MarkdownEditor";
const INPUT_CONTEXT: &str = "MarkdownEditorInput";
const FLUSH_DELAY: Duration = Duration::from_millis(120);
const AUTOSAVE_DELAY: Duration = Duration::from_millis(700);
const MAX_EDITOR_WIDTH: f32 = 780.;
const BODY_FONT_SIZE: f32 = 17.;
const BODY_LINE_HEIGHT: f32 = 28.;
const CODE_FONT_SIZE: f32 = 15.;
const CODE_LINE_HEIGHT: f32 = 24.;
