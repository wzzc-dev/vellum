///| Environment module for dependency injection and theme/context propagation
///|
///| This module provides environment functionality similar to SwiftUI @Environment,
///| Compose CompositionLocal, or Flutter InheritedWidget, including:
///| - Type-safe environment values
///| - Environment providers
///| - Environment readers
///| - Predefined common keys (theme, locale, etc.)

///| Environment value types
pub enum EnvValue {
  StringValue(String)
  IntValue(Int)
  FloatValue(Double)
  BoolValue(Bool)
}

///| Environment change event
pub struct EnvChangeEvent {
  pub key : String
  pub value : EnvValue
  pub previous_value : Option[EnvValue]
  pub timestamp : UInt64
}

///| Environment provider for managing environment values
pub struct EnvironmentProvider {
  mut values : @list.List[(String, EnvValue)]
}

pub fn EnvironmentProvider::new() -> EnvironmentProvider {
  EnvironmentProvider::{ values: @list.empty() }
}

fn find_value(values : @list.List[(String, EnvValue)], key : String) -> Option[EnvValue] {
  match values.head() {
    Some(pair) => {
      let (k, v) = pair
      if k == key {
        Some(v)
      } else {
        find_value(values.unsafe_tail(), key)
      }
    }
    None => None
  }
}

fn remove_value(values : @list.List[(String, EnvValue)], key : String) -> @list.List[(String, EnvValue)] {
  values.filter(fn(pair) { let (k, _) = pair; k != key })
}

///| Set a string value
pub fn EnvironmentProvider::set_string(self : EnvironmentProvider, key : String, value : String) -> Unit {
  self.values = remove_value(self.values, key).add((key, StringValue(value)))
}

///| Get a string value
pub fn EnvironmentProvider::get_string(self : EnvironmentProvider, key : String) -> Option[String] {
  match find_value(self.values, key) {
    Some(StringValue(v)) => Some(v)
    _ => None
  }
}

///| Set an integer value
pub fn EnvironmentProvider::set_int(self : EnvironmentProvider, key : String, value : Int) -> Unit {
  self.values = remove_value(self.values, key).add((key, IntValue(value)))
}

///| Get an integer value
pub fn EnvironmentProvider::get_int(self : EnvironmentProvider, key : String) -> Option[Int] {
  match find_value(self.values, key) {
    Some(IntValue(v)) => Some(v)
    _ => None
  }
}

///| Set a float value
pub fn EnvironmentProvider::set_float(self : EnvironmentProvider, key : String, value : Double) -> Unit {
  self.values = remove_value(self.values, key).add((key, FloatValue(value)))
}

///| Get a float value
pub fn EnvironmentProvider::get_float(self : EnvironmentProvider, key : String) -> Option[Double] {
  match find_value(self.values, key) {
    Some(FloatValue(v)) => Some(v)
    _ => None
  }
}

///| Set a boolean value
pub fn EnvironmentProvider::set_bool(self : EnvironmentProvider, key : String, value : Bool) -> Unit {
  self.values = remove_value(self.values, key).add((key, BoolValue(value)))
}

///| Get a boolean value
pub fn EnvironmentProvider::get_bool(self : EnvironmentProvider, key : String) -> Option[Bool] {
  match find_value(self.values, key) {
    Some(BoolValue(v)) => Some(v)
    _ => None
  }
}

///| Remove a value
pub fn EnvironmentProvider::remove(self : EnvironmentProvider, key : String) -> Bool {
  let had = match find_value(self.values, key) {
    Some(_) => true
    None => false
  }
  if had {
    self.values = remove_value(self.values, key)
  }
  had
}

///| Clear all values
pub fn EnvironmentProvider::clear(self : EnvironmentProvider) -> Unit {
  self.values = @list.empty()
}

///| Get all keys
pub fn EnvironmentProvider::keys(self : EnvironmentProvider) -> @list.List[String] {
  self.values.map(fn(pair) { let (k, _) = pair; k })
}

///| Check if a key exists
pub fn EnvironmentProvider::has(self : EnvironmentProvider, key : String) -> Bool {
  match find_value(self.values, key) {
    Some(_) => true
    None => false
  }
}

///| Environment reader for read-only access
pub struct EnvironmentReader {
  provider : EnvironmentProvider
}

