// Binding 机制 - 类似 SwiftUI @Binding 的双向绑定
// Binding 提供对状态的读写访问，支持父子组件间的数据同步

// Binding 类型 - 泛型绑定接口
pub type Binding[T] = {
  get_fn: () -> T
  set_fn: (T) -> Bool
  id: String
}

// 创建 Binding
pub fn Binding::new[T](get_fn: () -> T, set_fn: (T) -> Bool) -> Binding[T] {
  {
    get_fn,
    set_fn,
    id: generate_binding_id()
  }
}

// 从 State 创建 Binding
pub fn Binding::from_state[T](state: State[T]) -> Binding[T] {
  {
    get_fn: fn() { state.value },
    set_fn: fn(v) { state.set(v) },
    id: "binding_\{state.id}"
  }
}

// 从只读值创建常量 Binding
pub fn Binding::constant[T](value: T) -> Binding[T] {
  {
    get_fn: fn() { value },
    set_fn: fn(_) { false },
    id: "constant_\{value}"
  }
}

// 获取 Binding 的值
pub fn Binding::get[T](self: Binding[T]) -> T {
  self.get_fn()
}

// 设置 Binding 的值
pub fn Binding::set[T](self: Binding[T], value: T) -> Bool {
  self.set_fn(value)
}

// 原子更新 - 使用函数更新值
pub fn Binding::update[T](self: Binding[T], updater: (T) -> T) -> Bool {
  let current = self.get()
  let new_value = updater(current)
  self.set(new_value)
}

// =============================================================================
// Binding 转换
// =============================================================================

// 映射 Binding - 转换读取和写入
pub fn Binding::map[T, U](self: Binding[T], get_fn: (T) -> U, set_fn: (U) -> T) -> Binding[U] {
  {
    get_fn: fn() {
      get_fn(self.get())
    },
    set_fn: fn(value) {
      let original = set_fn(value)
      self.set(original)
    },
    id: "mapped_\{self.id}"
  }
}

// 只读映射
pub fn Binding::map_get[T, U](self: Binding[T], transform: (T) -> U) -> Binding[U] {
  {
    get_fn: fn() {
      transform(self.get())
    },
    set_fn: fn(_) { false },
    id: "readonly_\{self.id}"
  }
}

// =============================================================================
// Binding 组合
// =============================================================================

// 组合两个 Binding - 类似 SwiftUI 的 combine
pub fn Binding::combine[T, U, V](self: Binding[T], other: Binding[U], combine_fn: (T, U) -> V, split_fn: (V) -> (T, U)) -> Binding[V] {
  {
    get_fn: fn() {
      combine_fn(self.get(), other.get())
    },
    set_fn: fn(value) {
      let (t, u) = split_fn(value)
      let result1 = self.set(t)
      let result2 = other.set(u)
      result1 || result2
    },
    id: "combined_\{self.id}_\{other.id}"
  }
}

// =============================================================================
// 验证 Binding
// =============================================================================

// 带验证的 Binding
pub fn Binding::validated[T](self: Binding[T], validator: (T) -> Bool) -> Binding[T] {
  {
    get_fn: self.get_fn,
    set_fn: fn(value) {
      if validator(value) {
        self.set(value)
      } else {
        false
      }
    },
    id: "validated_\{self.id}"
  }
}

// =============================================================================
// Binding 运算符
// =============================================================================

// 等于比较
pub fn Binding::equal[T: Eq](self: Binding[T], value: T) -> Binding[Bool] {
  {
    get_fn: fn() {
      self.get() == value
    },
    set_fn: fn(b) {
      if b {
        false // 不能直接设置等同性
      } else {
        false
      }
    },
    id: "equal_\{self.id}"
  }
}

// 条件 Binding - 当条件满足时可写
pub fn Binding::if_true[T](self: Binding[Bool], then_binding: Binding[T]) -> Binding[T] {
  {
    get_fn: fn() {
      if self.get() {
        then_binding.get()
      } else {
        // 返回默认值
        @default[T]()
      }
    },
    set_fn: fn(value) {
      if self.get() {
        then_binding.set(value)
      } else {
        false
      }
    },
    id: "if_true_\{self.id}"
  }
}

// =============================================================================
// Binding 便捷函数
// =============================================================================

// 整数 Binding 快捷函数
pub fn binding_int(get_fn: () -> Int, set_fn: (Int) -> Bool) -> Binding[Int] {
  Binding::new(get_fn, set_fn)
}

