// Gesture Demo - 手势识别演示
// 展示各种手势识别、组合手势和自定义手势

// 导入状态管理模块
use @gui.state.{State, autorun}
use @gui.types.{Color, color, Point}
use @gui.container.{Column, Row}
use @gui.text.{Text, Heading}
use @gui.gesture.{Gesture, GestureRecognizer, on_tap, on_long_press, on_pan, on_pinch, on_rotate}

// =============================================================================
// 手势状态
// =============================================================================

// 手势事件记录
struct GestureEvent {
  gesture_type: String
  timestamp: UInt64
  details: String
}

// 手势演示状态
struct GestureState {
  mut tap_count: Int
  mut long_press_count: Int
  mut pan_offset: Point
  mut pinch_scale: Float
  mut rotation_angle: Float
  mut swipe_count: Int
  mut events: List[GestureEvent]
  mut last_gesture: Option<String]
}

impl GestureState {
  fn new() -> GestureState {
    {
      mut tap_count: 0,
      mut long_press_count: 0,
      mut pan_offset: { x: 0.0, y: 0.0 },
      mut pinch_scale: 1.0,
      mut rotation_angle: 0.0,
      mut swipe_count: 0,
      mut events: [],
      mut last_gesture: None
    }
  }
  
  fn record_event(self: GestureState, gesture_type: String, details: String) -> Unit {
    let event = {
      gesture_type,
      timestamp: @time.now_ms(),
      details
    }
    self.events.push(event)
    if self.events.length() > 10 {
      self.events.remove(0)
    }
    self.last_gesture = Some(gesture_type)
  }
  
  fn reset(self: GestureState) -> Unit {
    self.tap_count = 0
    self.long_press_count = 0
    self.pan_offset = { x: 0.0, y: 0.0 }
    self.pinch_scale = 1.0
    self.rotation_angle = 0.0
    self.swipe_count = 0
    self.events = []
    self.last_gesture = None
  }
}

// =============================================================================
// 手势演示区域组件
// =============================================================================

// 可交互的手势区域
fn GestureArea(
  label: String,
  gesture: Gesture,
  on_trigger: () -> Unit,
  color: Color
) -> Widget {
  Column::new([
    Text::new(label)
      .font_size(14.0)
      .font_weight(Medium),
    
    // 手势区域
    Container::new()
      .width(150.0)
      .height(150.0)
      .background(color)
      .border_radius(16.0)
      .on_gesture(gesture, on_trigger)
      .align_items(Center)
      .justify_content(Center)
      .child(
        Text::new("Tap me")
          .color(color(1.0, 1.0, 1.0, 1.0))
          .font_size(16.0)
      ),
  ])
  .gap(8.0)
  .align_items(Center)
}

// 可拖拽的组件
fn DraggableBox(state: GestureState) -> Widget {
  Container::new()
    .width(100.0)
    .height(100.0)
    .background(color(0.2, 0.6, 1.0, 1.0))
    .border_radius(12.0)
    .offset(state.pan_offset.x, state.pan_offset.y)
    .scale(state.pinch_scale)
    .rotation(state.rotation_angle)
    .on_pan(fn(delta) {
      state.pan_offset = {
        x: state.pan_offset.x + delta.x,
        y: state.pan_offset.y + delta.y
      }
      state.record_event("Pan", "Delta: (\{delta.x}, \{delta.y})")
    })
    .on_pinch(fn(scale) {
      state.pinch_scale = state.pinch_scale * scale
      state.record_event("Pinch", "Scale: \{scale}")
    })
    .on_rotate(fn(angle) {
      state.rotation_angle = state.rotation_angle + angle
      state.record_event("Rotate", "Angle: \{angle}")
    })
    .child(
      Text::new("Drag me")
        .color(color(1.0, 1.0, 1.0, 1.0))
    )
}

// 事件历史列表
fn EventHistory(events: List[GestureEvent]) -> Widget {
  Column::new([
    Text::new("Recent Events")
      .font_size(14.0)
      .font_weight(Medium),
    
    Column::new(
      events.map(fn(event) {
        Row::new([
          Text::new(event.gesture_type)
            .font_size(12.0)
            .font_weight(Medium)
            .width(80.0),
          
          Text::new(event.details)
            .font_size(12.0)
            .color(color(0.5, 0.5, 0.5, 1.0)),
        ])
        .gap(8.0)
      })
    )
    .gap(4.0),
  ])
  .gap(8.0)
  .padding(16.0)
  .background(color(0.95, 0.95, 0.95, 1.0))
  .border_radius(8.0)
}

// =============================================================================
// 主视图
// =============================================================================

