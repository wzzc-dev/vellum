///|
/// Comprehensive tests for Text widget.
test "text creation" {
  let text = Text::new("Hello")
  inspect(text.get_content(), content="Hello")
  inspect(text.get_font_size(), content="14")
  inspect(text.get_font_weight(), content="400")
  inspect(text.get_color(), content="#000000")
}

test "text with custom font size" {
  let text = Text::new("Large Text").with_font_size(24.0)
  inspect(text.get_font_size(), content="24")
}

test "text with custom font weight" {
  let text = Text::new("Bold Text").with_font_weight(700.0)
  inspect(text.get_font_weight(), content="700")
}

test "text with custom color" {
  let text = Text::new("Red Text").with_color("#FF0000")
  inspect(text.get_color(), content="#FF0000")
}

test "text chaining properties" {
  let text = Text::new("Styled")
    .with_font_size(18.0)
    .with_font_weight(600.0)
    .with_color("#00FF00")
  inspect(text.get_content(), content="Styled")
  inspect(text.get_font_size(), content="18")
  inspect(text.get_font_weight(), content="600")
  inspect(text.get_color(), content="#00FF00")
}

test "heading level 1" {
  let h1 = Text::heading("Title", 1)
  inspect(h1.get_content(), content="Title")
  inspect(h1.get_font_size(), content="32")
  inspect(h1.get_font_weight(), content="700")
}

test "heading level 2" {
  let h2 = Text::heading("Subtitle", 2)
  inspect(h2.get_font_size(), content="24")
}

test "heading level 3" {
  let h3 = Text::heading("Section", 3)
  inspect(h3.get_font_size(), content="20")
}

test "heading level 4" {
  let h4 = Text::heading("Subsection", 4)
  inspect(h4.get_font_size(), content="18")
}

test "heading level 5" {
  let h5 = Text::heading("Minor", 5)
  inspect(h5.get_font_size(), content="16")
}

test "heading level 6" {
  let h6 = Text::heading("Smallest", 6)
  inspect(h6.get_font_size(), content="14")
}

test "heading default size for invalid level" {
  let h_invalid = Text::heading("Too High", 10)
  inspect(h_invalid.get_font_size(), content="14")
}

test "text base widget" {
  let text = Text::new("Content")
  let base = text.base()
  inspect(base.get_type(), content="text")
}

test "heading base widget" {
  let heading = Text::heading("Title", 1)
  let base = heading.base()
  inspect(base.get_type(), content="heading")
}

test "text strikethrough default" {
  let text = Text::new("Hello")
  inspect(text.get_strikethrough(), content="false")
}

test "text strikethrough enabled" {
  let text = Text::new("Hello").strikethrough(true)
  inspect(text.get_strikethrough(), content="true")
}

test "text on_double_click" {
  let mut clicked = false
  let text = Text::new("Click me").on_double_click(fn() { clicked = true })
  text.emit_double_click()
  inspect(clicked, content="true")
}

test "text strikethrough chainable" {
  let text = Text::new("Styled")
    .with_font_size(16.0)
    .strikethrough(true)
    .with_color("#FF0000")
  inspect(text.get_strikethrough(), content="true")
  inspect(text.get_font_size(), content="16")
}
