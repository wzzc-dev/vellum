// 状态管理系统单元测试

// =============================================================================
// Observable 测试
// =============================================================================

test("Observable::new creates observable with initial value") {
  let obs = Observable::new(42)
  assert_eq!(obs.value, 42)
}

test("Observable::observe registers observer") {
  let obs = Observable::new(0)
  let mut called = false
  
  let id = obs.observe(fn(v) {
    called = true
  })
  
  assert!(id.length() > 0)
  assert_eq!(called, false)
}

test("Observable::notify calls all observers") {
  let obs = Observable::new(10)
  let mut sum = 0
  
  obs.observe(fn(v) {
    sum = sum + v
  })
  
  obs.observe(fn(v) {
    sum = sum + v * 2
  })
  
  obs.notify()
  
  assert_eq!(sum, 30) // 10 + 20
}

test("Observable::unobserve removes observer") {
  let obs = Observable::new(5)
  let mut count = 0
  
  let id = obs.observe(fn(v) {
    count = count + 1
  })
  
  obs.notify()
  assert_eq!(count, 1)
  
  obs.unobserve(id)
  obs.notify()
  assert_eq!(count, 1) // 不应该再增加
}

// =============================================================================
// State 测试
// =============================================================================

test("State::new creates state with initial value") {
  let state = State::new(100)
  assert_eq!(state.get(), 100)
}

test("State::set updates value and returns true") {
  let state = State::new(0)
  let result = state.set(42)
  
  assert!(result)
  assert_eq!(state.get(), 42)
}

test("State::set returns false if value unchanged") {
  let state = State::new(10)
  let result = state.set(10)
  
  assert_eq!(result, false)
}

test("State::update applies function to value") {
  let state = State::new(5)
  state.update(fn(n) { n * 2 })
  
  assert_eq!(state.get(), 10)
}

test("State::observe is called on value change") {
  let state = State::new(0)
  let mut new_value = 0
  
  state.observe(fn(v) {
    new_value = v
  })
  
  state.set(100)
  assert_eq!(new_value, 100)
}

test("State::validated rejects invalid values") {
  let state = State::validated(0, fn(n) { n >= 0 })
  
  assert!(state.set(10))    // 有效
  assert_eq!(state.set(-5), false)  // 无效
  assert_eq!(state.get(), 10)       // 值未改变
}

// =============================================================================
// Binding 测试
// =============================================================================

test("Binding::new creates binding") {
  let mut value = 10
  let binding = Binding::new(
    fn() { value },
    fn(v) { value = v; true }
  )
  
  assert_eq!(binding.get(), 10)
}

test("Binding::set updates source") {
  let mut value = 0
  let binding = Binding::new(
    fn() { value },
    fn(v) { value = v; true }
  )
  
  binding.set(42)
  assert_eq!(value, 42)
}

test("Binding::from_state creates binding from state") {
  let state = State::new("hello")
  let binding = Binding::from_state(state)
  
  assert_eq!(binding.get(), "hello")
  
  binding.set("world")
  assert_eq!(state.get(), "world")
}

test("Binding::constant creates read-only binding") {
  let binding = Binding::constant(42)
  
  assert_eq!(binding.get(), 42)
  assert_eq!(binding.set(100), false)  // 不能修改
  assert_eq!(binding.get(), 42)        // 值未变
}

test("Binding::map transforms values") {
  let state = State::new(10)
  let binding = Binding::from_state(state)
  
  let doubled = binding.map(
    fn(n) { n * 2 },
    fn(n) { n / 2 }
  )
  
  assert_eq!(doubled.get(), 20)
  
  doubled.set(40)
  assert_eq!(state.get(), 20)
}

test("Binding::combine combines two bindings") {
  let a = State::new(10)
  let b = State::new(20)
  
  let combined = Binding::combine(
    Binding::from_state(a),
    Binding::from_state(b),
    fn(x, y) { x + y },
    fn(sum) { (sum / 2, sum / 2) }
  )
  
  assert_eq!(combined.get(), 30)
}

// =============================================================================
// 依赖追踪测试
// =============================================================================

test("track records dependency") {
  let state_id = "test_state"
  let observer_id = "test_observer"
  
  begin_track(observer_id)
  track(state_id)
  end_track()
  
  let deps = get_dependencies(observer_id)
  assert!(deps.contains(state_id))
  
  clear_dependencies(observer_id)
}

