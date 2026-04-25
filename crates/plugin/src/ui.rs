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
    ButtonClicked { element_id: String },
    InputChanged { element_id: String, value: String },
    CheckboxToggled { element_id: String, checked: bool },
    SelectChanged { element_id: String, index: usize },
    ToggleChanged { element_id: String, active: bool },
    LinkClicked { element_id: String },
    ListItemClicked { element_id: String, item_id: String },
    DisclosureToggled { element_id: String, open: bool },
}
