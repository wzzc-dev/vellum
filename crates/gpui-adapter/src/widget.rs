use crate::error::{AdapterError, Result};
use crate::types::{
    Alignment, BoxShadow, Color, CrossAlignment, EdgeInsets, FlexDirection, FlexParams, FontStyle,
    FontWeight, Rect, Size, TextDecoration, TextStyle, Visibility, WidgetDisplay, WidgetLayout,
    WidgetPosition, Wrap,
};
use crate::EventDispatcher;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub type WidgetId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: WidgetId,
    pub widget_type: String,
    pub parent_id: Option<WidgetId>,
    pub children: Vec<WidgetId>,
    pub layout: WidgetLayout,
    pub size: Size,
    pub position: (f32, f32),
    pub padding: EdgeInsets,
    pub margin: EdgeInsets,
    pub background: Color,
    pub opacity: f32,
    pub visibility: Visibility,
    pub z_index: i32,
    pub needs_layout: bool,
    pub needs_paint: bool,
    pub clip: bool,
    pub clip_bounds: Rect,
    pub border_radius: f32,
    pub shadow: Option<BoxShadow>,
}

impl Widget {
    pub fn new(widget_type: impl Into<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            widget_type: widget_type.into(),
            parent_id: None,
            children: Vec::new(),
            layout: WidgetLayout::default(),
            size: Size::default(),
            position: (0.0, 0.0),
            padding: EdgeInsets::default(),
            margin: EdgeInsets::default(),
            background: Color::transparent(),
            opacity: 1.0,
            visibility: Visibility::Visible,
            z_index: 0,
            needs_layout: true,
            needs_paint: true,
            clip: false,
            clip_bounds: Rect::default(),
            border_radius: 0.0,
            shadow: None,
        }
    }

    pub fn with_id(mut self, id: WidgetId) -> Self {
        self.id = id;
        self
    }

    pub fn is_visible(&self) -> bool {
        self.visibility == Visibility::Visible
    }

    pub fn mark_needs_layout(&mut self) {
        self.needs_layout = true;
        for child_id in &self.children {
            // Children will be marked in WidgetManager
        }
    }

    pub fn mark_needs_paint(&mut self) {
        self.needs_paint = true;
    }

    pub fn global_bounds(&self, parent_bounds: Option<&Rect>) -> Rect {
        let (x, y) = self.position;
        match self.layout.position {
            WidgetPosition::Absolute => Rect::new(x, y, self.size.width, self.size.height),
            _ => {
                if let Some(parent) = parent_bounds {
                    Rect::new(
                        parent.x + x,
                        parent.y + y,
                        self.size.width,
                        self.size.height,
                    )
                } else {
                    Rect::new(x, y, self.size.width, self.size.height)
                }
            }
        }
    }
}

#[derive(Default)]
pub struct WidgetManager {
    widgets: HashMap<WidgetId, Widget>,
    root_id: Option<WidgetId>,
    dirty_widgets: Vec<WidgetId>,
}

