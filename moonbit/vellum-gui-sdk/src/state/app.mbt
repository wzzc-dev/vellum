///|
/// AppOptions represents application configuration.
pub struct AppOptions {
  name : String
  version : String
  author : String
}

///|
/// Create new AppOptions.
pub fn AppOptions::new(name : String, version : String) -> AppOptions {
  { name: name, version: version, author: "" }
}

///|
/// Set the author.
pub fn AppOptions::with_author(self : AppOptions, author : String) -> AppOptions {
  { ..self, author: author }
}

///|
/// Get the app name.
pub fn AppOptions::get_name(self : AppOptions) -> String {
  self.name
}

///|
/// Get the app version.
pub fn AppOptions::get_version(self : AppOptions) -> String {
  self.version
}

///|
/// Get the author.
pub fn AppOptions::get_author(self : AppOptions) -> String {
  self.author
}

///|
/// WindowOptions represents window configuration.
pub struct WindowOptions {
  title : String
  width : Int
  height : Int
  min_width : Int
  min_height : Int
  resizable : Bool
  fullscreen : Bool
}

///|
/// Create new WindowOptions with a title.
pub fn WindowOptions::new(title : String) -> WindowOptions {
  {
    title: title,
    width: 800,
    height: 600,
    min_width: 200,
    min_height: 150,
    resizable: true,
    fullscreen: false,
  }
}

///|
/// Set the window size.
pub fn WindowOptions::with_size(self : WindowOptions, width : Int, height : Int) -> WindowOptions {
  { ..self, width: width, height: height }
}

///|
/// Set the minimum size.
pub fn WindowOptions::with_min_size(self : WindowOptions, min_width : Int, min_height : Int) -> WindowOptions {
  { ..self, min_width: min_width, min_height: min_height }
}

///|
/// Set whether the window is resizable.
pub fn WindowOptions::with_resizable(self : WindowOptions, resizable : Bool) -> WindowOptions {
  { ..self, resizable: resizable }
}

///|
/// Set whether the window is fullscreen.
pub fn WindowOptions::with_fullscreen(self : WindowOptions, fullscreen : Bool) -> WindowOptions {
  { ..self, fullscreen: fullscreen }
}

///|
/// Get the title.
pub fn WindowOptions::get_title(self : WindowOptions) -> String {
  self.title
}

///|
/// Get the width.
pub fn WindowOptions::get_width(self : WindowOptions) -> Int {
  self.width
}

///|
/// Get the height.
pub fn WindowOptions::get_height(self : WindowOptions) -> Int {
  self.height
}

///|
/// Check if the window is resizable.
pub fn WindowOptions::is_resizable(self : WindowOptions) -> Bool {
  self.resizable
}

///|
/// Check if the window is fullscreen.
pub fn WindowOptions::is_fullscreen(self : WindowOptions) -> Bool {
  self.fullscreen
}

///|
/// Window represents an application window.
pub struct Window {
  id : String
  options : WindowOptions
  root_widget : Widget
}

///|
/// Create a new Window.
pub fn Window::new(id : String, options : WindowOptions) -> Window {
  {
    id: id,
    options: options,
    root_widget: Widget::new("empty", "empty"),
  }
}

///|
/// Mount a root widget.
pub fn Window::mount(self : Window, widget : Widget) -> Window {
  { ..self, root_widget: widget }
}

///|
/// Get the window ID.
pub fn Window::get_id(self : Window) -> String {
  self.id
}

///|
/// Get the window options.
pub fn Window::get_options(self : Window) -> WindowOptions {
  self.options
}

///|
/// Get the root widget.
pub fn Window::get_root_widget(self : Window) -> Widget {
  self.root_widget
}

///|
/// App represents the application instance.
pub struct App {
  options : AppOptions
  windows : @list.List[Window]
  is_running : Bool
  window_counter : Int
}

///|
/// Create a new App.
pub fn App::create(options : AppOptions) -> App {
  {
    options: options,
    windows: @list.empty(),
    is_running: false,
    window_counter: 0,
  }
}

fn make_window_id(counter : Int) -> String {
  "window-" + int_to_string(counter)
}

fn int_to_string(n : Int) -> String {
  if n < 10 {
    char_to_string(int_to_char(n))
  } else {
    int_to_string(n / 10) + char_to_string(int_to_char(n % 10))
  }
}

fn int_to_char(n : Int) -> Char {
  match n {
    0 => '0'
    1 => '1'
    2 => '2'
    3 => '3'
    4 => '4'
    5 => '5'
    6 => '6'
    7 => '7'
    8 => '8'
    _ => '9'
  }
}

fn char_to_string(c : Char) -> String {
  match c {
    '0' => "0"
    '1' => "1"
    '2' => "2"
    '3' => "3"
    '4' => "4"
    '5' => "5"
    '6' => "6"
    '7' => "7"
    '8' => "8"
    _ => "9"
  }
}

///|
/// Create a new window and return its ID.
pub fn App::create_window(self : App, options : WindowOptions) -> (App, String) {
  let counter = self.window_counter + 1
  let id = make_window_id(counter)
  let window = Window::new(id, options)
  let new_app = { ..self, windows: self.windows.add(window), window_counter: counter }
  (new_app, id)
}

///|
/// Close a window by ID.
pub fn App::close_window(self : App, window_id : String) -> App {
  let filtered = self.windows.filter(
    fn(w) {
      w.get_id() != window_id
    }
  )
  { ..self, windows: filtered }
}

///|
/// Mount a widget to a window.
pub fn App::mount_widget(self : App, window_id : String, widget : Widget) -> App {
  let updated = self.windows.map(
    fn(w) {
      if w.get_id() == window_id {
        w.mount(widget)
      } else {
        w
      }
    }
  )
  { ..self, windows: updated }
}

///|
/// Start the application.
pub fn App::run(self : App) -> App {
  { ..self, is_running: true }
}

///|
/// Quit the application.
pub fn App::quit(self : App) -> App {
  { ..self, is_running: false }
}

///|
/// Check if the app is running.
pub fn App::is_running(self : App) -> Bool {
  self.is_running
}

///|
/// Get the app options.
pub fn App::get_options(self : App) -> AppOptions {
  self.options
}

///|
/// Get the number of windows.
pub fn App::window_count(self : App) -> Int {
  self.windows.length()
}

///|
/// Find a window by ID.
fn find_window(windows : @list.List[Window], window_id : String) -> Window? {
  match windows.length() {
    0 => None
    _ => {
      match windows.head() {
        Some(head) => {
          if head.get_id() == window_id {
            Some(head)
          } else {
            find_window(windows.unsafe_tail(), window_id)
          }
        }
        None => None
      }
    }
  }
}

///|
/// Get a window by ID.
pub fn App::get_window(self : App, window_id : String) -> Window? {
  find_window(self.windows, window_id)
}
