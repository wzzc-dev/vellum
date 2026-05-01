///|
/// Comprehensive tests for Text widget.
test "text creation" {
  let text = Text::new("Hello")
  assert_eq(text.get_content(), "Hello")
  assert_eq(text.get_font_size(), 14.0)
  assert_eq(text.get_font_weight(), 400.0)
  assert_eq(text.get_color(), "#000000")
}

test "text with custom font size" {
  let text = Text::new("Large Text").with_font_size(24.0)
  assert_eq(text.get_font_size(), 24.0)
}

test "text with custom font weight" {
  let text = Text::new("Bold Text").with_font_weight(700.0)
  assert_eq(text.get_font_weight(), 700.0)
}

test "text with custom color" {
  let text = Text::new("Red Text").with_color("#FF0000")
  assert_eq(text.get_color(), "#FF0000")
}

test "text chaining properties" {
  let text = Text::new("Styled")
    .with_font_size(18.0)
    .with_font_weight(600.0)
    .with_color("#00FF00")
  assert_eq(text.get_content(), "Styled")
  assert_eq(text.get_font_size(), 18.0)
  assert_eq(text.get_font_weight(), 600.0)
  assert_eq(text.get_color(), "#00FF00")
}

test "heading level 1" {
  let h1 = Text::heading("Title", 1)
  assert_eq(h1.get_content(), "Title")
  assert_eq(h1.get_font_size(), 32.0)
  assert_eq(h1.get_font_weight(), 700.0)
}

test "heading level 2" {
  let h2 = Text::heading("Subtitle", 2)
  assert_eq(h2.get_font_size(), 24.0)
}

test "heading level 3" {
  let h3 = Text::heading("Section", 3)
  assert_eq(h3.get_font_size(), 20.0)
}

test "heading level 4" {
  let h4 = Text::heading("Subsection", 4)
  assert_eq(h4.get_font_size(), 18.0)
}

test "heading level 5" {
  let h5 = Text::heading("Minor", 5)
  assert_eq(h5.get_font_size(), 16.0)
}

test "heading level 6" {
  let h6 = Text::heading("Smallest", 6)
  assert_eq(h6.get_font_size(), 14.0)
}

test "heading default size for invalid level" {
  let h_invalid = Text::heading("Too High", 10)
  assert_eq(h_invalid.get_font_size(), 14.0)
}

test "text base widget" {
  let text = Text::new("Content")
  let base = text.base()
  assert_eq(base.get_type(), "text")
}

test "heading base widget" {
  let heading = Text::heading("Title", 1)
  let base = heading.base()
  assert_eq(base.get_type(), "heading")
}
