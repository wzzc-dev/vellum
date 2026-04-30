use gpui::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use gpui_component::ActiveTheme;

use crate::EditCommand;

pub(crate) struct SlashCommandItem {
    pub label: &'static str,
    pub keywords: &'static [&'static str],
    pub description: &'static str,
    pub command: SlashCommandAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlashCommandAction {
    Heading1,
    Heading2,
    Heading3,
    BulletList,
    OrderedList,
    TaskList,
    Blockquote,
    CodeFence,
    Table,
    HorizontalRule,
    MathBlock,
    Image,
    Link,
}

impl SlashCommandAction {
    pub fn to_edit_command(&self) -> EditCommand {
        match self {
            Self::Heading1 => EditCommand::ToggleHeading { depth: 1 },
            Self::Heading2 => EditCommand::ToggleHeading { depth: 2 },
            Self::Heading3 => EditCommand::ToggleHeading { depth: 3 },
            Self::BulletList => EditCommand::ToggleBulletList,
            Self::OrderedList => EditCommand::ToggleOrderedList,
            Self::Blockquote => EditCommand::ToggleBlockquote,
            Self::CodeFence => EditCommand::InsertCodeFence,
            Self::Table => EditCommand::InsertTable,
            Self::HorizontalRule => EditCommand::InsertHorizontalRule,
            Self::TaskList => EditCommand::ToggleBulletList,
            Self::MathBlock => EditCommand::InsertMathBlock,
            Self::Image | Self::Link => EditCommand::ToggleInlineMarkup {
                before: if *self == SlashCommandAction::Image {
                    "![](".to_string()
                } else {
                    "[](".to_string()
                },
                after: ")".to_string(),
            },
        }
    }

    pub fn insert_text(&self) -> Option<&'static str> {
        match self {
            Self::TaskList => Some("- [ ] "),
            Self::MathBlock => Some("$$\n\n$$"),
            Self::Image => Some("![]()"),
            Self::Link => Some("[]()"),
            _ => None,
        }
    }
}

const SLASH_COMMANDS: &[SlashCommandItem] = &[
    SlashCommandItem {
        label: "Heading 1",
        keywords: &["h1", "heading1", "title", "标题1", "一级标题"],
        description: "Large heading",
        command: SlashCommandAction::Heading1,
    },
    SlashCommandItem {
        label: "Heading 2",
        keywords: &["h2", "heading2", "标题2", "二级标题"],
        description: "Medium heading",
        command: SlashCommandAction::Heading2,
    },
    SlashCommandItem {
        label: "Heading 3",
        keywords: &["h3", "heading3", "标题3", "三级标题"],
        description: "Small heading",
        command: SlashCommandAction::Heading3,
    },
    SlashCommandItem {
        label: "Bullet List",
        keywords: &["bullet", "list", "ul", "unordered", "无序列表", "列表"],
        description: "Unordered list item",
        command: SlashCommandAction::BulletList,
    },
    SlashCommandItem {
        label: "Numbered List",
        keywords: &["number", "ordered", "ol", "有序列表", "编号列表"],
        description: "Ordered list item",
        command: SlashCommandAction::OrderedList,
    },
    SlashCommandItem {
        label: "Task List",
        keywords: &["task", "todo", "checkbox", "任务列表", "待办"],
        description: "Task list with checkbox",
        command: SlashCommandAction::TaskList,
    },
    SlashCommandItem {
        label: "Quote",
        keywords: &["quote", "blockquote", "引用", "块引用"],
        description: "Block quotation",
        command: SlashCommandAction::Blockquote,
    },
    SlashCommandItem {
        label: "Code Block",
        keywords: &["code", "fence", "代码块", "代码"],
        description: "Fenced code block",
        command: SlashCommandAction::CodeFence,
    },
    SlashCommandItem {
        label: "Table",
        keywords: &["table", "表格"],
        description: "Insert a table",
        command: SlashCommandAction::Table,
    },
    SlashCommandItem {
        label: "Divider",
        keywords: &["hr", "divider", "horizontal", "rule", "分隔线", "水平线"],
        description: "Horizontal divider",
        command: SlashCommandAction::HorizontalRule,
    },
    SlashCommandItem {
        label: "Math Block",
        keywords: &["math", "latex", "formula", "公式", "数学"],
        description: "Math formula block",
        command: SlashCommandAction::MathBlock,
    },
    SlashCommandItem {
        label: "Image",
        keywords: &["image", "img", "picture", "图片", "照片"],
        description: "Insert an image",
        command: SlashCommandAction::Image,
    },
    SlashCommandItem {
        label: "Link",
        keywords: &["link", "url", "hyperlink", "链接", "超链接"],
        description: "Insert a link",
        command: SlashCommandAction::Link,
    },
];

