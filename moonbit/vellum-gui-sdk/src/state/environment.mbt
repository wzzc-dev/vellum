///| Environment module for dependency injection and theme/context propagation
///|
///| This module provides environment functionality similar to SwiftUI @Environment,
///| Compose CompositionLocal, or Flutter InheritedWidget, including:
///| - Type-safe environment values
///| - Environment providers
///| - Environment readers
///| - Predefined common keys (theme, locale, etc.)

pub all type EnvironmentProviderImpl = @environment.EnvironmentProvider
pub all type EnvironmentReaderImpl = @environment.EnvReader
pub all type EnvironmentModifierImpl = @environment.EnvModifier

///| Environment value types
pub enum EnvValue {
  /// String value
  StringValue(String)
  /// Integer value
  IntValue(Int)
  /// Float value
  FloatValue(Double)
  /// Boolean value
  BoolValue(Bool)
}

pub fn EnvValue::to_wit(self : EnvValue) -> @environment.EnvValue {
  match self {
    StringValue(v) => @environment.string({ value: v })
    IntValue(v) => @environment.int({ value: v })
    FloatValue(v) => @environment.float({ value: v })
    BoolValue(v) => @environment.bool({ value: v })
  }
}

///| Environment change event
pub struct EnvChangeEvent {
  /// Key that changed
  pub key : String
  /// New value
  pub value : EnvValue
  /// Previous value (for tracking)
  pub previous_value : Option[EnvValue]
  /// Timestamp of change
  pub timestamp : UInt64
}

///| Environment provider for managing environment values
pub struct EnvironmentProvider {
  provider : EnvironmentProviderImpl
}

pub fn EnvironmentProvider::new() -> EnvironmentProvider {
  { provider: @environment.environment_provider_new() }
}

///| Set a string value
pub fn EnvironmentProvider::set_string(self : EnvironmentProvider, key : String, value : String) -> Unit {
  @environment.environment_provider_set_string(self.provider, key, value)
}

///| Get a string value
pub fn EnvironmentProvider::get_string(self : EnvironmentProvider, key : String) -> Option[String] {
  @environment.environment_provider_get_string(self.provider, key)
}

///| Set an integer value
pub fn EnvironmentProvider::set_int(self : EnvironmentProvider, key : String, value : Int) -> Unit {
  @environment.environment_provider_set_int(self.provider, key, value)
}

///| Get an integer value
pub fn EnvironmentProvider::get_int(self : EnvironmentProvider, key : String) -> Option[Int] {
  @environment.environment_provider_get_int(self.provider, key)
}

///| Set a float value
pub fn EnvironmentProvider::set_float(self : EnvironmentProvider, key : String, value : Double) -> Unit {
  @environment.environment_provider_set_float(self.provider, key, value)
}

///| Get a float value
pub fn EnvironmentProvider::get_float(self : EnvironmentProvider, key : String) -> Option[Double] {
  @environment.environment_provider_get_float(self.provider, key)
}

///| Set a boolean value
pub fn EnvironmentProvider::set_bool(self : EnvironmentProvider, key : String, value : Bool) -> Unit {
  @environment.environment_provider_set_bool(self.provider, key, value)
}

///| Get a boolean value
pub fn EnvironmentProvider::get_bool(self : EnvironmentProvider, key : String) -> Option[Bool] {
  @environment.environment_provider_get_bool(self.provider, key)
}

///| Set a value (type-safe wrapper)
pub fn[T] EnvironmentProvider::set(self : EnvironmentProvider, key : String, value : T) -> Unit {
  match value {
    String(v) => self.set_string(key, v)
    Int(v) => self.set_int(key, v)
    Double(v) => self.set_float(key, v)
    Bool(v) => self.set_bool(key, v)
    _ => ()
  }
}

///| Remove a value
pub fn EnvironmentProvider::remove(self : EnvironmentProvider, key : String) -> Bool {
  @environment.environment_provider_remove(self.provider, key)
}

///| Clear all values
pub fn EnvironmentProvider::clear(self : EnvironmentProvider) -> Unit {
  @environment.environment_provider_clear(self.provider)
}

///| Get all keys
pub fn EnvironmentProvider::keys(self : EnvironmentProvider) -> @list.List[String] {
  @environment.environment_provider_keys(self.provider)
}

///| Check if a key exists
pub fn EnvironmentProvider::has(self : EnvironmentProvider, key : String) -> Bool {
  @environment.environment_provider_has(self.provider, key)
}

///| Subscribe to changes for a key
pub fn EnvironmentProvider::subscribe(self : EnvironmentProvider, key : String) -> Unit {
  @environment.environment_provider_subscribe(self.provider, key)
}

///| Unsubscribe from a key
pub fn EnvironmentProvider::unsubscribe(self : EnvironmentProvider, key : String) -> Unit {
  @environment.environment_provider_unsubscribe(self.provider, key)
}

///| Environment reader for read-only access
pub struct EnvironmentReader {
  reader : EnvironmentReaderImpl
}

