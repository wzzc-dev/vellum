///| Navigation module for declarative routing
///|
///| This module provides navigation functionality similar to SwiftUI NavigationStack
///| or Compose Navigation, including:
///| - Declarative routing
///| - Stack-based navigation (push/pop)
///| - Tab navigation
///| - Modal/Sheet support
///| - Deep linking

///| Navigation state record
pub struct NavigationState {
  pub current_route : String
  pub params : @list.List[(String, String)]
  pub can_go_back : Bool
  pub stack_depth : Int
}

///| Navigation parameters
pub struct NavigationParams {
  pub params : @list.List[(String, String)]
  pub replace : Bool
  pub transition : Option[TransitionType]
}

pub fn NavigationParams::new() -> NavigationParams {
  {
    params: @list.empty(),
    replace: false,
    transition: None
  }
}

///| Transition animation types
pub enum TransitionType {
  PushRight
  PushBottom
  PushLeft
  PushTop
  Fade
  None
}

///| Navigator for programmatic navigation
pub struct Navigator {
  pub initial_route : String
  pub current_route : String
  pub history : @list.List[String]
}

pub fn Navigator::new(initial_route : String) -> Navigator {
  {
    initial_route: initial_route,
    current_route: initial_route,
    history: @list.empty().add(initial_route)
  }
}

///| Navigate to a new route
pub fn Navigator::push(self : Navigator, route : String) -> Navigator {
  {
    initial_route: self.initial_route,
    current_route: route,
    history: self.history.add(route)
  }
}

///| Go back to previous route
pub fn Navigator::pop(self : Navigator) -> Navigator {
  match self.history.length() > 1 {
    true => {
      let prev = self.history.unsafe_tail()
      match prev.head() {
        Some(route) => {
          {
            initial_route: self.initial_route,
            current_route: route,
            history: prev
          }
        }
        None => self
      }
    }
    false => self
  }
}

///| Replace current route without adding to history
pub fn Navigator::replace(self : Navigator, route : String) -> Navigator {
  {
    initial_route: self.initial_route,
    current_route: route,
    history: self.history
  }
}

///| Check if we can go back
pub fn Navigator::can_go_back(self : Navigator) -> Bool {
  self.history.length() > 1
}

///| Get current navigation state
pub fn Navigator::get_state(self : Navigator) -> NavigationState {
  {
    current_route: self.current_route,
    params: @list.empty(),
    can_go_back: self.can_go_back(),
    stack_depth: self.history.length()
  }
}

///| Clear all navigation history
pub fn Navigator::clear_history(self : Navigator) -> Navigator {
  {
    initial_route: self.initial_route,
    current_route: self.current_route,
    history: @list.empty().add(self.current_route)
  }
}

///| Set the root route
pub fn Navigator::set_root(self : Navigator, route : String) -> Navigator {
  {
    initial_route: route,
    current_route: route,
    history: @list.empty().add(route)
  }
}

///| Route configuration for NavHost
pub struct RouteConfig {
  pub path : String
  pub name : Option[String]
  pub requires_auth : Bool
}

pub fn RouteConfig::new(path : String) -> RouteConfig {
  {
    path: path,
    name: None,
    requires_auth: false
  }
}

pub fn RouteConfig::with_name(self : RouteConfig, name : String) -> RouteConfig {
  { ..self, name: Some(name) }
}

pub fn RouteConfig::with_auth(self : RouteConfig) -> RouteConfig {
  { ..self, requires_auth: true }
}

///| NavHost configuration
pub struct NavHostConfig {
  pub initial_route : String
  pub routes : @list.List[RouteConfig]
  pub default_transition : TransitionType
  pub show_back_button : Bool
  pub deep_link_base : Option[String]
}

pub fn NavHostConfig::new(initial_route : String) -> NavHostConfig {
  {
    initial_route: initial_route,
    routes: @list.empty(),
    default_transition: TransitionType::PushRight,
    show_back_button: true,
    deep_link_base: None
  }
}

pub fn NavHostConfig::with_routes(self : NavHostConfig, routes : @list.List[RouteConfig]) -> NavHostConfig {
  { ..self, routes: routes }
}

pub fn NavHostConfig::with_transition(self : NavHostConfig, transition : TransitionType) -> NavHostConfig {
  { ..self, default_transition: transition }
}

pub fn NavHostConfig::with_back_button(self : NavHostConfig, show : Bool) -> NavHostConfig {
  { ..self, show_back_button: show }
}

///| Route builder function type
pub type RouteBuilder = (Unit) -> Widget

///| Route definition
pub struct Route {
  pub path : String
  pub name : String
  builder : RouteBuilder
  pub requires_auth : Bool
}

pub fn Route::new(path : String, name : String, builder : RouteBuilder) -> Route {
  {
    path: path,
    name: name,
    builder: builder,
    requires_auth: false
  }
}

pub fn Route::with_auth(self : Route) -> Route {
  { ..self, requires_auth: true }
}

///| Navigation event types
pub enum NavigationEvent {
  Pushed(String, @list.List[(String, String)])
  Popped(String, String)
  Replaced(String, String)
  StateChanged(NavigationState)
  DeepLink(String)
}

///| Navigation controller for managing navigation state
pub struct NavigationController {
  pub config : NavHostConfig
  pub navigator : Navigator
}

pub fn NavigationController::new(config : NavHostConfig) -> NavigationController {
  {
    config: config,
    navigator: Navigator::new(config.initial_route)
  }
}

///| Register a route
pub fn NavigationController::register_route(self : NavigationController, route : RouteConfig) -> NavigationController {
  { ..self, config: { ..self.config, routes: self.config.routes.add(route) } }
}

///| Unregister a route
pub fn NavigationController::unregister_route(self : NavigationController, path : String) -> NavigationController {
  let filtered = self.config.routes.filter(fn(r) { r.path != path })
  { ..self, config: { ..self.config, routes: filtered } }
}

///| Start navigation
pub fn NavigationController::start(self : NavigationController) -> Navigator {
  self.navigator
}

///| Get navigator instance
pub fn NavigationController::get_navigator(self : NavigationController) -> Navigator {
  self.navigator
}

///| Tab bar item
pub struct TabBarItem {
  pub id : String
  pub label : String
  pub icon : Option[String]
  pub enabled : Bool
}

pub fn TabBarItem::new(id : String, label : String) -> TabBarItem {
  {
    id: id,
    label: label,
    icon: None,
    enabled: true
  }
}

pub fn TabBarItem::with_icon(self : TabBarItem, icon : String) -> TabBarItem {
  { ..self, icon: Some(icon) }
}

pub fn TabBarItem::disabled(self : TabBarItem) -> TabBarItem {
  { ..self, enabled: false }
}

///| Tab bar for managing tabs
pub struct TabBar {
  pub selected_index : Int
  pub items : @list.List[TabBarItem]
}

pub fn TabBar::new(items : @list.List[TabBarItem]) -> TabBar {
  {
    selected_index: 0,
    items: items
  }
}