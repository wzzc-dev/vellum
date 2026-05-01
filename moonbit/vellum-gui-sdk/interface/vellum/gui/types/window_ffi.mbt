// vellum:gui/window FFI 绑定
//
// 本文件包含与 Rust GPUI Adapter 通过 WIT Component Model 通信的 window 相关 FFI 函数

fn window_create(title: String, width: u32, height: u32) -> Int = "vellum_gui_window_create"

fn window_close(window_id: Int) -> Void = "vellum_gui_window_close"

fn window_set_title(window_id: Int, title: String) -> Void = "vellum_gui_window_set_title"

fn window_get_title(window_id: Int) -> String = "vellum_gui_window_get_title"

fn window_set_size(window_id: Int, width: u32, height: u32) -> Void = "vellum_gui_window_set_size"

fn window_minimize(window_id: Int) -> Void = "vellum_gui_window_minimize"

fn window_maximize(window_id: Int) -> Void = "vellum_gui_window_maximize"

fn window_restore(window_id: Int) -> Void = "vellum_gui_window_restore"

fn window_show(window_id: Int) -> Void = "vellum_gui_window_show"

fn window_hide(window_id: Int) -> Void = "vellum_gui_window_hide"

fn window_set_fullscreen(window_id: Int, fullscreen: Bool) -> Void = "vellum_gui_window_set_fullscreen"
