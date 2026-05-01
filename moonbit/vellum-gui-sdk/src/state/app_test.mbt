///|
/// Tests for AppOptions.
test "app options new" {
  let options = AppOptions::new("MyApp", "1.0.0")
  assert_eq(options.get_name(), "MyApp")
  assert_eq(options.get_version(), "1.0.0")
}

test "app options with author" {
  let options = AppOptions::new("MyApp", "1.0.0").with_author("John Doe")
  assert_eq(options.get_author(), "John Doe")
}

///|
/// Tests for WindowOptions.
test "window options new" {
  let options = WindowOptions::new("Main Window")
  assert_eq(options.get_title(), "Main Window")
  assert_eq(options.get_width(), 800)
  assert_eq(options.get_height(), 600)
}

test "window options with size" {
  let options = WindowOptions::new("Main").with_size(1024, 768)
  assert_eq(options.get_width(), 1024)
  assert_eq(options.get_height(), 768)
}

test "window options with resizable" {
  let options = WindowOptions::new("Main").with_resizable(false)
  assert_eq(options.is_resizable(), false)
}

test "window options with fullscreen" {
  let options = WindowOptions::new("Main").with_fullscreen(true)
  assert_eq(options.is_fullscreen(), true)
}

///|
/// Tests for Window.
test "window new" {
  let options = WindowOptions::new("Test")
  let window = Window::new("win-1", options)
  assert_eq(window.get_id(), "win-1")
}

test "window mount" {
  let options = WindowOptions::new("Test")
  let root = Widget::new("column", "root")
  let window = Window::new("win-1", options).mount(root)
  assert_eq(window.get_root_widget().get_id(), "root")
}

///|
/// Tests for App.
test "app create" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options)
  assert_eq(app.is_running(), false)
  assert_eq(app.window_count(), 0)
}

test "app create window" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options)
  let (new_app, id) = app.create_window(WindowOptions::new("Main"))
  assert_eq(new_app.window_count(), 1)
}

test "app close window" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options)
  let (app2, id) = app.create_window(WindowOptions::new("Main"))
  let app3 = app2.close_window(id)
  assert_eq(app3.window_count(), 0)
}

test "app run" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options).run()
  assert_eq(app.is_running(), true)
}

test "app quit" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options).run().quit()
  assert_eq(app.is_running(), false)
}

test "app mount widget" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options)
  let (app2, id) = app.create_window(WindowOptions::new("Main"))
  let root = Widget::new("column", "root")
  let app3 = app2.mount_widget(id, root)
  match app3.get_window(id) {
    Some(w) => assert_eq(w.get_root_widget().get_id(), "root")
    None => assert_eq(0, 1)
  }
}

test "app get options" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options)
  let opts = app.get_options()
  assert_eq(opts.get_name(), "MyApp")
}

test "app multiple windows" {
  let options = AppOptions::new("MyApp", "1.0.0")
  let app = App::create(options)
  let (app2, id1) = app.create_window(WindowOptions::new("Win1"))
  let (app3, id2) = app2.create_window(WindowOptions::new("Win2"))
  assert_eq(app3.window_count(), 2)
}
