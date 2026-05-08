fn no_children() -> FixedArray[UInt] {
  FixedArray::makei(0, fn(_) { 0U })
}

fn node(id : String, kind : @types.ViewKind, children : FixedArray[UInt]) -> @types.ViewNode {
  @types.ViewNode::{ id, kind, children }
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
