// Timer Demo - 计时器应用
// 展示定时器状态管理、异步操作和格式化显示

// 导入状态管理模块
use @gui.state.{State, autorun, watch}
use @gui.types.{Color, color}
use @gui.container.{Column, Row}
use @gui.text.{Text, Heading}
use @gui.button.{Button}
use @gui.input.{Select}

// =============================================================================
// 计时器状态
// =============================================================================

// 计时器模式
enum TimerMode {
  Stopwatch  // 秒表模式（向上计数）
  Countdown  // 倒计时模式（向下计数）
}

// 计时器状态
struct TimerState {
  mut mode: TimerMode
  mut elapsed_ms: UInt64      // 已过时间（毫秒）
  mut target_ms: UInt64       // 目标时间（倒计时模式）
  mut is_running: Bool
  mut start_time: Option[UInt64]
  mut laps: List[UInt64]
}

impl TimerState {
  fn new() -> TimerState {
    {
      mut mode: Stopwatch,
      mut elapsed_ms: 0,
      mut target_ms: 60000,  // 默认 1 分钟
      mut is_running: false,
      mut start_time: None,
      mut laps: []
    }
  }
  
  // 开始计时
  fn start(self: TimerState) -> Unit {
    if !self.is_running {
      self.is_running = true
      self.start_time = Some(@time.now_ms())
    }
  }
  
  // 暂停计时
  fn pause(self: TimerState) -> Unit {
    if self.is_running {
      self.is_running = false
      // 保存已计时间
      self.elapsed_ms = self.current_elapsed()
      self.start_time = None
    }
  }
  
  // 重置计时器
  fn reset(self: TimerState) -> Unit {
    self.is_running = false
    self.elapsed_ms = 0
    self.start_time = None
    self.laps = []
  }
  
  // 切换运行状态
  fn toggle(self: TimerState) -> Unit {
    if self.is_running {
      self.pause()
    } else {
      self.start()
    }
  }
  
  // 记录圈数
  fn lap(self: TimerState) -> Unit {
    if self.is_running {
      self.laps.push(self.current_elapsed())
    }
  }
  
  // 设置目标时间（倒计时模式）
  fn set_target(self: TimerState, minutes: Int) -> Unit {
    self.target_ms = (minutes * 60 * 1000).to_uint64()
  }
  
  // 获取当前已过时间
  fn current_elapsed(self: TimerState) -> UInt64 {
    match self.start_time {
      Some(start) => {
        let now = @time.now_ms()
        let delta = now - start
        self.elapsed_ms + delta
      }
      None => self.elapsed_ms
    }
  }
  
  // 获取剩余时间（倒计时模式）
  fn remaining(self: TimerState) -> UInt64 {
    if self.elapsed_ms >= self.target_ms {
      0
    } else {
      self.target_ms - self.elapsed_ms
    }
  }
  
  // 是否超时（倒计时模式）
  fn is_expired(self: TimerState) -> Bool {
    self.mode == Countdown && self.remaining() == 0
  }
}

// =============================================================================
// 时间格式化
// =============================================================================

// 格式化时间为 HH:MM:SS.ms
fn format_time(ms: UInt64) -> String {
  let total_seconds = ms / 1000
  let hours = total_seconds / 3600
  let minutes = (total_seconds % 3600) / 60
  let seconds = total_seconds % 60
  let milliseconds = ms % 1000
  
  if hours > 0 {
    "\{pad(hours)}:\{pad(minutes)}:\{pad(seconds)}"
  } else {
    "\{pad(minutes)}:\{pad(seconds)}.\{pad_ms(milliseconds)}"
  }
}

// 格式化简短时间
fn format_short(ms: UInt64) -> String {
  let total_seconds = ms / 1000
  let minutes = total_seconds / 60
  let seconds = total_seconds % 60
  
  "\{minutes}:\{pad(seconds)}"
}

