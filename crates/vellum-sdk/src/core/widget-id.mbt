// Vellum SDK - Widget ID 生成器

// 唯一 ID 生成器
struct IdGenerator {
  mut next_id: U64,
}

let gen = IdGenerator { next_id: 0 }

pub fn next_widget_id(): U64 {
  gen.next_id = gen.next_id + 1
  gen.next_id - 1
}
