// Main app

use @vellum/sdk.{ Widget, Container, Text, Column, Row, SizedBox, PaddingWidget, App, run_app, mark_needs_rebuild, white, black, blue, rgb, Insets, TextStyle }
use counter.*

// 我们的应用结构
struct MyApp {
}

impl MyApp {
  // 创建 UI
  fn build_widget(): Widget {
    // 根容器
    Container::new()
      .color(rgb(26, 26, 26)) // Dark background
      .child(
        PaddingWidget::all(20.0)
          .child(
            Column::new()
              .spacing(20.0)
              .child(
                SizedBox::new()
                  .height(20.0)
              )
              .child(
                Text::new("Vellum Framework")
                  .font_size(32.0)
                  .color(white())
              )
              .child(
                Text::new("Hello World!")
                  .font_size(24.0)
                  .color(rgb(100, 149, 237)) // Cornflower blue
              )
              .child(
                SizedBox::new()
                  .height(40.0)
              )
              .child(
                Text::new("Counter: " + Int.to_string(get_count()))
                  .font_size(28.0)
                  .color(white())
              )
              .child(
                Row::new()
                  .spacing(20.0)
                  .child(
                    Text::new("-")
                      .font_size(40.0)
                      .color(blue())
                  )
                  .child(
                    Text::new("+")
                      .font_size(40.0)
                      .color(blue())
                  )
              )
          )
      )
  }
}

impl App for MyApp {
  fn init() {
    // 初始化
  }

  fn build_ui(): Widget {
    MyApp::build_widget()
  }

  fn handle_event(event_type: String, data: String): Bool {
    false // No events handled yet
  }
}

// 入口点
pub fn main() {
  run_app<MyApp>()
}