// 补零
fn pad(n: UInt64) -> String {
  if n < 10 {
    "0\{n}"
  } else {
    "\{n}"
  }
}

// 毫秒补零
fn pad_ms(ms: UInt64) -> String {
  if ms < 10 {
    "00\{ms}"
  } else if ms < 100 {
    "0\{ms}"
  } else {
    "\{ms}"
  }
}

// =============================================================================
// 计时器视图组件
// =============================================================================

// 时间显示组件
fn TimeDisplay(time_ms: UInt64, is_expired: Bool) -> Widget {
  Column::new([
    Text::new(format_time(time_ms))
      .font_size(64.0)
      .font_weight(Bold)
      .font_family("monospace")
      .color(
        if is_expired {
          color(0.8, 0.0, 0.0, 1.0)
        } else {
          color(0.0, 0.0, 0.0, 1.0)
        }
      ),
  ])
  .align_items(Center)
}

// 圈数列表组件
fn LapsList(laps: List[UInt64]) -> Widget {
  if laps.length() == 0 {
    Text::new("")
  } else {
    Column::new([
      Text::new("Laps")
        .font_size(16.0)
        .font_weight(Medium),
      
      Column::new(
        laps.enumerate().map(fn((index, lap_time)) {
          Row::new([
            Text::new("Lap \{index + 1}")
              .font_size(14.0),
            
            Text::new(format_time(lap_time))
              .font_size(14.0)
              .font_family("monospace"),
          ])
          .justify_content(SpaceBetween)
          .padding(8.0)
        })
      )
      .gap(4.0),
    ])
    .gap(12.0)
    .padding(16.0)
    .background(color(0.95, 0.95, 0.95, 1.0))
    .border_radius(8.0)
  }
}

// 控制按钮组件
fn ControlButtons(state: TimerState) -> Widget {
  Row::new([
    // 重置按钮
    Button::new("Reset")
      .on_click(fn() {
        state.reset()
      })
      .variant(Ghost)
      .padding(16.0, 32.0),
    
    // 开始/暂停按钮
    Button::new(if state.is_running { "Pause" } else { "Start" })
      .on_click(fn() {
        state.toggle()
      })
      .variant(if state.is_running { Secondary } else { Primary })
      .padding(16.0, 48.0),
    
    // 圈数按钮（秒表模式）
    if state.mode == Stopwatch {
      Button::new("Lap")
        .on_click(fn() {
          state.lap()
        })
        .variant(Ghost)
        .enabled(state.is_running)
        .padding(16.0, 32.0)
    } else {
      Text::new("")
    },
  ])
  .gap(16.0)
}

// 模式选择组件
fn ModeSelector(current_mode: TimerMode, on_change: (TimerMode) -> Unit) -> Widget {
  Row::new([
    Button::new("Stopwatch")
      .variant(if current_mode == Stopwatch { Primary } else { Ghost })
      .on_click(fn() { on_change(Stopwatch) })
      .padding(12.0, 24.0),
    
    Button::new("Countdown")
      .variant(if current_mode == Countdown { Primary } else { Ghost })
      .on_click(fn() { on_change(Countdown) })
      .padding(12.0, 24.0),
  ])
  .gap(8.0)
}

// 倒计时设置组件
fn CountdownSettings(target_minutes: Int, on_change: (Int) -> Unit) -> Widget {
  Column::new([
    Text::new("Set Duration")
      .font_size(14.0)
      .font_weight(Medium),
    
    Row::new([
      Button::new("1 min")
        .variant(if target_minutes == 1 { Secondary } else { Ghost })
        .on_click(fn() { on_change(1) }),
      
      Button::new("5 min")
        .variant(if target_minutes == 5 { Secondary } else { Ghost })
        .on_click(fn() { on_change(5) }),
      
      Button::new("10 min")
        .variant(if target_minutes == 10 { Secondary } else { Ghost })
        .on_click(fn() { on_change(10) }),
      
      Button::new("25 min")
        .variant(if target_minutes == 25 { Secondary } else { Ghost })
        .on_click(fn() { on_change(25) }),
    ])
    .gap(8.0),
  ])
  .gap(8.0)
}

