// Vellum SDK - Widget 基类和核心类型

pub struct Color {
  r: U8,
  g: U8,
  b: U8,
  a: U8,
}

pub struct Size {
  width: F32,
  height: F32,
}

pub struct Insets {
  top: F32,
  right: F32,
  bottom: F32,
  left: F32,
}

pub struct TextStyle {
  font_size: F32,
  color: Option<Color>,
}

// Widget 节点
pub enum WidgetNode {
  Container(id: U64, props: ContainerProps, children: List<WidgetNode>),
  Text(id: U64, props: TextProps),
  Button(id: U64, props: ButtonProps, children: List<WidgetNode>),
  Column(id: U64, props: FlexProps, children: List<WidgetNode>),
  Row(id: U64, props: FlexProps, children: List<WidgetNode>),
  SizedBox(id: U64, props: SizedBoxProps, children: List<WidgetNode>),
  Padding(id: U64, insets: Insets, child: WidgetNode),
}

pub struct ContainerProps {
  width: Option<F32>,
  height: Option<F32>,
  color: Option<Color>,
  padding: Option<Insets>,
}

pub struct TextProps {
  content: String,
  style: Option<TextStyle>,
}

pub struct ButtonProps {
  on_click: Option<U64>,
  enabled: Bool,
}

pub struct FlexProps {
  spacing: Option<F32>,
}

pub struct SizedBoxProps {
  width: Option<F32>,
  height: Option<F32>,
}

// Widget builder
pub trait Widget {
  fn build(self): WidgetNode
}

// ID 生成器
let mut next_id: U64 = 0

pub fn new_widget_id(): U64 {
  next_id = next_id + 1
  next_id - 1
}

// Container widget
pub struct Container {
  width: Option<F32>,
  height: Option<F32>,
  color: Option<Color>,
  padding: Option<Insets>,
  children: List<WidgetNode>,
}

impl Container {
  pub fn new() -> Container {
    {
      width: None,
      height: None,
      color: None,
      padding: None,
      children: []
    }
  }

  pub fn width(mut self, w: F32) -> Container {
    self.width = Some(w);
    self
  }

  pub fn height(mut self, h: F32) -> Container {
    self.height = Some(h);
    self
  }

  pub fn color(mut self, color: Color) -> Container {
    self.color = Some(color);
    self
  }

  pub fn padding(mut self, insets: Insets) -> Container {
    self.padding = Some(insets);
    self
  }

  pub fn child(mut self, child: WidgetNode) -> Container {
    self.children = Cons(child, self.children);
    self
  }
}

impl Widget for Container {
  fn build(self) -> WidgetNode {
    WidgetNode::Container(
      new_widget_id(),
      {
        width: self.width,
        height: self.height,
        color: self.color,
        padding: self.padding
      },
      List.reverse(self.children)
    )
  }
}

// Text widget
pub struct Text {
  content: String,
  style: Option<TextStyle>,
}

impl Text {
  pub fn new(content: String) -> Text {
    { content, style: None }
  }

  pub fn style(mut self, style: TextStyle) -> Text {
    self.style = Some(style);
    self
  }

  pub fn font_size(mut self, size: F32) -> Text {
    self.style = match self.style {
      Some(s) => Some({ s with font_size: size }),
      None => Some({ font_size: size, color: None })
    };
    self
  }

  pub fn color(mut self, color: Color) -> Text {
    self.style = match self.style {
      Some(s) => Some({ s with color: Some(color) }),
      None => Some({ font_size: 16.0, color: Some(color) })
    };
    self
  }
}

impl Widget for Text {
  fn build(self) -> WidgetNode {
    WidgetNode::Text(new_widget_id(), {
      content: self.content,
      style: self.style
    })
  }
}

// Column widget
pub struct Column {
  spacing: Option<F32>,
  children: List<WidgetNode>,
}

impl Column {
  pub fn new() -> Column {
    { spacing: None, children: [] }
  }

  pub fn spacing(mut self, s: F32) -> Column {
    self.spacing = Some(s);
    self
  }

  pub fn child(mut self, child: WidgetNode) -> Column {
    self.children = Cons(child, self.children);
    self
  }
}

impl Widget for Column {
  fn build(self) -> WidgetNode {
    WidgetNode::Column(
      new_widget_id(),
      { spacing: self.spacing },
      List.reverse(self.children)
    )
  }
}

// Row widget
pub struct Row {
  spacing: Option<F32>,
  children: List<WidgetNode>,
}

impl Row {
  pub fn new() -> Row {
    { spacing: None, children: [] }
  }

  pub fn spacing(mut self, s: F32) -> Row {
    self.spacing = Some(s);
    self
  }

  pub fn child(mut self, child: WidgetNode) -> Row {
    self.children = Cons(child, self.children);
    self
  }
}

impl Widget for Row {
  fn build(self) -> WidgetNode {
    WidgetNode::Row(
      new_widget_id(),
      { spacing: self.spacing },
      List.reverse(self.children)
    )
  }
}

// SizedBox widget
pub struct SizedBox {
  width: Option<F32>,
  height: Option<F32>,
  child: Option<WidgetNode>,
}

impl SizedBox {
  pub fn new() -> SizedBox {
    { width: None, height: None, child: None }
  }

  pub fn width(mut self, w: F32) -> SizedBox {
    self.width = Some(w);
    self
  }

  pub fn height(mut self, h: F32) -> SizedBox {
    self.height = Some(h);
    self
  }

  pub fn child(mut self, child: WidgetNode) -> SizedBox {
    self.child = Some(child);
    self
  }
}

impl Widget for SizedBox {
  fn build(self) -> WidgetNode {
    let children = match self.child {
      Some(c) => [c],
      None => []
    };
    WidgetNode::SizedBox(
      new_widget_id(),
      { width: self.width, height: self.height },
      children
    )
  }
}

// Padding widget
pub struct PaddingWidget {
  insets: Insets,
  child: Option<WidgetNode>,
}

impl PaddingWidget {
  pub fn new(insets: Insets) -> PaddingWidget {
    { insets, child: None }
  }

  pub fn all(value: F32) -> PaddingWidget {
    PaddingWidget::new({ top: value, right: value, bottom: value, left: value })
  }

  pub fn child(mut self, child: WidgetNode) -> PaddingWidget {
    self.child = Some(child);
    self
  }
}

impl Widget for PaddingWidget {
  fn build(self) -> WidgetNode {
    match self.child {
      Some(c) => WidgetNode::Padding(new_widget_id(), self.insets, c),
      None => panic!("Padding requires a child")
    }
  }
}