impl WidgetManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_widget(&mut self, widget_type: impl Into<String>) -> WidgetId {
        let widget = Widget::new(widget_type);
        let id = widget.id.clone();
        self.widgets.insert(id.clone(), widget);
        self.dirty_widgets.push(id.clone());
        id
    }

    pub fn destroy_widget(&mut self, id: &WidgetId) -> Result<()> {
        if let Some(mut widget) = self.widgets.remove(id) {
            // Remove from parent's children
            if let Some(parent_id) = widget.parent_id.take() {
                if let Some(parent) = self.widgets.get_mut(&parent_id) {
                    parent.children.retain(|c| c != id);
                }
            }

            // Destroy children recursively
            let children: Vec<WidgetId> = widget.children.clone();
            for child_id in children {
                let _ = self.destroy_widget(&child_id);
            }

            Ok(())
        } else {
            Err(AdapterError::WidgetNotFound(id.clone()))
        }
    }

    pub fn get_widget(&self, id: &WidgetId) -> Option<&Widget> {
        self.widgets.get(id)
    }

    pub fn get_widget_mut(&mut self, id: &WidgetId) -> Option<&mut Widget> {
        self.widgets.get_mut(id)
    }

    pub fn mount_widget(&mut self, id: &WidgetId, parent_id: &WidgetId) -> Result<()> {
        // First, check both widgets exist
        if !self.widgets.contains_key(id) {
            return Err(AdapterError::WidgetNotFound(id.clone()));
        }
        if !self.widgets.contains_key(parent_id) {
            return Err(AdapterError::WidgetNotFound(parent_id.clone()));
        }

        // Get the parent_id to store before borrowing
        let parent_id_clone = parent_id.clone();
        let id_clone = id.clone();

        // Now mutate both
        {
            let widget = self.widgets.get_mut(id).unwrap();
            widget.parent_id = Some(parent_id_clone.clone());
        }

        {
            let parent = self.widgets.get_mut(&parent_id_clone).unwrap();
            parent.children.push(id_clone);
        }

        // Mark needs layout
        if let Some(widget) = self.widgets.get_mut(id) {
            widget.mark_needs_layout();
        }
        self.schedule_layout(parent_id);

        Ok(())
    }

    pub fn unmount_widget(&mut self, id: &WidgetId) -> Result<()> {
        let parent_id_opt = {
            let widget = self
                .widgets
                .get(id)
                .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;
            widget.parent_id.clone()
        };

        if let Some(parent_id) = parent_id_opt {
            if let Some(parent) = self.widgets.get_mut(&parent_id) {
                parent.children.retain(|c| c != id);
            }
            // Clear parent_id
            if let Some(widget) = self.widgets.get_mut(id) {
                widget.parent_id = None;
            }
            self.schedule_layout(&parent_id);
        }

        Ok(())
    }

    pub fn get_children(&self, id: &WidgetId) -> Option<&Vec<WidgetId>> {
        self.widgets.get(id).map(|w| &w.children)
    }

    pub fn get_parent(&self, id: &WidgetId) -> Option<&Option<WidgetId>> {
        self.widgets.get(id).map(|w| &w.parent_id)
    }

    pub fn get_all_widgets(&self) -> impl Iterator<Item = &Widget> {
        self.widgets.values()
    }

    pub fn set_layout(&mut self, id: &WidgetId, layout: WidgetLayout) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.layout = layout;
        widget.mark_needs_layout();
        self.schedule_layout(id);
        Ok(())
    }

    pub fn get_layout(&self, id: &WidgetId) -> Result<WidgetLayout> {
        self.widgets
            .get(id)
            .map(|w| w.layout.clone())
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))
    }

    pub fn set_size(&mut self, id: &WidgetId, width: f32, height: f32) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.size = Size::new(width, height);
        widget.mark_needs_layout();
        self.schedule_layout(id);
        Ok(())
    }

    pub fn get_size(&self, id: &WidgetId) -> Result<Size> {
        self.widgets
            .get(id)
            .map(|w| w.size)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))
    }

    pub fn set_position(&mut self, id: &WidgetId, x: f32, y: f32) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.position = (x, y);
        widget.mark_needs_paint();
        Ok(())
    }

    pub fn get_position(&self, id: &WidgetId) -> Result<(f32, f32)> {
        self.widgets
            .get(id)
            .map(|w| w.position)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))
    }

    pub fn set_padding(&mut self, id: &WidgetId, insets: EdgeInsets) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.padding = insets;
        widget.mark_needs_layout();
        self.schedule_layout(id);
        Ok(())
    }

    pub fn set_margin(&mut self, id: &WidgetId, insets: EdgeInsets) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.margin = insets;
        widget.mark_needs_layout();
        self.schedule_layout(id);
        Ok(())
    }

    pub fn set_background(&mut self, id: &WidgetId, color: Color) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.background = color;
        widget.mark_needs_paint();
        Ok(())
    }

    pub fn set_opacity(&mut self, id: &WidgetId, opacity: f32) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.opacity = opacity;
        widget.mark_needs_paint();
        Ok(())
    }

    pub fn set_visibility(&mut self, id: &WidgetId, visibility: Visibility) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.visibility = visibility;
        widget.mark_needs_layout();
        self.schedule_layout(id);
        Ok(())
    }

    pub fn get_bounds(&self, id: &WidgetId) -> Result<Rect> {
        let widget = self
            .widgets
            .get(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        Ok(Rect::new(
            widget.position.0,
            widget.position.1,
            widget.size.width,
            widget.size.height,
        ))
    }

    pub fn get_global_bounds(&self, id: &WidgetId) -> Result<Rect> {
        let widget = self
            .widgets
            .get(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        let mut bounds = self.get_bounds(id)?;

        let mut current_parent = widget.parent_id.clone();
        while let Some(parent_id) = current_parent {
            if let Some(parent) = self.widgets.get(&parent_id) {
                bounds.x += parent.position.0;
                bounds.y += parent.position.1;
                current_parent = parent.parent_id.clone();
            } else {
                break;
            }
        }

        Ok(bounds)
    }

    pub fn set_clip(&mut self, id: &WidgetId, clip: bool, bounds: Rect) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.clip = clip;
        widget.clip_bounds = bounds;
        widget.mark_needs_paint();
        Ok(())
    }

    pub fn set_shadow(&mut self, id: &WidgetId, shadow: Option<BoxShadow>) -> Result<()> {
        let widget = self
            .widgets
            .get_mut(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        widget.shadow = shadow;
        widget.mark_needs_paint();
        Ok(())
    }

    fn schedule_layout(&mut self, id: &WidgetId) {
        if !self.dirty_widgets.contains(id) {
            self.dirty_widgets.push(id.clone());
        }
    }

    pub fn get_dirty_widgets(&mut self) -> Vec<WidgetId> {
        std::mem::take(&mut self.dirty_widgets)
    }

    pub fn clear_dirty(&mut self, id: &WidgetId) {
        if let Some(widget) = self.widgets.get_mut(id) {
            widget.needs_layout = false;
            widget.needs_paint = false;
        }
    }

    pub fn calculate_layout(&mut self, id: &WidgetId, parent_size: Size) -> Result<()> {
        let widget = self
            .widgets
            .get(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        match widget.layout.display {
            WidgetDisplay::None => return Ok(()),
            _ => {}
        }

        let layout = &widget.layout;

        // Calculate flex layout for container widgets
        match widget.widget_type.as_str() {
            "column" | "row" | "stack" | "container" => {
                self.calculate_flex_layout(id, parent_size)?;
            }
            _ => {
                // For leaf widgets, just use the available space
                if widget.size.width == 0.0 || widget.size.height == 0.0 {
                    if let Some(widget) = self.widgets.get_mut(id) {
                        widget.size = parent_size;
                    }
                }
            }
        }

        Ok(())
    }

    fn calculate_flex_layout(&mut self, id: &WidgetId, parent_size: Size) -> Result<()> {
        let widget = self
            .widgets
            .get(id)
            .ok_or_else(|| AdapterError::WidgetNotFound(id.clone()))?;

        let children: Vec<WidgetId> = widget.children.clone();
        let layout = widget.layout.clone();

        let is_column = matches!(
            layout.flex_direction,
            FlexDirection::Column | FlexDirection::ColumnReverse
        );

        let mut offset_main = 0.0f32;
        let main_size = if is_column {
            parent_size.height
        } else {
            parent_size.width
        };

        for child_id in &children {
            if let Some(child) = self.widgets.get_mut(child_id) {
                let child_flex = FlexParams::default();

                let child_main_size = if is_column {
                    child.size.height
                } else {
                    child.size.width
                };

                let actual_main_size = if child_main_size == 0.0 {
                    100.0 // Default size
                } else {
                    child_main_size
                };

                if is_column {
                    child.position = (layout.gap, offset_main);
                    child.size.width = parent_size.width - layout.gap * 2.0;
                } else {
                    child.position = (offset_main, layout.gap);
                    child.size.height = parent_size.height - layout.gap * 2.0;
                }

                offset_main += actual_main_size + layout.gap;

                // Calculate child layout recursively
                let child_size = child.size;
                drop(child);
                let _ = self.calculate_layout(child_id, child_size);
            }
        }

        Ok(())
    }
}

pub struct WidgetState {
    pub widget_id: String,
    pub event_dispatcher: EventDispatcher,
    pub widget_manager: Arc<RwLock<WidgetManager>>,
}

impl WidgetState {
    pub fn new(widget_id: String) -> Self {
        Self {
            widget_id,
            event_dispatcher: EventDispatcher::new(),
            widget_manager: Arc::new(RwLock::new(WidgetManager::new())),
        }
    }
}
