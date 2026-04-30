// Vellum SDK - 主入口

mod core {
  mod color
  mod widget_id
}

mod widget
mod app

pub use color::{
  Color, rgba, rgb, white, black, red, green, blue, gray
}

pub use widget::{
  WidgetNode, Color, Size, Insets, TextStyle,
  Widget,
  Container, Text, Column, Row, SizedBox, PaddingWidget
}

pub use app::{ App, run_app, mark_needs_rebuild }
