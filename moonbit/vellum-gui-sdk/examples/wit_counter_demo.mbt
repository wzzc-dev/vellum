// WIT Counter Demo - 使用 WIT Component Model 的 Counter 示例
//
// 本示例展示了如何使用 MoonBit SDK 的 WIT 集成层创建 GUI 界面

let counter_value = ref 0

fn init() -> Void {
  // 初始化
  println("WIT Counter Demo initialized")
}

fn update() -> Void {
  // 更新界面
  ()
}

fn increment() -> Void {
  counter_value.val = counter_value.val + 1
  update()
}

fn decrement() -> Void {
  counter_value.val = counter_value.val - 1
  update()
}

fn reset() -> Void {
  counter_value.val = 0
  update()
}

// 导出函数供外部调用
pub fn main() -> Void {
  init()
  increment()
  increment()
  decrement()
  println("Final counter value: \{counter_value.val}")
}