pub fn EnvironmentReader::new(provider : EnvironmentProvider) -> EnvironmentReader {
  { provider: provider }
}

///| Read a string with default
pub fn EnvironmentReader::get_string(self : EnvironmentReader, key : String, default : String) -> String {
  match self.provider.get_string(key) {
    Some(v) => v
    None => default
  }
}

///| Read an int with default
pub fn EnvironmentReader::get_int(self : EnvironmentReader, key : String, default : Int) -> Int {
  match self.provider.get_int(key) {
    Some(v) => v
    None => default
  }
}

///| Read a float with default
pub fn EnvironmentReader::get_float(self : EnvironmentReader, key : String, default : Double) -> Double {
  match self.provider.get_float(key) {
    Some(v) => v
    None => default
  }
}

///| Read a bool with default
pub fn EnvironmentReader::get_bool(self : EnvironmentReader, key : String, default : Bool) -> Bool {
  match self.provider.get_bool(key) {
    Some(v) => v
    None => default
  }
}

///| Color scheme values
pub enum ColorScheme {
  Light
  Dark
  System
} derive(Eq)

pub fn ColorScheme::to_string(self : ColorScheme) -> String {
  match self {
    Light => "light"
    Dark => "dark"
    System => "system"
  }
}

pub fn ColorScheme::from_string(s : String) -> Option[ColorScheme] {
  match s {
    "light" => Some(Light)
    "dark" => Some(Dark)
    "system" => Some(System)
    _ => None
  }
}

///| Text direction values
pub enum TextDirection {
  LeftToRight
  RightToLeft
}

pub fn TextDirection::to_string(self : TextDirection) -> String {
  match self {
    LeftToRight => "ltr"
    RightToLeft => "rtl"
  }
}

pub fn TextDirection::from_string(s : String) -> Option[TextDirection] {
  match s {
    "ltr" => Some(LeftToRight)
    "rtl" => Some(RightToLeft)
    _ => None
  }
}

pub fn env_key_color_scheme() -> String { "colorScheme" }
pub fn env_key_primary_color() -> String { "primaryColor" }
pub fn env_key_accent_color() -> String { "accentColor" }
pub fn env_key_background_color() -> String { "backgroundColor" }
pub fn env_key_text_color() -> String { "textColor" }
pub fn env_key_font_family() -> String { "fontFamily" }
pub fn env_key_font_size_base() -> String { "fontSizeBase" }
pub fn env_key_text_direction() -> String { "textDirection" }
pub fn env_key_layout_direction() -> String { "layoutDirection" }
pub fn env_key_screen_width() -> String { "screenWidth" }
pub fn env_key_screen_height() -> String { "screenHeight" }
pub fn env_key_platform() -> String { "platform" }
pub fn env_key_locale() -> String { "locale" }
pub fn env_key_language() -> String { "language" }
pub fn env_key_region() -> String { "region" }
pub fn env_key_timezone() -> String { "timezone" }
pub fn env_key_reduce_motion() -> String { "reduceMotion" }
pub fn env_key_high_contrast() -> String { "highContrast" }
pub fn env_key_bold_text() -> String { "boldText" }
pub fn env_key_current_route() -> String { "currentRoute" }
pub fn env_key_keyboard_visible() -> String { "keyboardVisible" }
pub fn env_key_safe_area() -> String { "safeArea" }

pub fn get_color_scheme(provider : EnvironmentProvider) -> ColorScheme {
  match provider.get_string(env_key_color_scheme()) {
    Some(s) => match ColorScheme::from_string(s) {
      Some(cs) => cs
      None => System
    }
    None => System
  }
}

pub fn set_color_scheme(provider : EnvironmentProvider, scheme : ColorScheme) -> Unit {
  provider.set_string(env_key_color_scheme(), scheme.to_string())
}

pub fn get_text_direction(provider : EnvironmentProvider) -> TextDirection {
  match provider.get_string(env_key_text_direction()) {
    Some(s) => match TextDirection::from_string(s) {
      Some(td) => td
      None => LeftToRight
    }
    None => LeftToRight
  }
}

pub fn set_text_direction(provider : EnvironmentProvider, direction : TextDirection) -> Unit {
  provider.set_string(env_key_text_direction(), direction.to_string())
}

pub fn is_dark_mode(provider : EnvironmentProvider) -> Bool {
  get_color_scheme(provider) == Dark
}