// 字符串 Binding 快捷函数
pub fn binding_string(get_fn: () -> String, set_fn: (String) -> Bool) -> Binding[String] {
  Binding::new(get_fn, set_fn)
}

// 布尔 Binding 快捷函数
pub fn binding_bool(get_fn: () -> Bool, set_fn: (Bool) -> Bool) -> Binding[Bool] {
  Binding::new(get_fn, set_fn)
}

// =============================================================================
// 双向绑定帮助器
// =============================================================================

// 创建双向绑定的辅助函数
pub fn two_way_binding[T](state: State[T]) -> Binding[T] {
  Binding::from_state(state)
}

// 创建带有转换的双向绑定
pub fn two_way_binding_with[T, U](state: State[T], transform: (U) -> T, inverse: (T) -> U) -> Binding[U] {
  {
    get_fn: fn() {
      inverse(state.value)
    },
    set_fn: fn(value) {
      let transformed = transform(value)
      state.set(transformed)
    },
    id: "transformed_\{state.id}"
  }
}

// =============================================================================
// Binding 集合操作
// =============================================================================

// List Binding 操作
pub fn list_binding_add[T](list_binding: Binding[List[T]], item: T) -> Bool {
  list_binding.update(fn(list) {
    let mut new_list = list
    new_list.push(item)
    new_list
  })
}

pub fn list_binding_remove[T](list_binding: Binding[List[T]], index: Int) -> Bool {
  list_binding.update(fn(list) {
    let mut new_list = list
    if index >= 0 && index < list.length() {
      new_list.remove(index)
    }
    new_list
  })
}

pub fn list_binding_clear[T](list_binding: Binding[List[T]]) -> Bool {
  list_binding.set(List::[])
}

pub fn list_binding_insert[T](list_binding: Binding[List[T]], index: Int, item: T) -> Bool {
  list_binding.update(fn(list) {
    let mut new_list = list
    new_list.insert(index, item)
    new_list
  })
}

// =============================================================================
// Binding 动画支持
// =============================================================================

// 带动画的 Binding
pub fn Binding::with_animation[T](self: Binding[T], animation: Animation) -> Binding[T] {
  {
    get_fn: self.get_fn,
    set_fn: fn(value) {
      // TODO: 实现动画支持
      self.set(value)
    },
    id: "animated_\{self.id}"
  }
}

// =============================================================================
// Binding 调试支持
// =============================================================================

// Binding 调试信息
pub fn Binding::debug[T](self: Binding[T]) -> Binding[T] {
  {
    get_fn: fn() {
      let value = self.get()
      @debug.log("[\{self.id}] Reading: \{value}")
      value
    },
    set_fn: fn(value) {
      @debug.log("[\{self.id}] Writing: \{value}")
      self.set(value)
    },
    id: "debug_\{self.id}"
  }
}

// =============================================================================
// Binding 宏（模拟 @Binding）
// =============================================================================

// 创建 Binding 的便捷方式
pub fn @binding[T](get_fn: () -> T, set_fn: (T) -> Bool) -> Binding[T] {
  Binding::new(get_fn, set_fn)
}

// =============================================================================
// 示例用法
// =============================================================================

/*
// 基本用法
let count = State::new(0)
let count_binding = Binding::from_state(count)

// 通过 Binding 修改
count_binding.set(10)
count_binding.update(fn(n) { n + 1 })

// 映射
let string_binding = count_binding.map(
  fn(n) { "Count: \{n}" },
  fn(s) {
    // 从 "Count: 10" 提取 10
    let num_str = s.replace("Count: ", "")
    @int.from_string(num_str).unwrap_or(0)
  }
)

// 组合
let first_name = State::new("John")
let last_name = State::new("Doe")
let full_name = Binding::combine(
  Binding::from_state(first_name),
  Binding::from_state(last_name),
  fn(first, last) { "\{first} \{last}" },
  fn(full) {
    let parts = full.split(" ")
    (parts[0], parts[1])
  }
)

// 在组件中使用
fn UserForm(user: Binding[User]) {
  Column::new([
    TextField("First Name", Binding::map(user, fn(u) { u.first_name }, fn(u, fn) { {..u, first_name: fn} }))
    TextField("Last Name", Binding::map(user, fn(u) { u.last_name }, fn(u, ln) { {..u, last_name: ln} }))
  ])
}
*/
