///|
/// Tests for Image component.
test "Image::new creates image" {
  let image = Image::new("test.png")
  inspect(image.get_source(), content="test.png")
  inspect(image.get_alt(), content="")
}

test "Image::with_alt sets alt text" {
  let image = Image::new("photo.jpg").with_alt("A beautiful sunset")
  inspect(image.get_alt(), content="A beautiful sunset")
}

test "Image::with_size sets dimensions" {
  let image = Image::new("test.png").with_size(100.0, 200.0)
  inspect(image.get_width(), content="100")
  inspect(image.get_height(), content="200")
}

test "Image::with_fit sets fit mode" {
  let image = Image::new("test.png").with_fit(Cover)
  inspect(image.get_fit(), content="Cover")
}

test "Image::base returns widget" {
  let image = Image::new("test.png")
  let widget = image.base()
  inspect(widget.get_type(), content="image")
}

test "Image::get_source returns source" {
  let image = Image::new("https://example.com/image.png")
  inspect(image.get_source(), content="https://example.com/image.png")
}

test "Image::with_size chainable" {
  let image = Image::new("test.png").with_size(50.0, 75.0).with_alt("desc")
  inspect(image.get_width(), content="50")
  inspect(image.get_alt(), content="desc")
}

test "Image default dimensions are zero" {
  let image = Image::new("test.png")
  inspect(image.get_width(), content="0")
  inspect(image.get_height(), content="0")
}
