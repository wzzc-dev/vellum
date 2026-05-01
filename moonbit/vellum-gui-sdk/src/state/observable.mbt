// Observable 机制 - 状态观察和依赖追踪系统
// 参考 SwiftUI @Observable 和 MobX 的观察者模式

// 观察者回调类型
pub type ObserverCallback[T] = (T) -> Unit

// 观察者信息
pub type ObserverId = String

pub struct Observer[T] {
  id: ObserverId
  callback: ObserverCallback[T]
  owner: Option[String]
}

// 依赖追踪器 - 追踪当前正在访问的观察者
struct DependencyTracker {
  current_observer: Option[ObserverId]
  dependencies: Map[ObserverId, Set[String]]
}

let tracker: DependencyTracker = {
  current_observer: None,
  dependencies: Map::new()
}

// 创建唯一的观察者ID
fn generate_observer_id() -> ObserverId {
  let timestamp = @time.now_ms()
  let random = @random.int(1000000)
  "observer_\{timestamp}_\{random}"
}

// 追踪依赖 - 记录当前观察者依赖的字段
pub fn track_dependency(field_id: String) -> Unit {
  match tracker.current_observer {
    Some(observer_id) => {
      match tracker.dependencies.get(observer_id) {
        Some(deps) => {
          deps.insert(field_id)
        }
        None => {
          let new_deps = Set::new()
          new_deps.insert(field_id)
          tracker.dependencies.insert(observer_id, new_deps)
        }
      }
    }
    None => ()
  }
}

// 开始观察 - 设置当前观察者
pub fn start_observing[T](observer_id: ObserverId) -> Unit {
  tracker.current_observer = Some(observer_id)
}

// 结束观察 - 清除当前观察者
pub fn stop_observing() -> Unit {
  tracker.current_observer = None
}

// 获取当前观察的依赖
pub fn get_dependencies(observer_id: ObserverId) -> Set[String] {
  match tracker.dependencies.get(observer_id) {
    Some(deps) => deps
    None => Set::new()
  }
}

// 清空观察者的依赖
pub fn clear_dependencies(observer_id: ObserverId) -> Unit {
  tracker.dependencies.remove(observer_id)
}

// 通知依赖的观察者
pub fn notify_observers(changed_field: String, observers: Map[ObserverId, Observer[_]]) -> Unit {
  let to_notify = []
  
  for (observer_id, _) in observers {
    let deps = get_dependencies(observer_id)
    if deps.contains(changed_field) {
      to_notify.push(observer_id)
    }
  }
  
  for observer_id in to_notify {
    match observers.get(observer_id) {
      Some(observer) => {
        // 注意：实际回调需要根据类型调用
        // 这里只是标记需要通知
      }
      None => ()
    }
  }
}

// 观察者管理器
pub type Observable[T] = {
  value: T
  observers: Map[ObserverId, Observer[T]]
  field_paths: Map[String, Any]  // 用于追踪字段访问
}

pub fn Observable::new[T](initial_value: T) -> Observable[T] {
  {
    value: initial_value,
    observers: Map::new(),
    field_paths: Map::new()
  }
}

// 创建观察者
pub fn Observable::observe[T](self: Observable[T], callback: ObserverCallback[T], ?owner: String) -> ObserverId {
  let id = generate_observer_id()
  let observer = {
    id,
    callback,
    owner: Some(owner)
  }
  self.observers.insert(id, observer)
  id
}

// 移除观察者
pub fn Observable::unobserve[T](self: Observable[T], observer_id: ObserverId) -> Bool {
  self.observers.remove(observer_id)
  clear_dependencies(observer_id)
}

// 通知所有观察者
pub fn Observable::notify[T](self: Observable[T]) -> Unit {
  let value_copy = self.value
  for (_, observer) in self.observers {
    observer.callback(value_copy)
  }
}

// 批量更新 - 延迟通知直到更新完成
pub type BatchUpdate = {
  pending_fields: Set[String]
  pending_notifications: Set[ObserverId]
}

let batch_state: BatchUpdate = {
  pending_fields: Set::new(),
  pending_notifications: Set::new()
}

