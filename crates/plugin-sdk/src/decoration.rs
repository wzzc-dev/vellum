use serde::{Deserialize, Serialize};

use crate::ui::UiNode;

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
    Highlight {
        color: String,
    },
    Strikethrough,
    GutterMark {
        icon: String,
        color: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnderlineStyle {
    Solid,
    Dotted,
    Wavy,
    Dashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tooltip {
    pub content: UiNode,
    pub position: TooltipPosition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TooltipPosition {
    Above,
    Below,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPanel {
    pub id: String,
    pub title: String,
    pub content: UiNode,
    pub position: OverlayPosition,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub close_on_escape: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OverlayPosition {
    Center,
    TopRight,
    BottomRight,
    EditorCenter,
}
