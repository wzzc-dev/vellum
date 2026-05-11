pub fn fixed_children(children : Array[UInt]) -> FixedArray[UInt] {
  FixedArray::makei(children.length(), fn(i) { children[i] })
}

pub fn node(
  id : String,
  kind : @types.ViewKind,
  children : Array[UInt]
) -> @types.ViewNode {
  @types.ViewNode::{ id, kind, children: fixed_children(children) }
}

pub fn add_node(
  nodes : Array[@types.ViewNode],
  id : String,
  kind : @types.ViewKind,
  children : Array[UInt]
) -> UInt {
  let index = nodes.length().reinterpret_as_uint()
  nodes.push(node(id, kind, children))
  index
}

pub fn tree(root : UInt, nodes : Array[@types.ViewNode]) -> @types.ViewTree {
  @types.ViewTree::{ root, nodes }
}

pub fn property(name : String, value : String) -> @types.Property {
  @types.Property::{ name, value }
}

pub fn edge_insets(top : Float, right : Float, bottom : Float, left : Float) -> @types.EdgeInsets {
  @types.EdgeInsets::{ top, right, bottom, left }
}

pub fn all_insets(value : Float) -> @types.EdgeInsets {
  edge_insets(value, value, value, value)
}

pub fn text_style(
  size : Float?,
  color : String?,
  bold : Bool?,
  italic : Bool?,
  monospace : Bool?
) -> @types.TextStyle {
  @types.TextStyle::{ size, color, bold, italic, monospace }
}

pub fn default_text_style() -> @types.TextStyle {
  text_style(None, None, None, None, None)
}

pub fn muted_text_style() -> @types.TextStyle {
  text_style(None, Some("muted-foreground"), None, None, None)
}

pub fn heading_style() -> @types.TextStyle {
  text_style(Some(16.0), None, Some(true), None, None)
}

pub fn container_props(gap : Float?, padding : @types.EdgeInsets?) -> @types.ContainerProps {
  @types.ContainerProps::{ gap, padding }
}

pub fn text(content : String) -> @types.ViewKind {
  styled_text(content, default_text_style())
}

pub fn styled_text(content : String, style : @types.TextStyle) -> @types.ViewKind {
  @types.ViewKind::Text(@types.TextProps::{ content, style })
}

pub fn muted_text(content : String) -> @types.ViewKind {
  styled_text(content, muted_text_style())
}

pub fn button(label : String) -> @types.ViewKind {
  button_with_variant(label, @types.ButtonVariant::PRIMARY, false)
}

pub fn button_with_variant(
  label : String,
  style : @types.ButtonVariant,
  disabled : Bool
) -> @types.ViewKind {
  @types.ViewKind::Button(@types.ButtonProps::{ label, style, disabled })
}

pub fn input(placeholder : String, value : String, single_line : Bool) -> @types.ViewKind {
  @types.ViewKind::Input(@types.InputProps::{ placeholder, value, single_line })
}

pub fn column(gap : Float?, padding : @types.EdgeInsets?) -> @types.ViewKind {
  @types.ViewKind::Column(container_props(gap, padding))
}

pub fn row(gap : Float?, padding : @types.EdgeInsets?) -> @types.ViewKind {
  @types.ViewKind::Row(container_props(gap, padding))
}

pub fn tab(id : String, label : String, child : UInt) -> @types.TabItem {
  @types.TabItem::{ id, label, child }
}

pub fn tabs(selected : UInt, tabs : Array[@types.TabItem]) -> @types.ViewKind {
  @types.ViewKind::Tabs(@types.TabsProps::{ selected, tabs })
}

pub fn split_view(axis : @types.SplitAxis, ratio : Float) -> @types.ViewKind {
  @types.ViewKind::SplitView(@types.SplitViewProps::{ axis, ratio })
}

pub fn scroll_view(axis : @types.SplitAxis) -> @types.ViewKind {
  @types.ViewKind::ScrollView(@types.ScrollViewProps::{ axis })
}

pub fn native_view(kind : String, props : Array[@types.Property]) -> @types.ViewKind {
  @types.ViewKind::NativeView(@types.NativeViewProps::{ kind, props })
}

pub fn markdown_editor() -> @types.ViewKind {
  native_view("markdown-editor", [])
}

pub fn plugin_panel(panel_id : String) -> @types.ViewKind {
  native_view("plugin-panel", [property("panel-id", panel_id)])
}

pub fn button_clicked(event : @types.UiEvent, target_id : String) -> Bool {
  event.target_id == target_id && event.event_kind == "button.clicked"
}

pub fn input_changed(event : @types.UiEvent, target_id : String) -> Bool {
  event.target_id == target_id && event.event_kind == "input.changed"
}

pub fn first_plugin_panel_id(plugins : Array[@types.PluginInfo]) -> String? {
  for index = 0; index < plugins.length(); index = index + 1 {
    let plugin = plugins[index]
    if plugin.panels.length() > 0 {
      return Some(plugin.panels[0].id)
    }
  }
  None
}

pub fn plugin_summary(plugin : @types.PluginInfo) -> String {
  plugin.name + " " + plugin.version
}
