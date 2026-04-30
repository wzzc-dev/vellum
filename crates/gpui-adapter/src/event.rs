use crate::types::Point;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

impl Default for MouseButton {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyCode {
    Backspace,
    Tab,
    Enter,
    Shift,
    Control,
    Alt,
    CapsLock,
    Escape,
    Space,
    PageUp,
    PageDown,
    End,
    Home,
    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
    PrintScreen,
    Insert,
    Delete,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,
    NumpadEqual,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

impl Default for KeyCode {
    fn default() -> Self {
        Self::Enter
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyModifiers {
    None,
    Shift,
    Control,
    Alt,
    Meta,
    CapsLock,
    NumLock,
}

impl KeyModifiers {
    pub fn is_shift(&self) -> bool {
        matches!(self, Self::Shift)
    }

    pub fn is_control(&self) -> bool {
        matches!(self, Self::Control)
    }

    pub fn is_alt(&self) -> bool {
        matches!(self, Self::Alt)
    }

    pub fn is_meta(&self) -> bool {
        matches!(self, Self::Meta)
    }
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum MouseEventKind {
    Move,
    Down,
    Up,
    Enter,
    Leave,
    Wheel,
    DoubleClick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub button: MouseButton,
    pub position: Point,
    pub global_position: Point,
    pub delta: Point,
    pub click_count: u32,
    pub modifiers: KeyModifiers,
}

impl MouseEvent {
    pub fn new(kind: MouseEventKind, position: Point) -> Self {
        Self {
            kind,
            button: MouseButton::None,
            position,
            global_position: position,
            delta: Point::default(),
            click_count: 1,
            modifiers: KeyModifiers::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    pub code: KeyCode,
    pub key: String,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyEventKind {
    Down,
    Up,
    Pressed,
}

impl Default for KeyEventKind {
    fn default() -> Self {
        Self::Pressed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusEvent {
    pub kind: FocusEventKind,
    pub related_target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FocusEventKind {
    FocusIn,
    FocusOut,
}

impl Default for FocusEventKind {
    fn default() -> Self {
        Self::FocusIn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchEvent {
    pub id: u64,
    pub position: Point,
    pub global_position: Point,
    pub force: f32,
    pub radius_x: f32,
    pub radius_y: f32,
    pub rotation: f32,
    pub kind: TouchEventKind,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TouchEventKind {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollEvent {
    pub position: Point,
    pub delta: Point,
    pub kind: ScrollEventKind,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollEventKind {
    Normal,
    Page,
    Begin,
    End,
}

impl Default for ScrollEventKind {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragEvent {
    pub position: Point,
    pub global_position: Point,
    pub kind: DragEventKind,
    pub data: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DragEventKind {
    Enter,
    Over,
    Leave,
    Drop,
}

impl Default for DragEventKind {
    fn default() -> Self {
        Self::Enter
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowEvent {
    pub kind: WindowEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowEventKind {
    CloseRequest,
    Close,
    Resize,
    Move,
    Focus,
    Blur,
    Maximize,
    Minimize,
    Restore,
    EnterFullscreen,
    ExitFullscreen,
    ThemeChanged,
}

impl Default for WindowEventKind {
    fn default() -> Self {
        Self::Close
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionEvent {
    pub kind: CompositionEventKind,
    pub data: String,
    pub cursor: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompositionEventKind {
    Start,
    Update,
    End,
}

impl Default for CompositionEventKind {
    fn default() -> Self {
        Self::End
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum GpuiEvent {
    Mouse {
        widget_id: String,
        event: MouseEvent,
    },
    Key {
        widget_id: String,
        event: KeyEvent,
    },
    Focus {
        widget_id: String,
        event: FocusEvent,
    },
    Touch {
        widget_id: String,
        event: TouchEvent,
    },
    Scroll {
        widget_id: String,
        event: ScrollEvent,
    },
    Drag {
        widget_id: String,
        event: DragEvent,
    },
    Composition {
        widget_id: String,
        event: CompositionEvent,
    },
    Window {
        window_id: u32,
        event: WindowEvent,
    },
}

impl GpuiEvent {
    pub fn widget_id(&self) -> Option<&str> {
        match self {
            Self::Mouse { widget_id, .. } => Some(widget_id),
            Self::Key { widget_id, .. } => Some(widget_id),
            Self::Focus { widget_id, .. } => Some(widget_id),
            Self::Touch { widget_id, .. } => Some(widget_id),
            Self::Scroll { widget_id, .. } => Some(widget_id),
            Self::Drag { widget_id, .. } => Some(widget_id),
            Self::Composition { widget_id, .. } => Some(widget_id),
            Self::Window { .. } => None,
        }
    }

    pub fn window_id(&self) -> Option<u32> {
        match self {
            Self::Window { window_id, .. } => Some(*window_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventType {
    pub name: String,
    pub enabled: bool,
}

impl EventType {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
        }
    }
}

#[derive(Default)]
pub struct EventDispatcher {
    listeners: std::collections::HashMap<String, Vec<Box<dyn Fn(&GpuiEvent) + Send + Sync>>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, widget_id: &str, callback: impl Fn(&GpuiEvent) + Send + Sync + 'static) {
        self.listeners
            .entry(widget_id.to_string())
            .or_default()
            .push(Box::new(callback));
    }

    pub fn unsubscribe(&mut self, widget_id: &str) {
        self.listeners.remove(widget_id);
    }

    pub fn dispatch(&self, event: &GpuiEvent) {
        if let Some(widget_id) = event.widget_id() {
            if let Some(callbacks) = self.listeners.get(widget_id) {
                for callback in callbacks {
                    callback(event);
                }
            }
        }
    }
}
