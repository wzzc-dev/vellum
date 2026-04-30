// Counter component

// 简单的计数状态管理
let mut count: Int = 0

pub fn get_count(): Int {
  count
}

pub fn increment() {
  count = count + 1
}

pub fn decrement() {
  count = count - 1
}
