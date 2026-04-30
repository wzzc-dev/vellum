use gpui::Entity;
use gpui_component::input::InputState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteCommand {
    Bold,
    Italic,
    InlineCode,
    Strikethrough,
    Link,
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Paragraph,
    Blockquote,
    BulletList,
    OrderedList,
    HorizontalRule,
    CodeFence,
    Table,
    SourceMode,
    ToggleSidebar,
    ToggleStatusBar,
    ToggleFocusMode,
    FindPanel,
    FindReplace,
    Undo,
    Redo,
    ThemeDefault,
    ThemeDracula,
    ThemeSolarized,
    ThemeGitHub,
    MathBlock,
}

pub(crate) struct CommandItem {
    pub label: &'static str,
    pub keywords: &'static [&'static str],
    pub description: &'static str,
    pub command: PaletteCommand,
}

pub(crate) const ALL_COMMANDS: &[CommandItem] = &[
    CommandItem {
        label: "Bold",
        keywords: &["bold", "加粗"],
        description: "Toggle bold text",
        command: PaletteCommand::Bold,
    },
    CommandItem {
        label: "Italic",
        keywords: &["italic", "斜体"],
        description: "Toggle italic text",
        command: PaletteCommand::Italic,
    },
    CommandItem {
        label: "Inline Code",
        keywords: &["code", "inline", "行内代码"],
        description: "Toggle inline code",
        command: PaletteCommand::InlineCode,
    },
    CommandItem {
        label: "Strikethrough",
        keywords: &["strikethrough", "delete", "删除线"],
        description: "Toggle strikethrough",
        command: PaletteCommand::Strikethrough,
    },
    CommandItem {
        label: "Link",
        keywords: &["link", "url", "链接"],
        description: "Insert link",
        command: PaletteCommand::Link,
    },
    CommandItem {
        label: "Heading 1",
        keywords: &["h1", "heading1", "标题1"],
        description: "Toggle heading level 1",
        command: PaletteCommand::Heading1,
    },
    CommandItem {
        label: "Heading 2",
        keywords: &["h2", "heading2", "标题2"],
        description: "Toggle heading level 2",
        command: PaletteCommand::Heading2,
    },
    CommandItem {
        label: "Heading 3",
        keywords: &["h3", "heading3", "标题3"],
        description: "Toggle heading level 3",
        command: PaletteCommand::Heading3,
    },
    CommandItem {
        label: "Heading 4",
        keywords: &["h4", "heading4", "标题4"],
        description: "Toggle heading level 4",
        command: PaletteCommand::Heading4,
    },
    CommandItem {
        label: "Heading 5",
        keywords: &["h5", "heading5", "标题5"],
        description: "Toggle heading level 5",
        command: PaletteCommand::Heading5,
    },
    CommandItem {
        label: "Heading 6",
        keywords: &["h6", "heading6", "标题6"],
        description: "Toggle heading level 6",
        command: PaletteCommand::Heading6,
    },
    CommandItem {
        label: "Paragraph",
        keywords: &["paragraph", "p", "段落"],
        description: "Convert to paragraph",
        command: PaletteCommand::Paragraph,
    },
    CommandItem {
        label: "Blockquote",
        keywords: &["quote", "blockquote", "引用"],
        description: "Toggle blockquote",
        command: PaletteCommand::Blockquote,
    },
    CommandItem {
        label: "Bullet List",
        keywords: &["bullet", "list", "ul", "无序列表"],
        description: "Toggle bullet list",
        command: PaletteCommand::BulletList,
    },
    CommandItem {
        label: "Ordered List",
        keywords: &["number", "ordered", "ol", "有序列表"],
        description: "Toggle ordered list",
        command: PaletteCommand::OrderedList,
    },
    CommandItem {
        label: "Insert Horizontal Rule",
        keywords: &["hr", "divider", "分隔线"],
        description: "Insert horizontal rule",
        command: PaletteCommand::HorizontalRule,
    },
    CommandItem {
        label: "Insert Code Block",
        keywords: &["code", "fence", "代码块"],
        description: "Insert code fence",
        command: PaletteCommand::CodeFence,
    },
    CommandItem {
        label: "Insert Table",
        keywords: &["table", "表格"],
        description: "Insert table",
        command: PaletteCommand::Table,
    },
    CommandItem {
        label: "Toggle Source Mode",
        keywords: &["source", "markdown", "源码"],
        description: "Switch between live preview and source mode",
        command: PaletteCommand::SourceMode,
    },
    CommandItem {
        label: "Toggle Sidebar",
        keywords: &["sidebar", "file tree", "侧边栏"],
        description: "Toggle sidebar visibility",
        command: PaletteCommand::ToggleSidebar,
    },
    CommandItem {
        label: "Toggle Status Bar",
        keywords: &["status", "状态栏"],
        description: "Toggle status bar visibility",
        command: PaletteCommand::ToggleStatusBar,
    },
    CommandItem {
        label: "Toggle Focus Mode",
        keywords: &["focus", "专注"],
        description: "Toggle focus mode",
        command: PaletteCommand::ToggleFocusMode,
    },
    CommandItem {
        label: "Find",
        keywords: &["find", "search", "查找"],
        description: "Open find panel",
        command: PaletteCommand::FindPanel,
    },
    CommandItem {
        label: "Find and Replace",
        keywords: &["replace", "查找替换"],
        description: "Open find and replace panel",
        command: PaletteCommand::FindReplace,
    },
    CommandItem {
        label: "Undo",
        keywords: &["undo", "撤销"],
        description: "Undo last edit",
        command: PaletteCommand::Undo,
    },
    CommandItem {
        label: "Redo",
        keywords: &["redo", "重做"],
        description: "Redo last edit",
        command: PaletteCommand::Redo,
    },
    CommandItem {
        label: "Theme: Default",
        keywords: &["theme", "default", "主题", "默认"],
        description: "Switch to default syntax theme",
        command: PaletteCommand::ThemeDefault,
    },
    CommandItem {
        label: "Theme: Dracula",
        keywords: &["theme", "dracula", "主题"],
        description: "Switch to Dracula syntax theme",
        command: PaletteCommand::ThemeDracula,
    },
    CommandItem {
        label: "Theme: Solarized",
        keywords: &["theme", "solarized", "主题"],
        description: "Switch to Solarized syntax theme",
        command: PaletteCommand::ThemeSolarized,
    },
    CommandItem {
        label: "Theme: GitHub",
        keywords: &["theme", "github", "主题"],
        description: "Switch to GitHub syntax theme",
        command: PaletteCommand::ThemeGitHub,
    },
    CommandItem {
        label: "Insert Math Formula",
        keywords: &["math", "formula", "katex", "latex", "公式", "数学"],
        description: "Insert inline math formula $$...$$",
        command: PaletteCommand::MathBlock,
    },
];

pub(crate) fn filter_commands(query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..ALL_COMMANDS.len()).collect();
    }
    let q = query.to_lowercase();
    ALL_COMMANDS
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.label.to_lowercase().contains(&q)
                || item.keywords.iter().any(|k| k.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}

pub(crate) struct CommandPaletteState {
    pub visible: bool,
    pub input: Entity<InputState>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
}

impl CommandPaletteState {
    pub fn new(input: Entity<InputState>) -> Self {
        Self {
            visible: false,
            input,
            filtered_indices: filter_commands(""),
            selected_index: 0,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.filtered_indices = filter_commands("");
        self.selected_index = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn update_filter(&mut self, query: &str) {
        self.filtered_indices = filter_commands(query);
        self.selected_index = 0;
    }

    pub fn select_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
    }

    pub fn select_prev(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.selected_index = if self.selected_index == 0 {
            self.filtered_indices.len() - 1
        } else {
            self.selected_index - 1
        };
    }

    pub fn selected_command(&self) -> Option<PaletteCommand> {
        self.filtered_indices
            .get(self.selected_index)
            .map(|&i| ALL_COMMANDS[i].command)
    }
}
