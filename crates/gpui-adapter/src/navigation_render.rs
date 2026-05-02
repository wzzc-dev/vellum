//! Navigation rendering module for GPUI
//! 
//! This module provides GPUI integration for navigation functionality,
//! including NavigationStack, TabBar, and Sheet/Modal support.

use crate::widget::{Widget, WidgetId, WidgetManager};
use crate::types::*;
use std::sync::{Arc, RwLock};

/// Navigation state for tracking current route and history
#[derive(Debug, Clone)]
pub struct NavigationState {
    pub current_route: String,
    pub params: Vec<(String, String)>,
    pub can_go_back: bool,
    pub stack_depth: u32,
    pub history: Vec<String>,
}

impl NavigationState {
    pub fn new(initial_route: &str) -> Self {
        Self {
            current_route: initial_route.to_string(),
            params: Vec::new(),
            can_go_back: false,
            stack_depth: 1,
            history: vec![initial_route.to_string()],
        }
    }
}

/// Navigator controller for programmatic navigation
pub struct NavigatorController {
    pub state: Arc<RwLock<NavigationState>>,
    pub routes: Arc<RwLock<Vec<RouteConfig>>>,
    pub listeners: Arc<RwLock<Vec<Box<dyn Fn(NavigationEvent) + Send + Sync>>>>,
}

