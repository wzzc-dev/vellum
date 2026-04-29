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

impl UiNode {
    pub fn column() -> ColumnBuilder {
        ColumnBuilder::new()
    }

    pub fn row() -> RowBuilder {
        RowBuilder::new()
    }

    pub fn text(content: &str) -> Self {
        Self::Text {
            content: content.into(),
            style: TextStyle::default(),
        }
    }

    pub fn styled_text(content: &str, style: TextStyle) -> Self {
        Self::Text {
            content: content.into(),
            style,
        }
    }

    pub fn heading(content: &str, level: u8) -> Self {
        Self::Heading {
            content: content.into(),
            level: level.clamp(1, 6),
        }
    }

    pub fn button(id: &str, label: &str) -> ButtonBuilder {
        ButtonBuilder::new(id, label)
    }

    pub fn text_input(id: &str, placeholder: &str) -> TextInputBuilder {
        TextInputBuilder::new(id, placeholder)
    }

    pub fn checkbox(id: &str, label: &str, checked: bool) -> Self {
        Self::Checkbox {
            id: id.into(),
            label: label.into(),
            checked,
        }
    }

    pub fn select(id: &str, options: &[&str]) -> SelectBuilder {
        SelectBuilder::new(id, options)
    }

    pub fn toggle(id: &str, label: &str, active: bool) -> Self {
        Self::Toggle {
            id: id.into(),
            label: label.into(),
            active,
        }
    }

    pub fn badge(label: &str) -> BadgeBuilder {
        BadgeBuilder::new(label)
    }

    pub fn progress(value: f32) -> Self {
        Self::Progress {
            value: value.clamp(0.0, 1.0),
            label: None,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }

    pub fn spacer() -> Self {
        Self::Spacer
    }

    pub fn list(items: Vec<ListItem>) -> Self {
        Self::List { items }
    }

    pub fn disclosure(label: &str) -> DisclosureBuilder {
        DisclosureBuilder::new(label)
    }

    pub fn link(id: &str, label: &str) -> Self {
        Self::Link {
            id: id.into(),
            label: label.into(),
        }
    }

    pub fn webview(id: &str, url: &str) -> WebViewBuilder {
        WebViewBuilder::new(id, url)
    }
}

pub struct ColumnBuilder {
    children: Vec<UiNode>,
    gap: Option<f32>,
    padding: Option<EdgeInsets>,
    scrollable: bool,
}

impl ColumnBuilder {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: None,
            padding: None,
            scrollable: false,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(EdgeInsets::uniform(padding));
        self
    }

    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    pub fn child(mut self, child: UiNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<UiNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Column {
            children: self.children,
            gap: self.gap,
            padding: self.padding,
            scrollable: self.scrollable,
        }
    }
}

pub struct RowBuilder {
    children: Vec<UiNode>,
    gap: Option<f32>,
    padding: Option<EdgeInsets>,
}

impl RowBuilder {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            gap: None,
            padding: None,
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(EdgeInsets::uniform(padding));
        self
    }

    pub fn child(mut self, child: UiNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<UiNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Row {
            children: self.children,
            gap: self.gap,
            padding: self.padding,
        }
    }
}

pub struct ButtonBuilder {
    id: String,
    label: String,
    variant: ButtonVariant,
    icon: Option<String>,
    disabled: bool,
}

impl ButtonBuilder {
    pub fn new(id: &str, label: &str) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Secondary,
            icon: None,
            disabled: false,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Button {
            id: self.id,
            label: self.label,
            variant: self.variant,
            icon: self.icon,
            disabled: self.disabled,
        }
    }
}

pub struct TextInputBuilder {
    id: String,
    placeholder: String,
    value: String,
    single_line: bool,
}

impl TextInputBuilder {
    pub fn new(id: &str, placeholder: &str) -> Self {
        Self {
            id: id.into(),
            placeholder: placeholder.into(),
            value: String::new(),
            single_line: true,
        }
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = value.into();
        self
    }

    pub fn single_line(mut self, single_line: bool) -> Self {
        self.single_line = single_line;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::TextInput {
            id: self.id,
            placeholder: self.placeholder,
            value: self.value,
            single_line: self.single_line,
        }
    }
}

pub struct SelectBuilder {
    id: String,
    options: Vec<String>,
    selected: Option<usize>,
}

impl SelectBuilder {
    pub fn new(id: &str, options: &[&str]) -> Self {
        Self {
            id: id.into(),
            options: options.iter().map(|s| s.to_string()).collect(),
            selected: None,
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Select {
            id: self.id,
            options: self.options,
            selected: self.selected,
        }
    }
}

pub struct BadgeBuilder {
    label: String,
    severity: Option<Severity>,
}

impl BadgeBuilder {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            severity: None,
        }
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Badge {
            label: self.label,
            severity: self.severity,
        }
    }
}

pub struct DisclosureBuilder {
    label: String,
    open: bool,
    children: Vec<UiNode>,
}

impl DisclosureBuilder {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            open: false,
            children: Vec::new(),
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn child(mut self, child: UiNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<UiNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::Disclosure {
            label: self.label,
            open: self.open,
            children: self.children,
        }
    }
}

pub struct WebViewBuilder {
    id: String,
    url: String,
    allow_scripts: bool,
    allow_devtools: bool,
}

impl WebViewBuilder {
    pub fn new(id: &str, url: &str) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            allow_scripts: false,
            allow_devtools: false,
        }
    }

    pub fn allow_scripts(mut self, allow: bool) -> Self {
        self.allow_scripts = allow;
        self
    }

    pub fn allow_devtools(mut self, allow: bool) -> Self {
        self.allow_devtools = allow;
        self
    }

    pub fn build(self) -> UiNode {
        UiNode::WebView {
            id: self.id,
            url: self.url,
            allow_scripts: self.allow_scripts,
            allow_devtools: self.allow_devtools,
        }
    }
}
