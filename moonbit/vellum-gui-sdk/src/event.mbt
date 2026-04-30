// Event system for Vellum GUI Framework

// Event types
pub type Event = @event.types.Event
pub type MouseEvent = @event.types.MouseEvent
pub type KeyEvent = @event.types.KeyEvent
pub type FocusEvent = @event.types.FocusEvent
pub type TouchEvent = @event.types.TouchEvent
pub type ScrollEvent = @event.types.ScrollEvent
pub type DragEvent = @event.types.DragEvent
pub type WindowEvent = @event.types.WindowEvent

// Mouse button types
pub enum MouseButton {
  None,
  Left,
  Right,
  Middle,
  Back,
  Forward,
}

// Key code types
pub enum KeyCode {
  Backspace,
  Tab,
  Enter,
  Shift,
  Control,
  Alt,
  Escape,
  Space,
  ArrowLeft,
  ArrowUp,
  ArrowRight,
  ArrowDown,
  A, B, C, D, E, F, G, H, I, J, K, L, M,
  N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
  F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}

// Key modifiers
pub enum KeyModifiers {
  None,
  Shift,
  Control,
  Alt,
  Meta,
  CapsLock,
  NumLock,
}

// Event handlers
pub type EventHandler = {
  event_type: String,
  callback: (Event) -> Void,
}

pub fn on_click(callback: () -> Void) -> EventHandler {
  {
    event_type: "click",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_double_click(callback: () -> Void) -> EventHandler {
  {
    event_type: "dblclick",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_mouse_down(callback: (MouseEvent) -> Void) -> EventHandler {
  {
    event_type: "mousedown",
    callback: fn(event: Event) {
      match event {
        Event::Mouse(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_mouse_up(callback: (MouseEvent) -> Void) -> EventHandler {
  {
    event_type: "mouseup",
    callback: fn(event: Event) {
      match event {
        Event::Mouse(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_mouse_move(callback: (MouseEvent) -> Void) -> EventHandler {
  {
    event_type: "mousemove",
    callback: fn(event: Event) {
      match event {
        Event::Mouse(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_mouse_enter(callback: () -> Void) -> EventHandler {
  {
    event_type: "mouseenter",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_mouse_leave(callback: () -> Void) -> EventHandler {
  {
    event_type: "mouseleave",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_wheel(callback: (ScrollEvent) -> Void) -> EventHandler {
  {
    event_type: "wheel",
    callback: fn(event: Event) {
      match event {
        Event::Scroll(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_key_down(callback: (KeyEvent) -> Void) -> EventHandler {
  {
    event_type: "keydown",
    callback: fn(event: Event) {
      match event {
        Event::Key(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_key_up(callback: (KeyEvent) -> Void) -> EventHandler {
  {
    event_type: "keyup",
    callback: fn(event: Event) {
      match event {
        Event::Key(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_input(callback: (String) -> Void) -> EventHandler {
  {
    event_type: "input",
    callback: fn(event: Event) {
      match event {
        Event::Input(value) => callback(value)
        _ => ()
      }
    },
  }
}

pub fn on_change(callback: (String) -> Void) -> EventHandler {
  {
    event_type: "change",
    callback: fn(event: Event) {
      match event {
        Event::Change(value) => callback(value)
        _ => ()
      }
    },
  }
}

pub fn on_focus(callback: () -> Void) -> EventHandler {
  {
    event_type: "focus",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_blur(callback: () -> Void) -> EventHandler {
  {
    event_type: "blur",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_scroll(callback: (ScrollEvent) -> Void) -> EventHandler {
  {
    event_type: "scroll",
    callback: fn(event: Event) {
      match event {
        Event::Scroll(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_touch_start(callback: (TouchEvent) -> Void) -> EventHandler {
  {
    event_type: "touchstart",
    callback: fn(event: Event) {
      match event {
        Event::Touch(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_touch_move(callback: (TouchEvent) -> Void) -> EventHandler {
  {
    event_type: "touchmove",
    callback: fn(event: Event) {
      match event {
        Event::Touch(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_touch_end(callback: (TouchEvent) -> Void) -> EventHandler {
  {
    event_type: "touchend",
    callback: fn(event: Event) {
      match event {
        Event::Touch(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_drag_start(callback: (DragEvent) -> Void) -> EventHandler {
  {
    event_type: "dragstart",
    callback: fn(event: Event) {
      match event {
        Event::Drag(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_drag(callback: (DragEvent) -> Void) -> EventHandler {
  {
    event_type: "drag",
    callback: fn(event: Event) {
      match event {
        Event::Drag(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_drag_end(callback: () -> Void) -> EventHandler {
  {
    event_type: "dragend",
    callback: fn(_event: Event) { callback() },
  }
}

pub fn on_drop(callback: (DragEvent) -> Void) -> EventHandler {
  {
    event_type: "drop",
    callback: fn(event: Event) {
      match event {
        Event::Drag(e) => callback(e)
        _ => ()
      }
    },
  }
}

pub fn on_resize(callback: (U32, U32) -> Void) -> EventHandler {
  {
    event_type: "resize",
    callback: fn(event: Event) {
      match event {
        Event::Window(WindowEvent::Resize(w, h)) => callback(w, h)
        _ => ()
      }
    },
  }
}

// Event target
pub type EventTarget = @event.types.EventTarget

// Event propagation
pub fn stop_propagation(event: Event) -> Void {
  @event.stop_propagation(event)
}

pub fn prevent_default(event: Event) -> Void {
  @event.prevent_default(event)
}

// Event creation helpers
pub fn create_mouse_event(
  kind: String,
  x: Float,
  y: Float,
  button: MouseButton,
) -> MouseEvent {
  {
    kind,
    position: { x, y },
    global_position: { x, y },
    button,
    click_count: 1,
    modifiers: KeyModifiers::None,
  }
}

pub fn create_key_event(
  kind: String,
  code: KeyCode,
  key: String,
) -> KeyEvent {
  {
    kind,
    code,
    key,
    modifiers: KeyModifiers::None,
  }
}
