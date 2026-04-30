// Vellum SDK - 应用基类

use widget.*

// 全局应用状态
struct AppState {
  root: WidgetNode,
  needs_rebuild: Bool,
}

let mut app_state: AppState? = None

pub trait App {
  fn init()
  fn build_ui() -> WidgetNode
  fn handle_event(event_type: String, data: String) -> Bool
}

pub fn run_app<T: App>() {
  // 初始化应用
  T.init()

  // 初始构建 UI
  let initial_root = T.build_ui()
  app_state = Some({ root: initial_root, needs_rebuild: false })
}

pub fn mark_needs_rebuild() {
  app_state = match app_state {
    Some(s) => Some({ s with needs_rebuild: true }),
    None => None
  }
}

pub fn set_root(widget: WidgetNode) {
  app_state = Some({ root: widget, needs_rebuild: false })
}
