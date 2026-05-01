// vellum:gui/window 类型绑定

pub struct WindowOptions {
  title: String,
  width: u32,
  height: u32,
  min_width: u32,
  min_height: u32,
  max_width: u32,
  max_height: u32,
  resizable: bool,
  decorated: bool,
  transparent: bool,
  always_on_top: bool,
  center: bool,
} derive(Debug)

pub fn window_options_default()->WindowOptions {
  WindowOptions::{
    title: "",
    width: 800,
    height: 600,
    min_width: 0,
    min_height: 0,
    max_width: 0,
    max_height: 0,
    resizable: true,
    decorated: true,
    transparent: false,
    always_on_top: false,
    center: true,
  }
}
