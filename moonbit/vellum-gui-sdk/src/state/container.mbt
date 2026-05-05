///|
/// Container types for layout management.
pub enum ContainerType {
  Column
  Row
  Stack
  Flex
}

///|
/// Container represents a layout container widget.
pub struct Container {
  base : Widget
  container_type : ContainerType
  spacing : Double
  padding : Double
}

///|
/// Create a new Column container.
pub fn Container::column(children : @list.List[Widget]) -> Container {
  Container::{
    base: build_column_base(children),
    container_type: Column,
    spacing: 0.0,
    padding: 0.0,
  }
}

///|
/// Create a new Row container.
pub fn Container::row(children : @list.List[Widget]) -> Container {
  Container::{
    base: build_row_base(children),
    container_type: Row,
    spacing: 0.0,
    padding: 0.0,
  }
}

///|
/// Create a new Stack container.
pub fn Container::stack(children : @list.List[Widget]) -> Container {
  Container::{
    base: build_stack_base(children),
    container_type: Stack,
    spacing: 0.0,
    padding: 0.0,
  }
}

///|
/// Create a new Flex container.
pub fn Container::flex(children : @list.List[Widget]) -> Container {
  Container::{
    base: build_flex_base(children),
    container_type: Flex,
    spacing: 0.0,
    padding: 0.0,
  }
}

fn build_column_base(children : @list.List[Widget]) -> Widget {
  let mut base = Widget::new("column", "col-gen-1")
  for child in children {
    base = base.add_child(child)
  }
  base
}

fn build_row_base(children : @list.List[Widget]) -> Widget {
  let mut base = Widget::new("row", "row-gen-1")
  for child in children {
    base = base.add_child(child)
  }
  base
}

fn build_stack_base(children : @list.List[Widget]) -> Widget {
  let mut base = Widget::new("stack", "stack-gen-1")
  for child in children {
    base = base.add_child(child)
  }
  base
}

fn build_flex_base(children : @list.List[Widget]) -> Widget {
  let mut base = Widget::new("flex", "flex-gen-1")
  for child in children {
    base = base.add_child(child)
  }
  base
}

///|
/// Set spacing for the container.
pub fn Container::with_spacing(self : Container, spacing : Double) -> Container {
  { ..self, spacing: spacing }
}

///|
/// Set padding for the container.
pub fn Container::with_padding(self : Container, padding : Double) -> Container {
  { ..self, padding: padding }
}

///|
/// Get the base widget.
pub fn Container::base(self : Container) -> Widget {
  self.base
}

///|
/// Get the container type.
pub fn Container::get_type(self : Container) -> ContainerType {
  self.container_type
}

///|
/// Get the spacing.
pub fn Container::get_spacing(self : Container) -> Double {
  self.spacing
}

///|
/// Get the padding.
pub fn Container::get_padding(self : Container) -> Double {
  self.padding
}