// 开始批量更新
pub fn begin_batch_update() -> Unit {
  batch_state.pending_fields.clear()
  batch_state.pending_notifications.clear()
}

// 结束批量更新并发送通知
pub fn end_batch_update() -> Unit {
  // 清空所有待处理的字段和通知
  batch_state.pending_fields.clear()
  batch_state.pending_notifications.clear()
}

// 追踪批量更新中的变化
pub fn track_batch_change(field_id: String) -> Unit {
  batch_state.pending_fields.insert(field_id)
}

// 检查是否在批量更新中
pub fn is_in_batch_update() -> Bool {
  batch_state.pending_fields.length() > 0
}

// 计算属性 - 自动追踪依赖
pub type Computed[T] = {
  compute_fn: () -> T
  cached_value: Option[T]
  dependencies: Set[String]
  dirty: Bool
}

pub fn Computed::new[T](compute_fn: () -> T) -> Computed[T] {
  {
    compute_fn,
    cached_value: None,
    dependencies: Set::new(),
    dirty: true
  }
}

// 获取计算属性值（带依赖追踪）
pub fn Computed::get[T](self: Computed[T]) -> T {
  if self.dirty {
    start_observing("computed")
    let value = self.compute_fn()
    stop_observing()
    
    self.dependencies = get_dependencies("computed")
    self.cached_value = Some(value)
    self.dirty = false
    value
  } else {
    match self.cached_value {
      Some(v) => v
      None => {
        let value = self.compute_fn()
        self.cached_value = Some(value)
        value
      }
    }
  }
}

// 标记计算属性为脏
pub fn Computed::invalidate[T](self: Computed[T]) -> Unit {
  self.dirty = true
  self.cached_value = None
}

// 检查计算属性是否依赖某个字段
pub fn Computed::depends_on[T](self: Computed[T], field_id: String) -> Bool {
  self.dependencies.contains(field_id)
}

// 派生状态 - 类似 SwiftUI @State 的派生状态
pub type DerivedState[T] = {
  source: Observable[_]
  transform: () -> T
  cached_value: T
  observers: Map[ObserverId, Observer[T]]
}

pub fn DerivedState::new[Source, T](source: Observable[Source], transform: () -> T) -> DerivedState[T] {
  let initial_value = transform()
  {
    source,
    transform,
    cached_value: initial_value,
    observers: Map::new()
  }
}

// 更新派生状态
pub fn DerivedState::update[T](self: DerivedState[T]) -> Unit {
  let new_value = self.transform()
  if new_value != self.cached_value {
    self.cached_value = new_value
    // 通知所有观察者
    for (_, observer) in self.observers {
      observer.callback(new_value)
    }
  }
}

// 自动清理机制
pub type AutoCleanup = {
  disposables: List[() -> Unit]
}

let global_cleanup: AutoCleanup = {
  disposables: []
}

// 注册清理函数
pub fn on_dispose(callback: () -> Unit) -> Unit {
  global_cleanup.disposables.push(callback)
}

// 执行所有清理
pub fn dispose_all() -> Unit {
  for callback in global_cleanup.disposables {
    callback()
  }
  global_cleanup.disposables.clear()
}

// Reaction - 类似 MobX 的 reaction
pub type Reaction = {
  id: ObserverId
  expression: () -> Bool
  effect: () -> Unit
  fire_immediately: Bool
}

let reactions: Map[ObserverId, Reaction] = Map::new()

pub fn reaction[T](expression: () -> Bool, effect: () -> Unit, ?fire_immediately: Bool) -> ObserverId {
  let id = generate_observer_id()
  let reaction_obj = {
    id,
    expression,
    effect,
    fire_immediately: fire_immediately
  }
  
  reactions.insert(id, reaction_obj)
  
  if fire_immediately {
    if expression() {
      effect()
    }
  }
  
  id
}

pub fn run_reaction(observer_id: ObserverId) -> Unit {
  match reactions.get(observer_id) {
    Some(reaction) => {
      if reaction.expression() {
        reaction.effect()
      }
    }
    None => ()
  }
}

pub fn dispose_reaction(observer_id: ObserverId) -> Bool {
  reactions.remove(observer_id)
}