test("get_dependents returns correct observers") {
  let state_id = "state_1"
  let observer1 = "obs_1"
  let observer2 = "obs_2"
  
  begin_track(observer1)
  track(state_id)
  end_track()
  
  begin_track(observer2)
  track(state_id)
  end_track()
  
  let dependents = get_dependents(state_id)
  assert!(dependents.contains(observer1))
  assert!(dependents.contains(observer2))
  
  clear_dependencies(observer1)
  clear_dependencies(observer2)
}

// =============================================================================
// 脏检查测试
// =============================================================================

test("mark_dirty sets dirty state") {
  let widget_id = "widget_1"
  
  mark_dirty(widget_id, LayoutDirty)
  
  assert!(is_dirty(widget_id))
  assert_eq!(get_dirty_state(widget_id), LayoutDirty)
  
  clear_dirty(widget_id)
}

test("dirty states are merged correctly") {
  let widget_id = "widget_2"
  
  mark_dirty(widget_id, LayoutDirty)
  mark_dirty(widget_id, PaintDirty)
  
  assert_eq!(get_dirty_state(widget_id), LayoutAndPaintDirty)
  
  clear_dirty(widget_id)
}

test("clear_all_dirty removes all dirty states") {
  mark_dirty("w1", LayoutDirty)
  mark_dirty("w2", PaintDirty)
  
  clear_all_dirty()
  
  assert_eq!(is_dirty("w1"), false)
  assert_eq!(is_dirty("w2"), false)
}

// =============================================================================
// 批量更新测试
// =============================================================================

test("batch update defers notifications") {
  let state = State::new(0)
  let mut notify_count = 0
  
  state.observe(fn(_) {
    notify_count = notify_count + 1
  })
  
  begin_batch()
  state.set(1)
  state.set(2)
  state.set(3)
  end_batch()
  
  // 应该只通知一次
  assert_eq!(notify_count, 1)
}

// =============================================================================
// Computed 测试
// =============================================================================

test("computed caches result") {
  let state = State::new(10)
  let mut compute_count = 0
  
  let computed_value = computed(fn() {
    compute_count = compute_count + 1
    state.get() * 2
  })
  
  assert_eq!(computed_value.get(), 20)
  assert_eq!(computed_value.get(), 20)
  assert_eq!(compute_count, 1)  // 只计算一次
}

// =============================================================================
// autorun 测试
// =============================================================================

test("autorun runs on dependency change") {
  let state = State::new(0)
  let mut run_count = 0
  
  let _id = autorun(fn() {
    let _ = state.get()
    run_count = run_count + 1
  })
  
  assert_eq!(run_count, 1)  // 初始运行
  
  state.set(10)
  assert_eq!(run_count, 2)  // 依赖变化后运行
}

// =============================================================================
// watch 测试
// =============================================================================

test("watch calls callback on change") {
  let state = State::new(0)
  let mut last_new = 0
  let mut last_old = -1
  
  let _id = watch(
    fn() { state.get() },
    fn(new_val, old_val) {
      last_new = new_val
      last_old = match old_val {
        Some(v) => v,
        None => -1
      }
    },
    { immediate: false, deep: false }
  )
  
  state.set(10)
  
  assert_eq!(last_new, 10)
  assert_eq!(last_old, 0)
}

// =============================================================================
// 集成测试
// =============================================================================

test("full reactive flow works correctly") {
  // 创建状态
  let count = State::new(0)
  let doubled = computed(fn() { count.get() * 2 })
  
  // 观察
  let mut doubled_value = 0
  let _id = autorun(fn() {
    doubled_value = doubled.get()
  })
  
  assert_eq!(doubled_value, 0)
  
  // 更新状态
  count.set(5)
  
  // 验证响应式更新
  assert_eq!(doubled_value, 10)
}

test("complex dependency graph updates correctly") {
  let a = State::new(1)
  let b = State::new(2)
  
  let sum = computed(fn() { a.get() + b.get() })
  let product = computed(fn() { a.get() * b.get() })
  
  let mut results = []
  
  let _id = autorun(fn() {
    results.push((sum.get(), product.get()))
  })
  
  a.set(3)
  b.set(4)
  
  assert_eq!(results.length(), 3)  // 初始 + 2次更新
}