pub fn EnvironmentReader::new(provider : EnvironmentProvider) -> EnvironmentReader {
  { reader: @environment.env_reader_new(provider.provider) }
}

///| Read a string with default
pub fn EnvironmentReader::get_string(self : EnvironmentReader, key : String, default : String) -> String {
  @environment.env_reader_get_string(self.reader, key, default)
}

///| Read an int with default
pub fn EnvironmentReader::get_int(self : EnvironmentReader, key : String, default : Int) -> Int {
  @environment.env_reader_get_int(self.reader, key, default)
}

///| Read a float with default
pub fn EnvironmentReader::get_float(self : EnvironmentReader, key : String, default : Double) -> Double {
  @environment.env_reader_get_float(self.reader, key, default)
}

///| Read a bool with default
pub fn EnvironmentReader::get_bool(self : EnvironmentReader, key : String, default : Bool) -> Bool {
  @environment.env_reader_get_bool(self.reader, key, default)
}

///| Environment modifier for conditional updates
pub struct EnvironmentModifier {
  modifier : EnvironmentModifierImpl
}

pub fn EnvironmentModifier::new(provider : EnvironmentProvider) -> EnvironmentModifier {
  { modifier: @environment.env_modifier_new(provider.provider) }
}

///| Set string if not already set
pub fn EnvironmentModifier::set_string_if_absent(self : EnvironmentModifier, key : String, value : String) -> Bool {
  @environment.env_modifier_set_string_if_absent(self.modifier, key, value)
}

///| Increment an integer value
pub fn EnvironmentModifier::increment(self : EnvironmentModifier, key : String, delta : Int) -> Int {
  @environment.env_modifier_increment(self.modifier, key, delta)
}

///| Decrement an integer value
pub fn EnvironmentModifier::decrement(self : EnvironmentModifier, key : String, delta : Int) -> Int {
  @environment.env_modifier_decrement(self.modifier, key, delta)
}

///| Color scheme values
pub enum ColorScheme {
  Light
  Dark
  System
}

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

///| Predefined environment keys
pub module EnvKeys {
  /// Color scheme (light/dark/system)
  pub fn color_scheme() -> String { "colorScheme" }
  /// Primary color hex
  pub fn primary_color() -> String { "primaryColor" }
  /// Accent color hex
  pub fn accent_color() -> String { "accentColor" }
  /// Background color hex
  pub fn background_color() -> String { "backgroundColor" }
  /// Text color hex
  pub fn text_color() -> String { "textColor" }
  /// Font family name
  pub fn font_family() -> String { "fontFamily" }
  /// Font size base
  pub fn font_size_base() -> String { "fontSizeBase" }
  /// Text direction (ltr/rtl)
  pub fn text_direction() -> String { "textDirection" }
  /// Layout direction
  pub fn layout_direction() -> String { "layoutDirection" }
  /// Screen width
  pub fn screen_width() -> String { "screenWidth" }
  /// Screen height
  pub fn screen_height() -> String { "screenHeight" }
  /// Platform type
  pub fn platform() -> String { "platform" }
  /// Current locale (e.g., "en-US")
  pub fn locale() -> String { "locale" }
  /// Language code
  pub fn language() -> String { "language" }
  /// Region code
  pub fn region() -> String { "region" }
  /// Time zone
  pub fn timezone() -> String { "timezone" }
  /// Reduce motion preference
  pub fn reduce_motion() -> String { "reduceMotion" }
  /// High contrast mode
  pub fn high_contrast() -> String { "highContrast" }
  /// Bold text enabled
  pub fn bold_text() -> String { "boldText" }
  /// Current route/path
  pub fn current_route() -> String { "currentRoute" }
  /// Is keyboard visible
  pub fn keyboard_visible() -> String { "keyboardVisible" }
  /// Safe area insets
  pub fn safe_area() -> String { "safeArea" }
}

///| Convenience functions for common environment values
pub module EnvConvenience {
  /// Get color scheme from provider
  pub fn get_color_scheme(provider : EnvironmentProvider) -> ColorScheme {
    match provider.get_string(EnvKeys::color_scheme()) {
      Some(s) => match ColorScheme::from_string(s) {
        Some(cs) => cs
        None => System
      }
      None => System
    }
  }

  /// Set color scheme
  pub fn set_color_scheme(provider : EnvironmentProvider, scheme : ColorScheme) -> Unit {
    provider.set_string(EnvKeys::color_scheme(), scheme.to_string())
  }

  /// Get text direction from provider
  pub fn get_text_direction(provider : EnvironmentProvider) -> TextDirection {
    match provider.get_string(EnvKeys::text_direction()) {
      Some(s) => match TextDirection::from_string(s) {
        Some(td) => td
        None => LeftToRight
      }
      None => LeftToRight
    }
  }

  /// Set text direction
  pub fn set_text_direction(provider : EnvironmentProvider, direction : TextDirection) -> Unit {
    provider.set_string(EnvKeys::text_direction(), direction.to_string())
  }

  /// Check if dark mode is active
  pub fn is_dark_mode(provider : EnvironmentProvider) -> Bool {
    get_color_scheme(provider) == Dark
  }
}