// =============================================================================
// 主视图
// =============================================================================

fn TimerApp(state: TimerState) -> Widget {
  // 创建响应式状态
  let mode_state = State::new(state.mode)
  let elapsed_state = State::new(state.elapsed_ms)
  let running_state = State::new(state.is_running)
  let target_minutes_state = State::new(1)
  
  // 计算显示时间
  let display_time = computed(fn() {
    if mode_state.get() == Stopwatch {
      elapsed_state.get()
    } else {
      let target_ms = (target_minutes_state.get() * 60 * 1000).to_uint64()
      if elapsed_state.get() >= target_ms {
        0
      } else {
        target_ms - elapsed_state.get()
      }
    }
  })
  
  // 主视图
  Column::new([
    // 标题
    Heading::new("Timer", 1)
      .text_align(Center)
      .padding(24.0),
    
    // 模式选择
    ModeSelector(mode_state.get(), fn(new_mode) {
      mode_state.set(new_mode)
      state.mode = new_mode
      state.reset()
    }),
    
    // 时间显示
    TimeDisplay(display_time.get(), state.is_expired()),
    
    // 倒计时设置（仅倒计时模式）
    if mode_state.get() == Countdown {
      CountdownSettings(target_minutes_state.get(), fn(minutes) {
        target_minutes_state.set(minutes)
        state.set_target(minutes)
      })
    },
    
    // 控制按钮
    ControlButtons(state),
    
    // 圈数列表（仅秒表模式）
    if mode_state.get() == Stopwatch {
      LapsList(state.laps)
    },
    
    // 统计信息
    Row::new([
      Text::new("Mode: \{match mode_state.get() {
        Stopwatch => "Stopwatch",
        Countdown => "Countdown"
      }}")
        .font_size(12.0)
        .color(color(0.5, 0.5, 0.5, 1.0)),
      
      Text::new("Laps: \{state.laps.length()}")
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
  let state = TimerState::new()
  let view = TimerApp(state)
  @gui.app.run(view)
}

// =============================================================================
// 测试
// =============================================================================

test("TimerState::start sets is_running to true") {
  let state = TimerState::new()
  
  state.start()
  
  assert!(state.is_running)
  assert!(state.start_time.is_some())
}

test("TimerState::pause stops the timer") {
  let state = TimerState::new()
  
  state.start()
  state.pause()
  
  assert_eq!(state.is_running, false)
  assert!(state.start_time.is_none())
}

test("TimerState::reset clears all state") {
  let state = TimerState::new()
  
  state.start()
  state.lap()
  state.reset()
  
  assert_eq!(state.is_running, false)
  assert_eq!(state.elapsed_ms, 0)
  assert_eq!(state.laps.length(), 0)
}

test("TimerState::lap records elapsed time") {
  let state = TimerState::new()
  
  state.start()
  state.lap()
  
  assert_eq!(state.laps.length(), 1)
}

test("format_time formats correctly") {
  assert_eq!(format_time(0), "00:00.000")
  assert_eq!(format_time(1000), "00:01.000")
  assert_eq!(format_time(60000), "01:00.000")
  assert_eq!(format_time(3600000), "01:00:00")
}

test("TimerState::remaining calculates correctly") {
  let state = TimerState::new()
  state.mode = Countdown
  state.target_ms = 10000
  state.elapsed_ms = 3000
  
  assert_eq!(state.remaining(), 7000)
}

test("TimerState::is_expired detects expiration") {
  let state = TimerState::new()
  state.mode = Countdown
  state.target_ms = 5000
  state.elapsed_ms = 5000
  
  assert!(state.is_expired())
}
