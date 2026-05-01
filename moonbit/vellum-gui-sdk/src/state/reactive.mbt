// 响应式更新系统 - 类似 SwiftUI 的视图失效和自动更新机制
// 实现依赖追踪、脏检查和最小化更新

// =============================================================================
// 依赖追踪器
// =============================================================================

// 依赖图 - 记录所有依赖关系
pub type DependencyGraph = {
  // state_id -> 依赖它的 observer_id 列表
  dependents: Map[String, Set[String]]
  // observer_id -> 它依赖的 state_id 列表
  dependencies: Map[String, Set[String]]
}

let global_graph: DependencyGraph = {
  dependents: Map::new(),
  dependencies: Map::new()
}

// 追踪上下文 - 当前正在执行的观察者
struct TrackingContext {
  active_observer: Option[String]
  stack: List[String]
}

let tracking_context: TrackingContext = {
  active_observer: None,
  stack: []
}

// 开始追踪依赖
pub fn begin_track(observer_id: String) -> Unit {
  tracking_context.stack.push(observer_id)
  tracking_context.active_observer = Some(observer_id)
}

// 结束追踪依赖
pub fn end_track() -> Unit {
  tracking_context.stack.pop()
  tracking_context.active_observer = match tracking_context.stack.peek() {
    Some(id) => Some(id),
    None => None
  }
}

// 获取当前观察者
pub fn current_observer() -> Option[String] {
  tracking_context.active_observer
}

// 记录依赖关系
pub fn track(state_id: String) -> Unit {
  match tracking_context.active_observer {
    Some(observer_id) => {
      // 添加 state_id -> observer_id 的依赖
      match global_graph.dependents.get(state_id) {
        Some(deps) => {
          deps.insert(observer_id)
        }
        None => {
          let new_deps = Set::new()
          new_deps.insert(observer_id)
          global_graph.dependents.insert(state_id, new_deps)
        }
      }
      
      // 添加 observer_id -> state_id 的反向依赖
      match global_graph.dependencies.get(observer_id) {
        Some(deps) => {
          deps.insert(state_id)
        }
        None => {
          let new_deps = Set::new()
          new_deps.insert(state_id)
          global_graph.dependencies.insert(observer_id, new_deps)
        }
      }
    }
    None => ()
  }
}

// 获取状态的所有依赖者
pub fn get_dependents(state_id: String) -> Set[String] {
  match global_graph.dependents.get(state_id) {
    Some(deps) => deps,
    None => Set::new()
  }
}

// 获取观察者的所有依赖
pub fn get_dependencies(observer_id: String) -> Set[String] {
  match global_graph.dependencies.get(observer_id) {
    Some(deps) => deps,
    None => Set::new()
  }
}

// 清除观察者的所有依赖
pub fn clear_dependencies(observer_id: String) -> Unit {
  // 从依赖图中移除
  match global_graph.dependencies.get(observer_id) {
    Some(deps) => {
      for state_id in deps {
        match global_graph.dependents.get(state_id) {
          Some(dependents) => {
            dependents.remove(observer_id)
          }
          None => ()
        }
      }
    }
    None => ()
  }
  
  global_graph.dependencies.remove(observer_id)
}

// =============================================================================
// 脏检查系统
// =============================================================================

// 脏状态
pub enum DirtyState {
  Clean
  LayoutDirty
  PaintDirty
  LayoutAndPaintDirty
}