impl NavigatorController {
    pub fn new(initial_route: &str) -> Self {
        Self {
            state: Arc::new(RwLock::new(NavigationState::new(initial_route))),
            routes: Arc::new(RwLock::new(Vec::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn push(&self, route: &str, params: Option<Vec<(String, String)>>) -> bool {
        let mut state = self.state.write().unwrap();
        
        // Check if route exists
        let routes = self.routes.read().unwrap();
        if !routes.iter().any(|r| r.path == route) {
            return false;
        }
        
        state.current_route = route.to_string();
        state.params = params.unwrap_or_default();
        state.history.push(route.to_string());
        state.stack_depth = state.history.len() as u32;
        state.can_go_back = state.history.len() > 1;
        
        self.notify_listeners(NavigationEvent::Pushed {
            route: route.to_string(),
            params: state.params.clone(),
        });
        
        true
    }

    pub fn pop(&self) -> bool {
        let mut state = self.state.write().unwrap();
        
        if state.history.len() <= 1 {
            return false;
        }
        
        let popped_route = state.current_route.clone();
        state.history.pop();
        state.current_route = state.history.last().cloned().unwrap_or_default();
        state.stack_depth = state.history.len() as u32;
        state.can_go_back = state.history.len() > 1;
        state.params = Vec::new();
        
        self.notify_listeners(NavigationEvent::Popped {
            route: state.current_route.clone(),
            popped_route,
        });
        
        true
    }

    pub fn replace(&self, route: &str, params: Option<Vec<(String, String)>>) -> bool {
        let mut state = self.state.write().unwrap();
        
        let old_route = state.current_route.clone();
        state.current_route = route.to_string();
        state.params = params.unwrap_or_default();
        
        // Replace last entry in history
        if let Some(last) = state.history.last_mut() {
            *last = route.to_string();
        }
        
        state.can_go_back = state.history.len() > 1;
        
        self.notify_listeners(NavigationEvent::Replaced {
            old_route,
            new_route: route.to_string(),
        });
        
        true
    }

    pub fn can_go_back(&self) -> bool {
        self.state.read().unwrap().can_go_back
    }

    pub fn get_state(&self) -> NavigationState {
        self.state.read().unwrap().clone()
    }

    pub fn clear_history(&self) {
        let mut state = self.state.write().unwrap();
        let root = state.history.first().cloned().unwrap_or_default();
        state.history = vec![root.clone()];
        state.current_route = root;
        state.can_go_back = false;
        state.stack_depth = 1;
        state.params = Vec::new();
        
        self.notify_listeners(NavigationEvent::StateChanged(state.clone()));
    }

    pub fn set_root(&self, route: &str) {
        let mut state = self.state.write().unwrap();
        state.history = vec![route.to_string()];
        state.current_route = route.to_string();
        state.can_go_back = false;
        state.stack_depth = 1;
        state.params = Vec::new();
        
        self.notify_listeners(NavigationEvent::StateChanged(state.clone()));
    }

    pub fn register_route(&self, config: RouteConfig) {
        self.routes.write().unwrap().push(config);
    }

    pub fn unregister_route(&self, path: &str) {
        self.routes.write().unwrap().retain(|r| r.path != path);
    }

    pub fn add_listener<F>(&self, callback: F)
    where
        F: Fn(NavigationEvent) + Send + Sync + 'static,
    {
        self.listeners.write().unwrap().push(Box::new(callback));
    }

    fn notify_listeners(&self, event: NavigationEvent) {
        let listeners = self.listeners.read().unwrap();
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }
}

/// Navigation event types
#[derive(Debug, Clone)]
pub enum NavigationEvent {
    Pushed { route: String, params: Vec<(String, String)> },
    Popped { route: String, popped_route: String },
    Replaced { old_route: String, new_route: String },
    StateChanged(NavigationState),
    DeepLink { url: String },
}

/// Route configuration
#[derive(Debug, Clone)]
pub struct RouteConfig {
    pub path: String,
    pub name: Option<String>,
    pub requires_auth: bool,
}

/// Transition types
#[derive(Debug, Clone, Copy)]
pub enum TransitionType {
    PushRight,
    PushBottom,
    PushLeft,
    PushTop,
    Fade,
    None,
}

impl Default for TransitionType {
    fn default() -> Self {
        TransitionType::PushRight
    }
}

/// Navigation controller resource
pub struct NavigationControllerResource {
    pub config: NavHostConfig,
    pub navigator: Arc<NavigatorController>,
    pub routes: Arc<RwLock<Vec<RouteConfig>>>,
}

impl NavigationControllerResource {
    pub fn new(config: NavHostConfig) -> Self {
        Self {
            navigator: Arc::new(NavigatorController::new(&config.initial_route)),
            config,
            routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn register_route(&self, route: RouteConfig) {
        self.routes.write().unwrap().push(route.clone());
        self.navigator.register_route(route);
    }

    pub fn unregister_route(&self, path: &str) {
        self.routes.write().unwrap().retain(|r| r.path != path);
        self.navigator.unregister_route(path);
    }

    pub fn handle_deep_link(&self, url: &str) -> bool {
        // Simple deep link parsing: extract route from URL
        // Format: scheme://host/path?query
        if let Some(path) = url.strip_prefix("vellum://") {
            let route = path.split('?').next().unwrap_or(path);
            if !route.is_empty() {
                self.navigator.push(route, None);
                return true;
            }
        }
        false
    }
}

/// Tab bar item
#[derive(Debug, Clone)]
pub struct TabBarItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub enabled: bool,
}

impl TabBarItem {
    pub fn new(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            icon: None,
            enabled: true,
        }
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Tab bar state
pub struct TabBar {
    pub items: Vec<TabBarItem>,
    pub selected_index: usize,
    pub on_change: Option<Box<dyn Fn(usize) + Send + Sync>>,
}

impl TabBar {
    pub fn new(items: Vec<TabBarItem>) -> Self {
        Self {
            items,
            selected_index: 0,
            on_change: None,
        }
    }

    pub fn with_on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn select(&mut self, index: usize) {
        if index < self.items.len() && self.items[index].enabled {
            self.selected_index = index;
            if let Some(ref callback) = self.on_change {
                callback(index);
            }
        }
    }
}

/// NavHost widget that manages navigation state
pub struct NavHostWidget {
    pub navigator: Arc<NavigatorController>,
    pub routes: Vec<RouteConfig>,
    pub default_transition: TransitionType,
    pub show_back_button: bool,
}

impl NavHostWidget {
    pub fn new(config: NavHostConfig) -> Self {
        let navigator = NavigatorController::new(&config.initial_route);
        
        // Register all routes
        for route in &config.routes {
            navigator.register_route(route);
        }
        
        Self {
            navigator: Arc::new(navigator),
            routes: config.routes,
            default_transition: TransitionType::PushRight,
            show_back_button: config.show_back_button,
        }
    }
}

/// Sheet/Modal presentation configuration
pub struct SheetConfig {
    pub content: WidgetId,
    pub dismissible: bool,
    pub show_handle: bool,
    pub detents: Vec<SheetDetent>,
}

pub enum SheetDetent {
    Small,
    Medium,
    Large,
    Custom(f64),
}

impl Default for SheetConfig {
    fn default() -> Self {
        Self {
            content: String::new(),
            dismissible: true,
            show_handle: true,
            detents: vec![SheetDetent::Medium],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_push_pop() {
        let nav = NavigatorController::new("home");
        nav.register_route(RouteConfig {
            path: "detail".to_string(),
            name: Some("Detail".to_string()),
            requires_auth: false,
        });
        
        assert!(nav.can_go_back() == false);
        
        nav.push("detail", None);
        assert!(nav.can_go_back() == true);
        
        let state = nav.get_state();
        assert_eq!(state.current_route, "detail");
        assert_eq!(state.stack_depth, 2);
        
        nav.pop();
        assert!(nav.can_go_back() == false);
    }

    #[test]
    fn test_navigation_replace() {
        let nav = NavigatorController::new("home");
        nav.register_route(RouteConfig {
            path: "settings".to_string(),
            name: None,
            requires_auth: false,
        });
        
        nav.push("detail", None);
        assert_eq!(nav.get_state().stack_depth, 2);
        
        nav.replace("settings", None);
        assert_eq!(nav.get_state().stack_depth, 2); // Same depth
        assert_eq!(nav.get_state().current_route, "settings");
    }

    #[test]
    fn test_tab_bar() {
        let items = vec![
            TabBarItem::new("home", "Home"),
            TabBarItem::new("profile", "Profile").disabled(),
            TabBarItem::new("settings", "Settings"),
        ];
        
        let mut tab_bar = TabBar::new(items);
        assert_eq!(tab_bar.selected_index, 0);
        
        tab_bar.select(2);
        assert_eq!(tab_bar.selected_index, 2);
        
        tab_bar.select(1); // Should not select disabled tab
        assert_eq!(tab_bar.selected_index, 2);
    }
}
