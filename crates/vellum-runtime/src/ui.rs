use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewTree {
    pub root: u32,
    pub nodes: Vec<ViewNode>,
}

impl ViewTree {
    pub fn root_node(&self) -> Option<&ViewNode> {
        self.nodes.get(self.root as usize)
    }

    pub fn child_nodes<'a>(
        &'a self,
        node: &'a ViewNode,
    ) -> impl Iterator<Item = &'a ViewNode> + 'a {
        node.children
            .iter()
            .filter_map(|index| self.nodes.get(*index as usize))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewNode {
    pub id: String,
    pub kind: ViewKind,
    pub children: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViewKind {
    Empty,
    Column(ContainerProps),
    Row(ContainerProps),
    Text(TextProps),
    Button(ButtonProps),
    Input(InputProps),
    Tabs(TabsProps),
    SplitView(SplitViewProps),
    ScrollView(ScrollViewProps),
    NativeView(NativeViewProps),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContainerProps {
    pub gap: Option<f32>,
    pub padding: Option<EdgeInsets>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextProps {
    pub content: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    pub size: Option<f32>,
    pub color: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub monospace: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ButtonProps {
    pub label: String,
    pub style: ButtonVariant,
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InputProps {
    pub placeholder: String,
    pub value: String,
    pub single_line: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabsProps {
    pub selected: u32,
    pub tabs: Vec<TabItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub child: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SplitViewProps {
    pub axis: SplitAxis,
    pub ratio: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScrollViewProps {
    pub axis: SplitAxis,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeViewProps {
    pub kind: String,
    pub props: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppEvent {
    Ui(UiEvent),
    Native(NativeEvent),
    Command(CommandEvent),
    Tick(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiEvent {
    pub target_id: String,
    pub event_kind: String,
    pub value: Option<String>,
    pub index: Option<u32>,
    pub checked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeEvent {
    pub view_id: String,
    pub event_kind: String,
    pub payload: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandEvent {
    pub command_id: String,
    pub payload: Vec<Property>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorSnapshot {
    pub display_name: String,
    pub path: Option<String>,
    pub dirty: bool,
    pub word_count: u32,
    pub document_text: String,
    pub view_mode: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginState {
    Enabled,
    Disabled,
    Failed,
}

impl Default for PluginState {
    fn default() -> Self {
        Self::Enabled
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginPanel {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub state: PluginState,
    pub commands: Vec<PluginCommand>,
    pub panels: Vec<PluginPanel>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_resolves_root_and_children() {
        let tree = ViewTree {
            root: 0,
            nodes: vec![
                ViewNode {
                    id: "root".into(),
                    kind: ViewKind::Column(ContainerProps::default()),
                    children: vec![1],
                },
                ViewNode {
                    id: "child".into(),
                    kind: ViewKind::Text(TextProps {
                        content: "hello".into(),
                        style: TextStyle::default(),
                    }),
                    children: vec![],
                },
            ],
        };

        let root = tree.root_node().unwrap();
        let children: Vec<_> = tree
            .child_nodes(root)
            .map(|node| node.id.as_str())
            .collect();

        assert_eq!(root.id, "root");
        assert_eq!(children, vec!["child"]);
    }
}
