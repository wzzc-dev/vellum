// Vellum SDK - 颜色模块

pub struct Color {
  r: U8,
  g: U8,
  b: U8,
  a: U8,
}

pub fn rgba(r: U8, g: U8, b: U8, a: U8): Color {
  { r, g, b, a }
}

pub fn rgb(r: U8, g: U8, b: U8): Color {
  rgba(r, g, b, 255)
}

// 常用颜色
pub fn white(): Color { rgb(255, 255, 255) }
pub fn black(): Color { rgb(0, 0, 0) }
pub fn red(): Color { rgb(255, 0, 0) }
pub fn green(): Color { rgb(0, 255, 0) }
pub fn blue(): Color { rgb(0, 0, 255) }
pub fn gray(level: U8): Color { rgb(level, level, level) }
