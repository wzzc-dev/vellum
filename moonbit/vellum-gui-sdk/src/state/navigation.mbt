///| Navigation module for declarative routing
///|
///| This module provides navigation functionality similar to SwiftUI NavigationStack
///| or Compose Navigation, including:
///| - Declarative routing
///| - Stack-based navigation (push/pop)
///| - Tab navigation
///| - Modal/Sheet support
///| - Deep linking

pub all type NavigatorController = @navigation.Navigator
pub all type NavigationState = @navigation.NavigationState

///| Navigation state record
pub struct NavigationState {
  /// Current route path
  pub current_route : String
  /// Route parameters
  pub params : @list.List[(String, String)]
  /// Whether we can go back
  pub can_go_back : Bool
  /// History stack depth
  pub stack_depth : Int
}

///| Navigation parameters
pub struct NavigationParams {
  /// Parameter key-value pairs
  pub params : @list.List[(String, String)]
  /// Whether to replace current route
  pub replace : Bool
  /// Transition animation type
  pub transition : Option[TransitionType]
}

pub fn NavigationParams::new() -> NavigationParams {
  {
    params: @list.Nil,
    replace: false,
    transition: None
  }
}

///| Transition animation types
pub enum TransitionType {
  /// Push from right (iOS style)
  PushRight
  /// Push from bottom
  PushBottom
  /// Push from left
  PushLeft
  /// Push from top
  PushTop
  /// Fade transition
  Fade
  /// No animation
  None
}

///| Navigator for programmatic navigation
pub struct Navigator {
  controller : NavigatorController
}

pub fn Navigator::new(initial_route : String) -> Navigator {
  { controller: @navigation.navigator_new(initial_route) }
}

///| Navigate to a new route
pub fn Navigator::push(self : Navigator, route : String) -> Unit {
  @navigation.navigator_push(self.controller, route, None)
}

///| Navigate to a route with parameters
pub fn Navigator::push_with_params(self : Navigator, route : String, params : NavigationParams) -> Unit {
  @navigation.navigator_push(self.controller, route, Some(params))
}

///| Go back to previous route
pub fn Navigator::pop(self : Navigator) -> Bool {
  @navigation.navigator_pop(self.controller)
}

///| Replace current route without adding to history
pub fn Navigator::replace(self : Navigator, route : String) -> Unit {
  @navigation.navigator_replace(self.controller, route, None)
}

///| Check if we can go back
pub fn Navigator::can_go_back(self : Navigator) -> Bool {
  @navigation.navigator_can_go_back(self.controller)
}

///| Get current navigation state
pub fn Navigator::get_state(self : Navigator) -> NavigationState {
  @navigation.navigator_get_state(self.controller)
}

///| Clear all navigation history
pub fn Navigator::clear_history(self : Navigator) -> Unit {
  @navigation.navigator_clear_history(self.controller)
}

///| Set the root route
pub fn Navigator::set_root(self : Navigator, route : String) -> Unit {
  @navigation.navigator_set_root(self.controller, route)
}

///| Route configuration for NavHost
pub struct RouteConfig {
  /// Route path pattern (e.g., "/users/:id")
  pub path : String
  /// Route name for debugging
  pub name : Option[String]
  /// Whether this route requires authentication
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
  /// Initial route
  pub initial_route : String
  /// All registered routes
  pub routes : @list.List[RouteConfig]
  /// Default transition animation
  pub default_transition : TransitionType
  /// Whether to show back button
  pub show_back_button : Bool
  /// Deep link base URL
  pub deep_link_base : Option[String]
}

pub fn NavHostConfig::new(initial_route : String) -> NavHostConfig {
  {
    initial_route: initial_route,
    routes: @list.Nil,
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
pub type RouteBuilder = Unit -> Widget

///| Route definition
pub struct Route {
  /// Route path
  pub path : String
  /// Route name
  pub name : String
  /// Widget builder
  builder : RouteBuilder
  /// Requires authentication
  requires_auth : Bool
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

///| Navigation controller for managing navigation state
pub struct NavigationController {
  controller : @navigation.NavigationController
}

pub fn NavigationController::new(config : NavHostConfig) -> NavigationController {
  { controller: @navigation.navigation_controller_new(config) }
}

///| Register a route
pub fn NavigationController::register_route(self : NavigationController, route : RouteConfig) -> Unit {
  @navigation.navigation_controller_register_route(self.controller, route)
}

///| Unregister a route
pub fn NavigationController::unregister_route(self : NavigationController, path : String) -> Unit {
  @navigation.navigation_controller_unregister_route(self.controller, path)
}

///| Start navigation with the given navigator
pub fn NavigationController::start(self : NavigationController, navigator : Navigator) -> Unit {
  @navigation.navigation_controller_start(self.controller, navigator.controller)
}

///| Handle deep link
pub fn NavigationController::handle_deep_link(self : NavigationController, url : String) -> Bool {
  @navigation.navigation_controller_handle_deep_link(self.controller, url)
}

///| Get navigator instance
pub fn NavigationController::get_navigator(self : NavigationController) -> Navigator {
  Navigator::{ controller: @navigation.navigation_controller_get_navigator(self.controller) }
}

///| Navigation event types
pub enum NavigationEvent {
  /// Route was pushed onto stack
  Pushed(route : String, params : @list.List[(String, String)])
  /// Route was popped from stack
  Popped(route : String, popped_route : String)
  /// Route was replaced
  Replaced(old_route : String, new_route : String)
  /// Navigation state changed
  StateChanged(state : NavigationState)
  /// Deep link received
  DeepLink(url : String)
}

///| Tab bar item
pub struct TabBarItem {
  /// Tab identifier
  pub id : String
  /// Tab label
  pub label : String
  /// Tab icon (optional)
  pub icon : Option[String]
  /// Whether the tab is enabled
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

///| Tab bar controller for managing tabs
pub struct TabBar {
  /// Current selected tab index
  pub selected_index : Int
  /// Tab items
  pub items : @list.List[TabBarItem]
  /// Callback when tab changes
  on_change : (Int) -> Unit
}

pub fn TabBar::new(items : @list.List[TabBarItem]) -> TabBar {
  {
    selected_index: 0,
    items: items,
    on_change: fn(_) { () }
  }
}

pub fn TabBar::with_on_change(self : TabBar, callback : (Int) -> Unit) -> TabBar {
  { ..self, on_change: callback }
}

///| Select a tab by index
pub fn TabBar::select(self : TabBar, index : Int) -> Unit {
  if index >= 0 && index < @list.length(self.items) {
    self.on_change(index)
  }
}
