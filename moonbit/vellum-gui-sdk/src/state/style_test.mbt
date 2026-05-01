///|
/// Tests for Color.
test "color new" {
  let color = Color::new(1.0, 0.5, 0.0, 1.0)
  assert_eq(color.get_r(), 1.0)
  assert_eq(color.get_g(), 0.5)
  assert_eq(color.get_b(), 0.0)
  assert_eq(color.get_a(), 1.0)
}

test "color from rgb" {
  let color = Color::from_rgb(0.5, 0.5, 0.5)
  assert_eq(color.get_a(), 1.0)
}

test "color transparent" {
  let color = Color::transparent()
  assert_eq(color.get_a(), 0.0)
}

test "color black" {
  let color = Color::black()
  assert_eq(color.get_r(), 0.0)
  assert_eq(color.get_g(), 0.0)
  assert_eq(color.get_b(), 0.0)
  assert_eq(color.get_a(), 1.0)
}

test "color white" {
  let color = Color::white()
  assert_eq(color.get_r(), 1.0)
  assert_eq(color.get_g(), 1.0)
  assert_eq(color.get_b(), 1.0)
}

test "color red" {
  let color = Color::red()
  assert_eq(color.get_r(), 1.0)
  assert_eq(color.get_g(), 0.0)
  assert_eq(color.get_b(), 0.0)
}

test "color green" {
  let color = Color::green()
  assert_eq(color.get_r(), 0.0)
  assert_eq(color.get_g(), 1.0)
  assert_eq(color.get_b(), 0.0)
}

test "color blue" {
  let color = Color::blue()
  assert_eq(color.get_r(), 0.0)
  assert_eq(color.get_g(), 0.0)
  assert_eq(color.get_b(), 1.0)
}

test "color gray" {
  let color = Color::gray()
  assert_eq(color.get_r(), 0.5)
  assert_eq(color.get_g(), 0.5)
  assert_eq(color.get_b(), 0.5)
}

test "color with alpha" {
  let color = Color::red().with_alpha(0.5)
  assert_eq(color.get_r(), 1.0)
  assert_eq(color.get_a(), 0.5)
}

///|
/// Tests for Border.
test "border new" {
  let border = Border::new(1.0, Color::red(), 4.0)
  assert_eq(border.get_width(), 1.0)
  assert_eq(border.get_radius(), 4.0)
}

test "border width only" {
  let border = Border::width(2.0)
  assert_eq(border.get_width(), 2.0)
  assert_eq(border.get_radius(), 0.0)
}

test "border radius only" {
  let border = Border::radius(8.0)
  assert_eq(border.get_width(), 0.0)
  assert_eq(border.get_radius(), 8.0)
}

test "border chaining" {
  let border = Border::new(1.0, Color::black(), 0.0)
    .with_width(2.0)
    .with_radius(4.0)
  assert_eq(border.get_width(), 2.0)
  assert_eq(border.get_radius(), 4.0)
}

///|
/// Tests for Shadow.
test "shadow new" {
  let shadow = Shadow::new(0.0, 4.0, 8.0, Color::black())
  assert_eq(shadow.get_x(), 0.0)
  assert_eq(shadow.get_y(), 4.0)
  assert_eq(shadow.get_blur(), 8.0)
}

test "shadow offset" {
  let shadow = Shadow::offset(2.0, 2.0)
  assert_eq(shadow.get_x(), 2.0)
  assert_eq(shadow.get_y(), 2.0)
}

test "shadow blur" {
  let shadow = Shadow::blur(10.0)
  assert_eq(shadow.get_blur(), 10.0)
}

test "shadow chaining" {
  let shadow = Shadow::new(0.0, 0.0, 0.0, Color::black())
    .with_x(1.0)
    .with_y(2.0)
    .with_blur(4.0)
  assert_eq(shadow.get_x(), 1.0)
  assert_eq(shadow.get_y(), 2.0)
  assert_eq(shadow.get_blur(), 4.0)
}

///|
/// Tests for Style.
test "style new" {
  let style = Style::new()
  assert_eq(style.get_opacity(), 1.0)
}

test "style with background" {
  let style = Style::new().with_background(Color::red())
  assert_eq(style.get_background().get_r(), 1.0)
}

test "style with foreground" {
  let style = Style::new().with_foreground(Color::blue())
  assert_eq(style.get_foreground().get_b(), 1.0)
}

test "style with opacity" {
  let style = Style::new().with_opacity(0.5)
  assert_eq(style.get_opacity(), 0.5)
}

test "style chaining" {
  let style = Style::new()
    .with_background(Color::white())
    .with_foreground(Color::black())
    .with_opacity(0.8)
  assert_eq(style.get_background().get_r(), 1.0)
  assert_eq(style.get_foreground().get_r(), 0.0)
  assert_eq(style.get_opacity(), 0.8)
}

test "style with border" {
  let border = Border::new(1.0, Color::red(), 4.0)
  let style = Style::new().with_border(border)
  assert_eq(style.get_border().get_width(), 1.0)
}

test "style with shadow" {
  let shadow = Shadow::new(0.0, 4.0, 8.0, Color::black())
  let style = Style::new().with_shadow(shadow)
  assert_eq(style.get_shadow().get_y(), 4.0)
}
