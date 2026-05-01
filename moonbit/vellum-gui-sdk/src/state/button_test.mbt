///|
/// Comprehensive tests for Button widget.
test "button creation" {
  let btn = Button::new("Click Me")
  assert_eq(btn.get_label(), "Click Me")
  match btn.get_type() {
    Normal => ()
    _ => assert_eq(0, 1)
  }
  assert_eq(btn.is_disabled(), false)
}

test "icon button creation" {
  let icon_btn = Button::icon("save")
  assert_eq(icon_btn.get_label(), "save")
  match icon_btn.get_type() {
    Icon => ()
    _ => assert_eq(0, 1)
  }
}

test "text button creation" {
  let text_btn = Button::text("OK")
  assert_eq(text_btn.get_label(), "OK")
  match text_btn.get_type() {
    Text => ()
    _ => assert_eq(0, 1)
  }
}

test "button with disabled" {
  let btn = Button::new("Disabled").with_disabled(true)
  assert_eq(btn.is_disabled(), true)
}

test "button chaining disabled" {
  let btn = Button::new("Toggle")
    .with_disabled(false)
    .with_disabled(true)
  assert_eq(btn.is_disabled(), true)
}

test "button base widget" {
  let btn = Button::new("Test")
  let base = btn.base()
  assert_eq(base.get_type(), "button")
}

test "icon button base widget" {
  let btn = Button::icon("menu")
  let base = btn.base()
  assert_eq(base.get_type(), "icon-button")
}

test "text button base widget" {
  let btn = Button::text("Cancel")
  let base = btn.base()
  assert_eq(base.get_type(), "text-button")
}

test "normal button type" {
  let btn = Button::new("Normal")
  match btn.get_type() {
    Normal => ()
    _ => assert_eq(0, 1)
  }
}

test "enabled button" {
  let btn = Button::new("Enabled")
  assert_eq(btn.is_disabled(), false)
}

test "button label preservation" {
  let btn = Button::new("Preserve This Label")
    .with_disabled(true)
  assert_eq(btn.get_label(), "Preserve This Label")
}

test "icon button label from name" {
  let icon_btn = Button::icon("close")
  assert_eq(icon_btn.get_label(), "close")
  assert_eq(icon_btn.is_disabled(), false)
}