// 脏检查管理器
pub type DirtyChecker = {
  dirty_widgets: Map[String, DirtyState]
  pending_updates: List[() -> Unit>
}

let dirty_checker: DirtyChecker = {
  dirty_widgets: Map::new(),
  pending_updates: []
}

// 标记组件为脏
pub fn mark_dirty(widget_id: String, state: DirtyState) -> Unit {
  match dirty_checker.dirty_widgets.get(widget_id) {
    Some(current) => {
      // 合并脏状态
      let new_state = match (current, state) {
        (Clean, s) => s,
        (LayoutDirty, PaintDirty) => LayoutAndPaintDirty,
        (PaintDirty, LayoutDirty) => LayoutAndPaintDirty,
        (LayoutAndPaintDirty, _) => LayoutAndPaintDirty,
        (_, LayoutAndPaintDirty) => LayoutAndPaintDirty,
        (s, Clean) => s,
        (s1, s2) if s1 == s2 => s1,
        _ => LayoutAndPaintDirty
      }
      dirty_checker.dirty_widgets.insert(widget_id, new_state)
    }
    None => {
      dirty_checker.dirty_widgets.insert(widget_id, state)
    }
  }
}

// 检查组件是否为脏
pub fn is_dirty(widget_id: String) -> Bool {
  match dirty_checker.dirty_widgets.get(widget_id) {
    Some(Clean) => false,
    Some(_) => true,
    None => false
  }
}

// 获取脏状态
pub fn get_dirty_state(widget_id: String) -> DirtyState {
  match dirty_checker.dirty_widgets.get(widget_id) {
    Some(state) => state,
    None => Clean
  }
}

// 清除脏状态
pub fn clear_dirty(widget_id: String) -> Unit {
  dirty_checker.dirty_widgets.remove(widget_id)
}

// 清除所有脏状态
pub fn clear_all_dirty() -> Unit {
  dirty_checker.dirty_widgets.clear()
}

// =============================================================================
// 批量更新系统
// =============================================================================

// 批量更新上下文
struct BatchContext {
  depth: Int
  pending_notifies: Set<String]
}

let batch_context: BatchContext = {
  depth: 0,
  pending_notifies: Set::new()
}

// 开始批量更新
pub fn begin_batch() -> Unit {
  batch_context.depth = batch_context.depth + 1
}

// 结束批量更新
pub fn end_batch() -> Unit {
  batch_context.depth = batch_context.depth - 1
  
  if batch_context.depth == 0 {
    // 执行所有待处理的通知
    let notifies = batch_context.pending_notifies
    batch_context.pending_notifies.clear()
    
    for state_id in notifies {
      notify_dependents(state_id)
    }
  }
}

// 在批量更新中追踪变化
pub fn track_change(state_id: String) -> Unit {
  if batch_context.depth > 0 {
    batch_context.pending_notifies.insert(state_id)
  } else {
    notify_dependents(state_id)
  }
}

// 通知所有依赖者
fn notify_dependents(state_id: String) -> Unit {
  let dependents = get_dependents(state_id)
  for observer_id in dependents {
    trigger_update(observer_id)
  }
}

// 触发更新
fn trigger_update(observer_id: String) -> Unit {
  // 查找观察者并调用其更新函数
  match global_observers.get(observer_id) {
    Some(observer) => {
      observer.update_fn()
    }
    None => ()
  }
}

// 全局观察者注册表
let global_observers: Map[String, ReactiveObserver] = Map::new()

// =============================================================================
// 响应式观察者
// =============================================================================

// 响应式观察者
pub type ReactiveObserver = {
  id: String
  update_fn: () -> Unit
  dispose_fn: Option<() -> Unit>
}

// 创建响应式观察者
pub fn create_observer(update_fn: () -> Unit) -> String {
  let id = generate_observer_id()
  let observer = {
    id,
    update_fn,
    dispose_fn: None
  }
  
  global_observers.insert(id, observer)
  id
}

// 销毁观察者
pub fn destroy_observer(observer_id: String) -> Unit {
  clear_dependencies(observer_id)
  global_observers.remove(observer_id)
}

// =============================================================================
// 自动运行 - 类似 MobX autorun
// =============================================================================

// 自动运行函数，自动追踪依赖并在依赖变化时重新执行
pub fn autorun(fn: () -> Unit) -> String {
  let observer_id = generate_observer_id()
  
  let wrapped_fn = fn() -> Unit {
    begin_track(observer_id)
    fn()
    end_track()
  }
  
  // 首次执行，建立依赖关系
  wrapped_fn()
  
  // 注册观察者
  global_observers.insert(observer_id, {
    id: observer_id,
    update_fn: wrapped_fn,
    dispose_fn: None
  })
  
  observer_id
}

// =============================================================================
// 计算值 - 类似 MobX computed
// =============================================================================

// 计算值
pub type Computed[T] = {
  id: String
  compute_fn: () -> T
  cached_value: Option[T]
  dirty: Bool
  observers: Set<String]
}

