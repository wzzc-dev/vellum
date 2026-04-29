use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredCommand {
    pub qualified_id: String,
    pub command_id: String,
    pub label: String,
    pub key_binding: Option<String>,
    pub extension_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredPanel {
    pub qualified_id: String,
    pub panel_id: String,
    pub label: String,
    pub icon: String,
    pub extension_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingEdit {
    Insert {
        position: usize,
        text: String,
    },
    ReplaceRange {
        start: usize,
        end: usize,
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedPayload<T> {
    pub version: u32,
    pub data: T,
}

impl<T> VersionedPayload<T> {
    pub fn new(data: T) -> Self {
        Self { version: 1, data }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decoration {
    pub id: String,
    pub start: usize,
    pub end: usize,
    pub kind: DecorationKind,
    pub tooltip: Option<String>,
    pub hover_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecorationKind {
    Underline {
        color: String,
        style: UnderlineStyle,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnderlineStyle {
    Solid,
    Dashed,
    Dotted,
    Wavy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tooltip {
    pub content: crate::ui::UiNode,
    pub position: TooltipPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TooltipPosition {
    Above,
    Below,
    Left,
    Right,
}
