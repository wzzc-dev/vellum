///|
/// Mouse event types.
pub enum MouseEventType {
  Click
  MouseDown
  MouseUp
  MouseMove
  MouseEnter
  MouseLeave
}

///|
/// Key event types.
pub enum KeyEventType {
  KeyDown
  KeyUp
}

///|
/// Mouse event data.
pub struct MouseEvent {
  event_type : MouseEventType
  x : Double
  y : Double
}

///|
/// Create a new MouseEvent.
pub fn MouseEvent::new(event_type : MouseEventType, x : Double, y : Double) -> MouseEvent {
  { event_type: event_type, x: x, y: y }
}

///|
/// Get the event type as a string.
pub fn MouseEvent::to_string(self : MouseEvent) -> String {
  match self.event_type {
    Click => "click"
    MouseDown => "mousedown"
    MouseUp => "mouseup"
    MouseMove => "mousemove"
    MouseEnter => "mouseenter"
    MouseLeave => "mouseleave"
  }
}

///|
/// Key event data.
pub struct KeyEvent {
  event_type : KeyEventType
  key_code : String
  key : String
}

///|
/// Create a new KeyEvent.
pub fn KeyEvent::new(event_type : KeyEventType, key_code : String, key : String) -> KeyEvent {
  { event_type: event_type, key_code: key_code, key: key }
}

///|
/// Get the event type as a string.
pub fn KeyEvent::to_string(self : KeyEvent) -> String {
  match self.event_type {
    KeyDown => "keydown"
    KeyUp => "keyup"
  }
}