fn GestureApp(state: GestureState) -> Widget {
  // 创建响应式状态
  let tap_state = State::new(state.tap_count)
  let long_press_state = State::new(state.long_press_count)
  let swipe_state = State::new(state.swipe_count)
  
  // 主视图
  Column::new([
    // 标题
    Heading::new("Gesture Demo", 1)
      .text_align(Center)
      .padding(24.0),
    
    // 基础手势区域
    Row::new([
      // 点击
      GestureArea(
        "Tap",
        Gesture::Tap,
        fn() {
          tap_state.update(fn(n) { n + 1 })
          state.record_event("Tap", "Single tap")
        },
        color(0.2, 0.8, 0.4, 1.0)
      ),
      
      // 长按
      GestureArea(
        "Long Press",
        Gesture::LongPress(500),
        fn() {
          long_press_state.update(fn(n) { n + 1 })
          state.record_event("LongPress", "Held for 500ms")
        },
        color(0.8, 0.4, 0.2, 1.0)
      ),
      
      // 双击
      GestureArea(
        "Double Tap",
        Gesture::DoubleTap,
        fn() {
          state.record_event("DoubleTap", "Two quick taps")
        },
        color(0.6, 0.2, 0.8, 1.0)
      ),
    ])
    .gap(24.0)
    .padding(16.0),
    
    // 可拖拽/缩放/旋转的区域
    Column::new([
      Text::new("Interactive Box")
        .font_size(16.0)
        .font_weight(Medium),
      
      Text::new("Drag, pinch, or rotate")
        .font_size(12.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
      
      Container::new()
        .width(300.0)
        .height(200.0)
        .background(color(0.9, 0.9, 0.9, 1.0))
        .border_radius(16.0)
        .align_items(Center)
        .justify_content(Center)
        .child(DraggableBox(state)),
    ])
    .gap(8.0)
    .padding(16.0),
    
    // 滑动手势
    Row::new([
      Column::new([
        Text::new("Swipe Left")
          .font_size(14.0),
        
        Container::new()
          .width(100.0)
          .height(60.0)
          .background(color(0.9, 0.6, 0.6, 1.0))
          .border_radius(8.0)
          .on_swipe(Left, fn() {
            swipe_state.update(fn(n) { n + 1 })
            state.record_event("Swipe", "Left")
          })
          .align_items(Center)
          .child(Text::new("←")),
      ])
      .gap(4.0),
      
      Column::new([
        Text::new("Swipe Right")
          .font_size(14.0),
        
        Container::new()
          .width(100.0)
          .height(60.0)
          .background(color(0.6, 0.9, 0.6, 1.0))
          .border_radius(8.0)
          .on_swipe(Right, fn() {
            swipe_state.update(fn(n) { n + 1 })
            state.record_event("Swipe", "Right")
          })
          .align_items(Center)
          .child(Text::new("→")),
      ])
      .gap(4.0),
    ])
    .gap(24.0)
    .padding(16.0),
    
    // 统计信息
    Row::new([
      Text::new("Taps: \{tap_state.get()}")
        .font_size(14.0),
      
      Text::new("Long Presses: \{long_press_state.get()}")
        .font_size(14.0),
      
      Text::new("Swipes: \{swipe_state.get()}")
        .font_size(14.0),
    ])
    .gap(24.0),
    
    // 事件历史
    EventHistory(state.events),
    
    // 重置按钮
    Button::new("Reset All")
      .on_click(fn() {
        state.reset()
        tap_state.set(0)
        long_press_state.set(0)
        swipe_state.set(0)
      })
      .variant(Ghost),
  ])
  .gap(24.0)
  .padding(32.0)
  .scrollable()
}

// =============================================================================
// 主函数
// =============================================================================

pub fn main() -> Unit {
  let state = GestureState::new()
  let view = GestureApp(state)
  @gui.app.run(view)
}

// =============================================================================
// 测试
// =============================================================================

test("GestureState::record_event adds event to list") {
  let state = GestureState::new()
  
  state.record_event("Tap", "Single tap")
  
  assert_eq!(state.events.length(), 1)
  assert_eq!(state.last_gesture, Some("Tap"))
}

test("GestureState::record_event limits to 10 events") {
  let state = GestureState::new()
  
  for i in 0..15 {
    state.record_event("Tap", "Tap \{i}")
  }
  
  assert_eq!(state.events.length(), 10)
}

test("GestureState::reset clears all state") {
  let state = GestureState::new()
  
  state.tap_count = 5
  state.long_press_count = 3
  state.pan_offset = { x: 10.0, y: 20.0 }
  state.record_event("Tap", "Test")
  
  state.reset()
  
  assert_eq!(state.tap_count, 0)
  assert_eq!(state.long_press_count, 0)
  assert_eq!(state.pan_offset.x, 0.0)
  assert_eq!(state.events.length(), 0)
  assert_eq!(state.last_gesture, None)
}
