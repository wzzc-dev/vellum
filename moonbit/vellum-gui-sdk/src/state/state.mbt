// State 包装器 - 类似 SwiftUI @State 的状态管理
// 提供响应式状态变量，自动追踪依赖并触发 UI 更新

// State 包装类型
pub type State[T] = {
  mut value: T
  id: String
  observers: Map[ObserverId, ObserverCallback[T]]
  equals: (T, T) -> Bool
  validate: Option[(T) -> Bool]
}

// 创建 State
pub fn State::new[T](initial_value: T) -> State[T] {
  {
    mut value: initial_value,
    id: generate_state_id(),
    observers: Map::new(),
    equals: basic_equals,
    validate: None
  }
}

// 创建带验证的 State
pub fn State::validated[T](initial_value: T, validator: (T) -> Bool) -> State[T] {
  {
    mut value: initial_value,
    id: generate_state_id(),
    observers: Map::new(),
    equals: basic_equals,
    validate: Some(validator)
  }
}

// 创建带自定义比较函数的 State
pub fn State::with_equals[T](initial_value: T, equals_fn: (T, T) -> Bool) -> State[T] {
  {
    mut value: initial_value,
    id: generate_state_id(),
    observers: Map::new(),
    equals: equals_fn,
    validate: None
  }
}

// 生成唯一 State ID
fn generate_state_id() -> String {
  let timestamp = @time.now_ms()
  let random = @random.int(1000000)
  "state_\{timestamp}_\{random}"
}

// 基本相等比较
fn basic_equals[T](a: T, b: T) -> Bool {
  a == b
}

// 获取 State 的值（自动追踪依赖）
pub fn State::get[T](self: State[T]) -> T {
  // TODO: 自动追踪当前观察者依赖此 State
  // track_dependency(self.id)
  self.value
}

// 设置 State 的值（验证并通知观察者）
pub fn State::set[T](self: State[T], new_value: T) -> Bool {
  // 验证新值
  match self.validate {
    Some(validator) => {
      if not validator(new_value) {
        return false
      }
    }
    None => ()
  }
  
  // 比较是否真的改变了
  if self.equals(self.value, new_value) {
    return false
  }
  
  // 更新值
  self.value = new_value
  
  // 通知所有观察者
  notify_state_observers(self)
  
  true
}

// 更新 State 的值（使用更新函数）
pub fn State::update[T](self: State[T], updater: (T) -> T) -> Bool {
  let new_value = updater(self.value)
  self.set(new_value)
}

// 重置 State 到初始值
pub fn State::reset[T](self: State[T], initial_value: T) -> Unit {
  self.value = initial_value
  notify_state_observers(self)
}

// 通知所有观察者
fn notify_state_observers[T](state: State[T]) -> Unit {
  let value_copy = state.value
  for (_, callback) in state.observers {
    callback(value_copy)
  }
}

// 观察 State 变化
pub fn State::observe[T](self: State[T], callback: ObserverCallback[T]) -> ObserverId {
  let id = generate_observer_id()
  self.observers.insert(id, callback)
  id
}

// 取消观察
pub fn State::unobserve[T](self: State[T], observer_id: ObserverId) -> Bool {
  self.observers.remove(observer_id)
}

// 重置 State（不清除观察者）
pub fn State::reset[T](self: State[T]) -> Unit {
  // 子类可以重写此方法
  notify_state_observers(self)
}

// State 的链式调用支持
pub fn State::modify[T](self: State[T], modifier: (T) -> T) -> State[T] {
  self.update(modifier)
  self
}

// 检查 State 是否有效
pub fn State::is_valid[T](self: State[T]) -> Bool {
  match self.validate {
    Some(validator) => validator(self.value)
    None => true
  }
}

// 获取 State 的 ID
pub fn State::id[T](self: State[T]) -> String {
  self.id
}

// 合并两个 State
pub fn State::combine[T, U, V](state1: State[T], state2: State[U], combine_fn: (T, U) -> V) -> State[V] {
  let combined = State::new(combine_fn(state1.value, state2.value))
  
  // 当任一 State 变化时，更新合并的 State
  state1.observe(fn(v1) {
    let v2 = state2.value
    combined.set(combine_fn(v1, v2))
  })
  
  state2.observe(fn(v2) {
    let v1 = state1.value
    combined.set(combine_fn(v1, v2))
  })
  
  combined
}

