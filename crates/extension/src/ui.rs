use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiNode {
    Column {
        children: Vec<UiNode>,
        gap: Option<f32>,
        padding: Option<EdgeInsets>,
        scrollable: bool,
    },
    Row {
        children: Vec<UiNode>,
        gap: Option<f32>,
        padding: Option<EdgeInsets>,
    },
    Text {
        content: String,
        style: TextStyle,
    },
    Heading {
        content: String,
        level: u8,
    },
    Button {
        id: String,
        label: String,
        variant: ButtonVariant,
        icon: Option<String>,
        disabled: bool,
    },
    TextInput {
        id: String,
        placeholder: String,
        value: String,
        single_line: bool,
    },
    Checkbox {
        id: String,
        label: String,
        checked: bool,
    },
    Select {
        id: String,
        options: Vec<String>,
        selected: Option<usize>,
    },
    Toggle {
        id: String,
        label: String,
        active: bool,
    },
    Badge {
        label: String,
        severity: Option<Severity>,
    },
    Progress {
        value: f32,
        label: Option<String>,
    },
    Separator,
    Spacer,
    List {
        items: Vec<ListItem>,
    },
    Conditional {
        condition: bool,
        when_true: Box<UiNode>,
        when_false: Option<Box<UiNode>>,
    },
    Disclosure {
        label: String,
        open: bool,
        children: Vec<UiNode>,
    },
    Link {
        id: String,
        label: String,
    },
    WebView {
        id: String,
        url: String,
        allow_scripts: bool,
        allow_devtools: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub fn uniform(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextStyle {
    pub size: Option<f32>,
    pub color: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub monospace: Option<bool>,
}

impl TextStyle {
    pub fn small() -> Self {
        Self {
            size: Some(11.0),
            ..Default::default()
        }
    }

    pub fn muted(mut self) -> Self {
        self.color = Some("muted-foreground".into());
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = Some(true);
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = Some(true);
        self
    }

    pub fn monospace(mut self) -> Self {
        self.monospace = Some(true);
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub severity: Option<Severity>,
    pub children: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiEvent {
    ButtonClicked {
        panel_id: String,
        element_id: String,
    },
    InputChanged {
        panel_id: String,
        element_id: String,
        value: String,
    },
    CheckboxToggled {
        panel_id: String,
        element_id: String,
        checked: bool,
    },
    SelectChanged {
        panel_id: String,
        element_id: String,
        index: usize,
    },
    ToggleChanged {
        panel_id: String,
        element_id: String,
        active: bool,
    },
    LinkClicked {
        panel_id: String,
        element_id: String,
    },
    ListItemClicked {
        panel_id: String,
        element_id: String,
        item_id: String,
    },
    DisclosureToggled {
        panel_id: String,
        element_id: String,
        open: bool,
    },
}

impl UiEvent {
    pub fn panel_id(&self) -> &str {
        match self {
            Self::ButtonClicked { panel_id, .. }
            | Self::InputChanged { panel_id, .. }
            | Self::CheckboxToggled { panel_id, .. }
            | Self::SelectChanged { panel_id, .. }
            | Self::ToggleChanged { panel_id, .. }
            | Self::LinkClicked { panel_id, .. }
            | Self::ListItemClicked { panel_id, .. }
            | Self::DisclosureToggled { panel_id, .. } => panel_id,
        }
    }
}

impl UiNode {
    pub fn contains_webview(&self) -> bool {
        match self {
            Self::WebView { .. } => true,
            Self::Column { children, .. }
            | Self::Row { children, .. }
            | Self::Disclosure { children, .. } => children.iter().any(Self::contains_webview),
            Self::List { items } => items.iter().any(list_item_contains_webview),
            Self::Conditional {
                when_true,
                when_false,
                ..
            } => {
                when_true.contains_webview()
                    || when_false
                        .as_deref()
                        .map(Self::contains_webview)
                        .unwrap_or(false)
            }
            Self::Text { .. }
            | Self::Heading { .. }
            | Self::Button { .. }
            | Self::TextInput { .. }
            | Self::Checkbox { .. }
            | Self::Select { .. }
            | Self::Toggle { .. }
            | Self::Badge { .. }
            | Self::Progress { .. }
            | Self::Separator
            | Self::Spacer
            | Self::Link { .. } => false,
        }
    }
}

fn list_item_contains_webview(item: &ListItem) -> bool {
    item.children.iter().any(list_item_contains_webview)
}