// 创建计算值
pub fn computed[T](compute_fn: () -> T) -> Computed[T] {
  let id = generate_computed_id()
  
  let computed = {
    id,
    compute_fn,
    cached_value: None,
    dirty: true,
    observers: Set::new()
  }
  
  // 首次计算，建立依赖
  begin_track(id)
  let value = compute_fn()
  end_track()
  
  computed.cached_value = Some(value)
  computed.dirty = false
  
  computed
}

// 获取计算值
pub fn Computed::get[T](self: Computed[T]) -> T {
  if self.dirty {
    begin_track(self.id)
    let value = self.compute_fn()
    end_track()
    
    self.cached_value = Some(value)
    self.dirty = false
    value
  } else {
    match self.cached_value {
      Some(v) => v,
      None => {
        let value = self.compute_fn()
        self.cached_value = Some(value)
        value
      }
    }
  }
}

// 使计算值失效
pub fn Computed::invalidate[T](self: Computed[T]) -> Unit {
  self.dirty = true
  self.cached_value = None
  
  // 通知依赖此计算值的观察者
  for observer_id in self.observers {
    trigger_update(observer_id)
  }
}

// =============================================================================
// Watch - 类似 Vue watch
// =============================================================================

// Watch 选项
pub type WatchOptions = {
  immediate: Bool
  deep: Bool
}

// Watch 回调
pub type WatchCallback[T] = (T, Option[T]) -> Unit

// 创建 watcher
pub fn watch[T](get_fn: () -> T, callback: WatchCallback[T], options: WatchOptions) -> String {
  let observer_id = generate_observer_id()
  let mut old_value: Option[T] = None
  
  let update_fn = fn() -> Unit {
    let new_value = get_fn()
    
    // 检查值是否变化
    let changed = match old_value {
      Some(old) => old != new_value,
      None => true
    }
    
    if changed {
      callback(new_value, old_value)
      old_value = Some(new_value)
    }
  }
  
  // 如果设置了 immediate，立即执行
  if options.immediate {
    update_fn()
  }
  
  // 注册观察者
  global_observers.insert(observer_id, {
    id: observer_id,
    update_fn,
    dispose_fn: None
  })
  
  observer_id
}

// =============================================================================
// 响应式对象 - 类似 Vue reactive
// =============================================================================

// 响应式对象代理
pub type ReactiveProxy[T] = {
  target: T
  id: String
  observers: Set[String]
}

// 创建响应式对象
pub fn reactive[T](obj: T) -> ReactiveProxy[T] {
  let id = generate_reactive_id()
  
  {
    target: obj,
    id,
    observers: Set::new()
  }
}

// 获取响应式对象的值
pub fn ReactiveProxy::get[T](self: ReactiveProxy[T], field: String) -> T {
  track("\{self.id}.\{field}")
  // 实际实现需要反射或字段访问
  self.target
}

// 设置响应式对象的值
pub fn ReactiveProxy::set[T](self: ReactiveProxy[T], field: String, value: T) -> Unit {
  // 实际实现需要反射或字段访问
  track_change("\{self.id}.\{field}")
}

// =============================================================================
// 辅助函数
// =============================================================================

fn generate_observer_id() -> String {
  let ts = @time.now_ms()
  let rand = @random.int(1000000)
  "observer_\{ts}_\{rand}"
}

fn generate_computed_id() -> String {
  let ts = @time.now_ms()
  let rand = @random.int(1000000)
  "computed_\{ts}_\{rand}"
}

fn generate_reactive_id() -> String {
  let ts = @time.now_ms()
  let rand = @random.int(1000000)
  "reactive_\{ts}_\{rand}"
}

// =============================================================================
// 调试工具
// =============================================================================

// 打印依赖图
pub fn debug_print_graph() -> Unit {
  @debug.log("=== Dependency Graph ===")
  
  @debug.log("Dependents:")
  for (state_id, deps) in global_graph.dependents {
    @debug.log("  \{state_id} -> \{deps}")
  }
  
  @debug.log("Dependencies:")
  for (observer_id, deps) in global_graph.dependencies {
    @debug.log("  \{observer_id} -> \{deps}")
  }
}

// 打印脏检查状态
pub fn debug_print_dirty() -> Unit {
  @debug.log("=== Dirty Widgets ===")
  for (widget_id, state) in dirty_checker.dirty_widgets {
    @debug.log("  \{widget_id}: \{state}")
  }
}
