// Counter Demo - 基础计数器应用
// 展示状态管理、响应式更新和基础 UI 组件

// 导入状态管理模块
use @gui.state.{State, Binding, autorun, computed}
use @gui.types.{Color, color}
use @gui.container.{Column, Row}
use @gui.text.{Text, Heading}
use @gui.button.{Button}
use @gui.input.{Slider}

// =============================================================================
// 计数器状态
// =============================================================================

// 计数器应用状态
struct CounterState {
  mut count: Int
  mut step: Int
  mut max_value: Int
  mut min_value: Int
}

impl CounterState {
  fn new() -> CounterState {
    {
      mut count: 0,
      mut step: 1,
      mut max_value: 100,
      mut min_value: -100
    }
  }
  
  fn increment(self: CounterState) -> Unit {
    if self.count + self.step <= self.max_value {
      self.count = self.count + self.step
    }
  }
  
  fn decrement(self: CounterState) -> Unit {
    if self.count - self.step >= self.min_value {
      self.count = self.count - self.step
    }
  }
  
  fn reset(self: CounterState) -> Unit {
    self.count = 0
  }
  
  fn set_step(self: CounterState, new_step: Int) -> Unit {
    if new_step > 0 && new_step <= 10 {
      self.step = new_step
    }
  }
}

// =============================================================================
// 计数器视图
// =============================================================================

// 创建计数器视图
fn CounterView(state: CounterState) -> Widget {
  // 创建响应式状态
  let count_state = State::new(state.count)
  let step_state = State::new(state.step)
  
  // 计算派生值
  let doubled_count = computed(fn() {
    count_state.get() * 2
  })
  
  let is_positive = computed(fn() {
    count_state.get() > 0
  })
  
  let is_negative = computed(fn() {
    count_state.get() < 0
  })
  
  // 主视图
  Column::new([
    // 标题
    Heading::new("Counter Demo", 1)
      .text_align(Center)
      .padding(24.0),
    
    // 计数显示
    Column::new([
      Text::new("Current Count")
        .font_size(16.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
      
      // 动态颜色的计数显示
      Text::new("\{count_state.get()}")
        .font_size(48.0)
        .font_weight(Bold)
        .color(
          if is_positive.get() {
            color(0.0, 0.8, 0.0, 1.0)  // 绿色
          } else if is_negative.get() {
            color(0.8, 0.0, 0.0, 1.0)  // 红色
          } else {
            color(0.0, 0.0, 0.0, 1.0)  // 黑色
          }
        ),
      
      // 双倍值显示
      Text::new("Doubled: \{doubled_count.get()}")
        .font_size(20.0)
        .color(color(0.3, 0.3, 0.8, 1.0)),
    ])
    .gap(8.0)
    .padding(32.0),
    
    // 控制按钮
    Row::new([
      Button::new("- \{step_state.get()}")
        .on_click(fn() {
          count_state.update(fn(n) { n - step_state.get() })
        })
        .variant(Secondary)
        .padding(16.0, 32.0),
      
      Button::new("Reset")
        .on_click(fn() {
          count_state.set(0)
        })
        .variant(Ghost)
        .padding(16.0, 32.0),
      
      Button::new("+ \{step_state.get()}")
        .on_click(fn() {
          count_state.update(fn(n) { n + step_state.get() })
        })
        .variant(Primary)
        .padding(16.0, 32.0),
    ])
    .gap(16.0)
    .padding(16.0),
    
    // 步长控制
    Column::new([
      Text::new("Step Size: \{step_state.get()}")
        .font_size(16.0),
      
      Slider::new("step", step_state.get().to_float(), 1.0, 10.0)
        .on_change(fn(value) {
          step_state.set(value.to_int())
        }),
    ])
    .gap(8.0)
    .padding(16.0),
    
    // 快捷按钮
    Row::new([
      Button::new("-10")
        .on_click(fn() { count_state.update(fn(n) { n - 10 }) })
        .variant(Ghost)
        .font_size(14.0),
      
      Button::new("-5")
        .on_click(fn() { count_state.update(fn(n) { n - 5 }) })
        .variant(Ghost)
        .font_size(14.0),
      
      Button::new("+5")
        .on_click(fn() { count_state.update(fn(n) { n + 5 }) })
        .variant(Ghost)
        .font_size(14.0),
      
      Button::new("+10")
        .on_click(fn() { count_state.update(fn(n) { n + 10 }) })
        .variant(Ghost)
        .font_size(14.0),
    ])
    .gap(8.0)
    .padding(8.0),
    
    // 状态指示器
    Row::new([
      Text::new("Min: \{state.min_value}")
        .font_size(12.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
      
      Text::new("Max: \{state.max_value}")
        .font_size(12.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
    ])
    .gap(32.0),
  ])
  .gap(24.0)
  .padding(32.0)
  .align_items(Center)
}

// =============================================================================
// 主函数
// =============================================================================

pub fn main() -> Unit {
  // 创建应用状态
  let state = CounterState::new()
  
  // 创建视图
  let view = CounterView(state)
  
  // 运行应用
  @gui.app.run(view)
}

// =============================================================================
// 测试
// =============================================================================

test("CounterState::increment increases count by step") {
  let state = CounterState::new()
  state.step = 5
  
  state.increment()
  assert_eq!(state.count, 5)
  
  state.increment()
  assert_eq!(state.count, 10)
}

test("CounterState::decrement decreases count by step") {
  let state = CounterState::new()
  state.step = 3
  
  state.decrement()
  assert_eq!(state.count, -3)
}

test("CounterState::reset sets count to 0") {
  let state = CounterState::new()
  state.count = 50
  
  state.reset()
  assert_eq!(state.count, 0)
}

test("CounterState respects max_value") {
  let state = CounterState::new()
  state.max_value = 10
  state.count = 8
  state.step = 5
  
  state.increment()
  assert_eq!(state.count, 8)  // 不应该超过最大值
}

test("CounterState respects min_value") {
  let state = CounterState::new()
  state.min_value = -10
  state.count = -8
  state.step = 5
  
  state.decrement()
  assert_eq!(state.count, -8)  // 不应该低于最小值
}