fn filter_commands(query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    SLASH_COMMANDS
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            let label_match = item.label.to_lowercase().contains(&query);
            let keyword_match = item
                .keywords
                .iter()
                .any(|k| k.to_lowercase().contains(&query));
            label_match || keyword_match
        })
        .map(|(i, _)| i)
        .collect()
}

const ITEM_HEIGHT: f32 = 30.;
const PANEL_MAX_HEIGHT: f32 = 280.;
const PANEL_WIDTH: f32 = 280.;

pub struct SlashCommandPanel {
    visible: bool,
    query: String,
    slash_visible_offset: usize,
    slash_source_offset: usize,
    filtered_indices: Vec<usize>,
    selected_index: usize,
    scroll_handle: ScrollHandle,
}

impl SlashCommandPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            slash_visible_offset: 0,
            slash_source_offset: 0,
            filtered_indices: (0..SLASH_COMMANDS.len()).collect(),
            selected_index: 0,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, slash_visible_offset: usize, slash_source_offset: usize) {
        self.visible = true;
        self.query.clear();
        self.slash_visible_offset = slash_visible_offset;
        self.slash_source_offset = slash_source_offset;
        self.filtered_indices = (0..SLASH_COMMANDS.len()).collect();
        self.selected_index = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.query.clear();
    }

    pub fn update_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.filtered_indices = filter_commands(&self.query);
        self.selected_index = 0;
    }

    pub fn slash_visible_offset(&self) -> usize {
        self.slash_visible_offset
    }

    pub fn slash_source_offset(&self) -> usize {
        self.slash_source_offset
    }

    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
            self.scroll_to_selected();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_indices.len() - 1
            } else {
                self.selected_index - 1
            };
            self.scroll_to_selected();
        }
    }

    fn scroll_to_selected(&self) {
        let item_top = self.selected_index as f32 * ITEM_HEIGHT;
        let viewport = self.scroll_handle.bounds();
        let viewport_top: f32 = (-self.scroll_handle.offset().y).into();
        let viewport_bottom = viewport_top + f32::from(viewport.size.height);

        if item_top < viewport_top {
            self.scroll_handle
                .set_offset(gpui::point(self.scroll_handle.offset().x, px(-(item_top))));
        } else if item_top + ITEM_HEIGHT > viewport_bottom {
            self.scroll_handle.set_offset(gpui::point(
                self.scroll_handle.offset().x,
                px(-(item_top + ITEM_HEIGHT - f32::from(viewport.size.height))),
            ));
        }
    }

    pub fn selected_command(&self) -> Option<SlashCommandAction> {
        self.filtered_indices
            .get(self.selected_index)
            .map(|&i| SLASH_COMMANDS[i].command)
    }

    pub fn query_len(&self) -> usize {
        self.query.len()
    }

    pub fn render_panel(
        &self,
        _window: &mut Window,
        cx: &mut App,
        cursor_y: Option<f32>,
    ) -> Option<gpui::AnyElement> {
        if !self.visible || self.filtered_indices.is_empty() {
            return None;
        }

        let theme = cx.theme();
        let list_items: Vec<gpui::AnyElement> = self
            .filtered_indices
            .iter()
            .enumerate()
            .map(|(ui_index, &cmd_index)| {
                let item = &SLASH_COMMANDS[cmd_index];
                let is_selected = ui_index == self.selected_index;

                let bg = if is_selected {
                    div().bg(theme.primary).opacity(0.15)
                } else {
                    div()
                };

                let desc_color = theme.muted_foreground;

                bg.id(SharedString::from(std::format!("slash-cmd-{cmd_index}")))
                    .w_full()
                    .h(px(ITEM_HEIGHT))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(theme.foreground)
                            .child(item.label.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(desc_color)
                            .child(item.description.to_string()),
                    )
                    .into_any_element()
            })
            .collect();

        let top = cursor_y.unwrap_or(0.) + 4.;

        let panel = div()
            .id("slash-command-panel")
            .absolute()
            .top(px(top))
            .left(px(0.))
            .w(px(PANEL_WIDTH))
            .max_h(px(PANEL_MAX_HEIGHT))
            .bg(theme.background)
            .border(px(1.))
            .border_color(theme.border)
            .rounded(px(8.))
            .shadow(vec![gpui::BoxShadow {
                color: gpui::hsla(0., 0., 0., 0.2),
                offset: gpui::point(px(0.), px(4.)),
                blur_radius: px(12.),
                spread_radius: px(0.),
            }])
            .p(px(4.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .children(list_items)
            .into_any_element();

        Some(panel)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlashCommandEvent {
    CommandSelected,
    Dismissed,
}

impl Render for SlashCommandPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
