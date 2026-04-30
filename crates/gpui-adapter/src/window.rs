use crate::error::{AdapterError, Result};
use crate::types::{Point, Size};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub resizable: bool,
    pub decorated: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub center: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "Vellum".to_string(),
            width: 1024,
            height: 768,
            min_width: 400,
            min_height: 300,
            max_width: 0,
            max_height: 0,
            resizable: true,
            decorated: true,
            transparent: false,
            always_on_top: false,
            center: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Window {
    pub id: WindowId,
    pub options: WindowOptions,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub is_visible: bool,
    pub is_maximized: bool,
    pub is_fullscreen: bool,
    pub is_minimized: bool,
}

pub type WindowId = u32;

impl Window {
    pub fn new(id: WindowId, options: WindowOptions) -> Self {
        let width = options.width;
        let height = options.height;

        Self {
            id,
            options,
            position: (100, 100),
            size: (width, height),
            is_visible: true,
            is_maximized: false,
            is_fullscreen: false,
            is_minimized: false,
        }
    }

    pub fn get_size(&self) -> Size {
        Size::new(self.size.0 as f32, self.size.1 as f32)
    }

    pub fn get_position(&self) -> Point {
        Point::new(self.position.0 as f32, self.position.1 as f32)
    }
}

#[derive(Default)]
pub struct WindowManager {
    windows: HashMap<WindowId, Window>,
    active_window: Option<WindowId>,
    next_id: WindowId,
}

impl WindowManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_window(&mut self, options: WindowOptions) -> WindowId {
        let id = self.next_id;
        self.next_id += 1;

        let window = Window::new(id, options);
        self.windows.insert(id, window);

        if self.active_window.is_none() {
            self.active_window = Some(id);
        }

        id
    }

    pub fn destroy_window(&mut self, id: WindowId) -> Result<()> {
        if self.windows.remove(&id).is_none() {
            return Err(AdapterError::WindowNotFound(id));
        }

        if self.active_window == Some(id) {
            self.active_window = self.windows.keys().next().copied();
        }

        Ok(())
    }

    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn get_all_windows(&self) -> impl Iterator<Item = &Window> {
        self.windows.values()
    }

    pub fn get_all_window_ids(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    pub fn set_active_window(&mut self, id: WindowId) -> Result<()> {
        if !self.windows.contains_key(&id) {
            return Err(AdapterError::WindowNotFound(id));
        }
        self.active_window = Some(id);
        Ok(())
    }

    pub fn get_active_window(&self) -> Option<WindowId> {
        self.active_window
    }

    pub fn set_title(&mut self, id: WindowId, title: String) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.options.title = title;
        Ok(())
    }

    pub fn get_title(&self, id: WindowId) -> Result<String> {
        self.windows
            .get(&id)
            .map(|w| w.options.title.clone())
            .ok_or_else(|| AdapterError::WindowNotFound(id))
    }

    pub fn set_size(&mut self, id: WindowId, width: u32, height: u32) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;

        // Apply constraints
        let width = width.clamp(window.options.min_width, window.options.max_width.max(1));
        let height = height.clamp(window.options.min_height, window.options.max_height.max(1));

        window.size = (width, height);
        Ok(())
    }

    pub fn set_position(&mut self, id: WindowId, x: i32, y: i32) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.position = (x, y);
        Ok(())
    }

    pub fn minimize(&mut self, id: WindowId) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_minimized = true;
        Ok(())
    }

    pub fn maximize(&mut self, id: WindowId) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_maximized = true;
        Ok(())
    }

    pub fn unmaximize(&mut self, id: WindowId) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_maximized = false;
        Ok(())
    }

    pub fn is_maximized(&self, id: WindowId) -> Result<bool> {
        self.windows
            .get(&id)
            .map(|w| w.is_maximized)
            .ok_or_else(|| AdapterError::WindowNotFound(id))
    }

    pub fn restore(&mut self, id: WindowId) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_minimized = false;
        window.is_maximized = false;
        Ok(())
    }

    pub fn show(&mut self, id: WindowId) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_visible = true;
        Ok(())
    }

    pub fn hide(&mut self, id: WindowId) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_visible = false;
        Ok(())
    }

    pub fn is_visible(&self, id: WindowId) -> Result<bool> {
        self.windows
            .get(&id)
            .map(|w| w.is_visible)
            .ok_or_else(|| AdapterError::WindowNotFound(id))
    }

    pub fn set_fullscreen(&mut self, id: WindowId, fullscreen: bool) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.is_fullscreen = fullscreen;
        Ok(())
    }

    pub fn is_fullscreen(&self, id: WindowId) -> Result<bool> {
        self.windows
            .get(&id)
            .map(|w| w.is_fullscreen)
            .ok_or_else(|| AdapterError::WindowNotFound(id))
    }

    pub fn set_always_on_top(&mut self, id: WindowId, always_on_top: bool) -> Result<()> {
        let window = self
            .windows
            .get_mut(&id)
            .ok_or_else(|| AdapterError::WindowNotFound(id))?;
        window.options.always_on_top = always_on_top;
        Ok(())
    }

    pub fn get_size(&self, id: WindowId) -> Result<Size> {
        self.windows
            .get(&id)
            .map(|w| w.get_size())
            .ok_or_else(|| AdapterError::WindowNotFound(id))
    }

    pub fn get_position(&self, id: WindowId) -> Result<Point> {
        self.windows
            .get(&id)
            .map(|w| w.get_position())
            .ok_or_else(|| AdapterError::WindowNotFound(id))
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }
}
