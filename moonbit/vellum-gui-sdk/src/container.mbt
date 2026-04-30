// Container widgets for Vellum GUI Framework

pub type Container = {
  widget: @types.Widget,
  gap: Float,
  cross_gap: Float,
  main_gap: Float,
}

// Column container - vertical layout
pub type Column = {
  pub @types.Widget
  gap: Float,
  cross_alignment: @types.CrossAlignment,
}

pub fn Column::new(children: List[@types.Widget]) -> Column {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "column",
      children,
      layout: {
        display: Flex,
        flex_direction: Column,
        ..@types.layout_default()
      },
      props: @types.props_default(),
    },
    gap: 8.0,
    cross_alignment: Stretch,
  }
}

pub fn Column::gap(self: Column, gap: Float) -> Column {
  { gap, ..self }
}

pub fn Column::cross_align(self: Column, alignment: @types.CrossAlignment) -> Column {
  { cross_alignment: alignment, ..self }
}

pub fn Column::padding(self: Column, padding: Float) -> Column {
  // Modify widget padding
  self
}

pub fn Column::child(self: Column, child: @types.Widget) -> Column {
  {
    widget: {
      ..self.widget with
      children: @types.append(self.widget.children, child)
    },
    ..self
  }
}

pub fn Column::children(self: Column, children: List[@types.Widget]) -> Column {
  {
    widget: {
      ..self.widget with
      children: @types.concat(self.widget.children, children)
    },
    ..self
  }
}

// Row container - horizontal layout
pub type Row = {
  pub @types.Widget
  gap: Float,
  main_alignment: @types.Alignment,
  cross_alignment: @types.CrossAlignment,
}

pub fn Row::new(children: List[@types.Widget]) -> Row {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "row",
      children,
      layout: {
        display: Flex,
        flex_direction: Row,
        ..@types.layout_default()
      },
      props: @types.props_default(),
    },
    gap: 8.0,
    main_alignment: Start,
    cross_alignment: Stretch,
  }
}

pub fn Row::gap(self: Row, gap: Float) -> Row {
  { gap, ..self }
}

pub fn Row::align(self: Row, alignment: @types.Alignment) -> Row {
  { main_alignment: alignment, ..self }
}

pub fn Row::cross_align(self: Row, alignment: @types.CrossAlignment) -> Row {
  { cross_alignment: alignment, ..self }
}

pub fn Row::child(self: Row, child: @types.Widget) -> Row {
  {
    widget: {
      ..self.widget with
      children: @types.append(self.widget.children, child)
    },
    ..self
  }
}

pub fn Row::children(self: Row, children: List[@types.Widget]) -> Row {
  {
    widget: {
      ..self.widget with
      children: @types.concat(self.widget.children, children)
    },
    ..self
  }
}

// Stack container - overlapping layout
pub type Stack = {
  pub @types.Widget
  alignment: @types.Alignment,
}

pub fn Stack::new(children: List[@types.Widget]) -> Stack {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "stack",
      children,
      layout: {
        display: Flex,
        ..@types.layout_default()
      },
      props: @types.props_default(),
    },
    alignment: Center,
  }
}

pub fn Stack::align(self: Stack, alignment: @types.Alignment) -> Stack {
  { alignment, ..self }
}

pub fn Stack::child(self: Stack, child: @types.Widget) -> Stack {
  {
    widget: {
      ..self.widget with
      children: @types.append(self.widget.children, child)
    },
    ..self
  }
}

// Spacer widget
pub type Spacer = @types.Widget

pub fn Spacer::new() -> Spacer {
  {
    id: @types.generate_id(),
    widget_type: "spacer",
    children: [],
    layout: {
      display: Flex,
      flex_grow: 1.0,
      flex_shrink: 1.0,
      ..@types.layout_default()
    },
    props: @types.props_default(),
  }
}

pub fn Spacer::fixed(size: Float) -> Spacer {
  {
    id: @types.generate_id(),
    widget_type: "spacer",
    children: [],
    layout: @types.layout_default(),
    props: {
      @types.props_default() with
      size: { width: size, height: size }
    },
  }
}

pub fn Spacer::width(w: Float) -> Spacer {
  {
    id: @types.generate_id(),
    widget_type: "spacer",
    children: [],
    layout: @types.layout_default(),
    props: {
      @types.props_default() with
      size: { width: w, height: 0.0 }
    },
  }
}

pub fn Spacer::height(h: Float) -> Spacer {
  {
    id: @types.generate_id(),
    widget_type: "spacer",
    children: [],
    layout: @types.layout_default(),
    props: {
      @types.props_default() with
      size: { width: 0.0, height: h }
    },
  }
}

// Expanded widget
pub type Expanded = @types.Widget

pub fn Expanded::new(child: @types.Widget) -> Expanded {
  {
    child with
    layout: {
      child.layout with
      flex_grow: 1.0,
      flex_shrink: 1.0,
    }
  }
}

pub fn Expanded::flex_grow(self: Expanded, grow: Float) -> Expanded {
  {
    self with
    layout: {
      self.layout with
      flex_grow: grow
    }
  }
}

// Divider widget
pub type Divider = @types.Widget

pub fn Divider::new() -> Divider {
  {
    id: @types.generate_id(),
    widget_type: "divider",
    children: [],
    layout: {
      display: Block,
      ..@types.layout_default()
    },
    props: {
      @types.props_default() with
      height: 1.0,
      background: @types.gray,
    },
  }
}

pub fn Divider::horizontal() -> Divider {
  {
    id: @types.generate_id(),
    widget_type: "divider",
    children: [],
    layout: {
      display: Block,
      ..@types.layout_default()
    },
    props: {
      @types.props_default() with
      height: 1.0,
      background: @types.light_gray,
    },
  }
}

pub fn Divider::vertical() -> Divider {
  {
    id: @types.generate_id(),
    widget_type: "divider",
    children: [],
    layout: {
      display: Flex,
      flex_direction: Row,
      ..@types.layout_default()
    },
    props: {
      @types.props_default() with
      width: 1.0,
      background: @types.light_gray,
    },
  }
}

// Wrap container
pub type Wrap = {
  pub @types.Widget
  spacing: Float,
  run_spacing: Float,
}

pub fn Wrap::new(children: List[@types.Widget]) -> Wrap {
  {
    widget: {
      id: @types.generate_id(),
      widget_type: "wrap",
      children,
      layout: {
        display: Flex,
        flex_wrap: Wrap,
        ..@types.layout_default()
      },
      props: @types.props_default(),
    },
    spacing: 8.0,
    run_spacing: 8.0,
  }
}

pub fn Wrap::spacing(self: Wrap, spacing: Float) -> Wrap {
  { spacing, ..self }
}

pub fn Wrap::run_spacing(self: Wrap, spacing: Float) -> Wrap {
  { run_spacing, ..self }
}
