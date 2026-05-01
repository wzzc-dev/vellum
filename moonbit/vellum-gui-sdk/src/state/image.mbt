///|
/// ImageFit represents how an image should fit its container.
pub enum ImageFit {
  Cover
  Contain
  Fill
  None
}

///|
/// Image displays an image from a source URL or path.
pub struct Image {
  base : Widget
  source : String
  alt_text : String
  width : Double
  height : Double
  fit : ImageFit
}

///|
/// Create a new Image with the given source.
pub fn Image::new(source : String) -> Image {
  {
    base: Widget::new("image", "image-gen-1"),
    source: source,
    alt_text: "",
    width: 0.0,
    height: 0.0,
    fit: None,
  }
}

///|
/// Set the alternative text.
pub fn Image::with_alt(self : Image, alt : String) -> Image {
  { ..self, alt_text: alt }
}

///|
/// Set the image dimensions.
pub fn Image::with_size(self : Image, width : Double, height : Double) -> Image {
  { ..self, width: width, height: height }
}

///|
/// Set the fit mode.
pub fn Image::with_fit(self : Image, fit : ImageFit) -> Image {
  { ..self, fit: fit }
}

///|
/// Get the source URL or path.
pub fn Image::get_source(self : Image) -> String {
  self.source
}

///|
/// Get the alternative text.
pub fn Image::get_alt(self : Image) -> String {
  self.alt_text
}

///|
/// Get the width.
pub fn Image::get_width(self : Image) -> Double {
  self.width
}

///|
/// Get the height.
pub fn Image::get_height(self : Image) -> Double {
  self.height
}

///|
/// Get the fit mode.
pub fn Image::get_fit(self : Image) -> ImageFit {
  self.fit
}

///|
/// Get the base widget.
pub fn Image::base(self : Image) -> Widget {
  self.base
}
