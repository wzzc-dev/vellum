use crate::ui::UiNode;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Decoration {
    pub id: String,
    pub start: usize,
    pub end: usize,
    pub kind: DecorationKind,
    pub tooltip: Option<String>,
    pub hover_data: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DecorationKind {
    Underline {
        color: String,
        style: UnderlineStyle,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum UnderlineStyle {
    Solid,
    Dashed,
    Dotted,
    Wavy,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tooltip {
    pub content: UiNode,
    pub position: TooltipPosition,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum TooltipPosition {
    Above,
    Below,
    Left,
    Right,
}