// =============================================================================
// State 便捷构造函数
// =============================================================================

// 整数 State
pub fn state_int(initial: Int) -> State[Int] {
  State::new(initial)
}

// 字符串 State
pub fn state_string(initial: String) -> State[String] {
  State::new(initial)
}

// 布尔 State
pub fn state_bool(initial: Bool) -> State[Bool] {
  State::new(initial)
}

// 浮点数 State
pub fn state_float(initial: Float) -> State[Float] {
  State::new(initial)
}

// 列表 State
pub fn state_list[T](initial: List[T]) -> State[List[T]] {
  State::new(initial)
}

// Map State
pub fn state_map[K, V](initial: Map[K, V]) -> State[Map[K, V]] {
  State::new(initial)
}

// =============================================================================
// @state 属性宏的模拟
// =============================================================================

// 使用 State 的示例模式
// 这模拟了 @state 宏的行为
pub fn create_state[T](initial_value: T) -> State[T] {
  State::new(initial_value)
}

// 在视图中使用 State 的帮助函数
pub fn use_state[T](initial_value: T) -> State[T] {
  State::new(initial_value)
}

// =============================================================================
// 状态持久化支持
// =============================================================================

// State 变更历史记录
pub type StateHistory[T] = {
  state: State[T]
  history: List[T]
  max_size: Int
}

pub fn StateHistory::new[T](initial: T, max_size: Int) -> StateHistory[T] {
  {
    state: State::new(initial),
    history: [initial],
    max_size
  }
}

// 撤销到上一个状态
pub fn StateHistory::undo[T](self: StateHistory[T]) -> Bool {
  if self.history.length() > 1 {
    let _ = self.history.pop()
    match self.history.peek() {
      Some(value) => {
        self.state.set(value)
        true
      }
      None => false
    }
  } else {
    false
  }
}

// 获取历史长度
pub fn StateHistory::length[T](self: StateHistory[T]) -> Int {
  self.history.length()
}

// =============================================================================
// 状态派生
// =============================================================================

// 派生的只读状态
pub type DerivedState[T] = {
  compute_fn: () -> T
  cached_value: T
  dirty: Bool
}

pub fn DerivedState::new[T](compute_fn: () -> T) -> DerivedState[T] {
  {
    compute_fn,
    cached_value: compute_fn(),
    dirty: false
  }
}

pub fn DerivedState::get[T](self: DerivedState[T]) -> T {
  if self.dirty {
    self.cached_value = self.compute_fn()
    self.dirty = false
  }
  self.cached_value
}

pub fn DerivedState::invalidate[T](self: DerivedState[T]) -> Unit {
  self.dirty = true
}

// =============================================================================
// 状态转换
// =============================================================================

// 将 State 转换为 Binding
pub fn State::to_binding[T](self: State[T]) -> Binding[T] {
  Binding::from_state(self)
}

// =============================================================================
// 状态助记符
// =============================================================================

// 数字 State 快捷方法
pub fn State::increment[T: Numeric](self: State[T]) -> Unit {
  self.update(fn(v) { v + 1 })
}

pub fn State::decrement[T: Numeric](self: State[T]) -> Unit {
  self.update(fn(v) { v - 1 })
}

pub fn State::toggle[T: Numeric + Eq](self: State[T]) -> Unit {
  self.update(fn(v) { if v == 0 { 1 } else { 0 } })
}

// 列表 State 快捷方法
pub fn State::add_item[T](self: State[List[T]], item: T) -> Unit {
  self.update(fn(list) { list.push(item); list })
}

pub fn State::remove_item[T](self: State[List[T]], item: T) -> Unit {
  self.update(fn(list) { list.filter(fn(i) { i != item }) })
}

pub fn State::clear_list[T](self: State[List[T]]) -> Unit {
  self.set(List::[])
}

// 字符串 State 快捷方法
pub fn State::append[T: String](self: State[T], text: T) -> Unit {
  self.update(fn(s) { s + text })
}

pub fn State::clear_string[T: String](self: State[T]) -> Unit {
  self.set("")
}
