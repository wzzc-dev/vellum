fn no_children() -> FixedArray[UInt] {
  FixedArray::makei(0, fn(_) { 0U })
}

fn children(indices : Array[UInt]) -> FixedArray[UInt] {
  FixedArray::makei(indices.length(), fn(i) { indices[i] })
}

fn empty_style() -> @types.TextStyle {
  @types.TextStyle::{
    size: None,
    color: None,
    bold: None,
    italic: None,
    monospace: None,
  }
}

fn text_style(
  size~ : Float? = None,
  color~ : String? = None,
  bold~ : Bool? = None,
  italic~ : Bool? = None,
  monospace~ : Bool? = None
) -> @types.TextStyle {
  @types.TextStyle::{
    size,
    color,
    bold,
    italic,
    monospace,
  }
}

fn container(gap~ : Float? = None) -> @types.ContainerProps {
  @types.ContainerProps::{ gap, padding: None }
}

fn node(id : String, kind : @types.ViewKind, children : FixedArray[UInt]) -> @types.ViewNode {
  @types.ViewNode::{ id, kind, children }
}

fn column(id : String, kids : Array[UInt]) -> @types.ViewNode {
  node(id, @types.ViewKind::Column(container(gap=Some(8.0))), children(kids))
}

fn row(id : String, kids : Array[UInt]) -> @types.ViewNode {
  node(id, @types.ViewKind::Row(container(gap=Some(8.0))), children(kids))
}

fn scroll_view(id : String, kids : Array[UInt]) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::ScrollView(@types.ScrollViewProps::{ axis: @types.SplitAxis::VERTICAL }),
    children(kids),
  )
}

fn split_view(id : String, ratio : Float, kids : Array[UInt]) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::SplitView(@types.SplitViewProps::{
      axis: @types.SplitAxis::HORIZONTAL,
      ratio,
    }),
    children(kids),
  )
}

fn text_node(id : String, content : String) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::Text(@types.TextProps::{ content, style: empty_style() }),
    no_children(),
  )
}

fn muted_text(id : String, content : String) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::Text(@types.TextProps::{
      content,
      style: text_style(size=Some(12.0), color=Some("muted-foreground")),
    }),
    no_children(),
  )
}

fn heading(id : String, content : String) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::Text(@types.TextProps::{
      content,
      style: text_style(size=Some(18.0), bold=Some(true)),
    }),
    no_children(),
  )
}

fn button(id : String, label : String, style : @types.ButtonVariant) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::Button(@types.ButtonProps::{ label, style, disabled: false }),
    no_children(),
  )
}

fn input(id : String, placeholder : String, value : String) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::Input(@types.InputProps::{ placeholder, value, single_line: true }),
    no_children(),
  )
}

fn tabs(id : String, selected : UInt, tabs : Array[@types.TabItem]) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::Tabs(@types.TabsProps::{ selected, tabs }),
    no_children(),
  )
}

fn native_markdown_editor(id : String) -> @types.ViewNode {
  node(
    id,
    @types.ViewKind::NativeView(@types.NativeViewProps::{
      kind: "markdown-editor",
      props: [],
    }),
    no_children(),
  )
}
