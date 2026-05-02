//! Grid rendering module for GPUI
//! 
//! This module provides GPUI integration for grid layout functionality,
//! including LazyVGrid, LazyHGrid, and responsive grid support.

use crate::types::*;
use std::collections::HashMap;

/// Grid alignment options
#[derive(Debug, Clone, Copy)]
pub enum GridAlignment {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Grid track sizing
#[derive(Debug, Clone)]
pub enum GridTrackSize {
    Fixed(f32),
    Fill { min_size: Option<f32>, max_size: Option<f32> },
    Auto,
    MinMax { min: f32, max: f32 },
}

/// Grid line placement
#[derive(Debug, Clone)]
pub struct GridLinePlacement {
    pub start: u32,
    pub end: Option<u32>,
    pub start_name: Option<String>,
    pub end_name: Option<String>,
}

impl Default for GridLinePlacement {
    fn default() -> Self {
        Self {
            start: 0,
            end: None,
            start_name: None,
            end_name: None,
        }
    }
}

/// Column span configuration
#[derive(Debug, Clone)]
pub struct ColumnSpan {
    pub span: u32,
    pub alignment: Option<GridAlignment>,
}

impl ColumnSpan {
    pub fn new(span: u32) -> Self {
        Self { span, alignment: None }
    }
}

/// Row span configuration
#[derive(Debug, Clone)]
pub struct RowSpan {
    pub span: u32,
    pub alignment: Option<GridAlignment>,
}

impl RowSpan {
    pub fn new(span: u32) -> Self {
        Self { span, alignment: None }
    }
}

/// Grid item placement
#[derive(Debug, Clone)]
pub struct GridItemPlacement {
    pub row: GridLinePlacement,
    pub column: GridLinePlacement,
    pub row_span: Option<RowSpan>,
    pub column_span: Option<ColumnSpan>,
}

impl Default for GridItemPlacement {
    fn default() -> Self {
        Self {
            row: GridLinePlacement::default(),
            column: GridLinePlacement::default(),
            row_span: None,
            column_span: None,
        }
    }
}

/// Grid item data
#[derive(Debug, Clone)]
pub struct GridItemData {
    pub widget_id: String,
    pub placement: GridItemPlacement,
    pub justify_self: Option<GridAlignment>,
    pub align_self: Option<GridAlignment>,
}

impl GridItemData {
    pub fn new(widget_id: &str) -> Self {
        Self {
            widget_id: widget_id.to_string(),
            placement: GridItemPlacement::default(),
            justify_self: None,
            align_self: None,
        }
    }

    pub fn with_column_span(mut self, span: u32) -> Self {
        self.placement.column_span = Some(ColumnSpan::new(span));
        self
    }

    pub fn with_row_span(mut self, span: u32) -> Self {
        self.placement.row_span = Some(RowSpan::new(span));
        self
    }
}

/// Grid configuration
#[derive(Debug, Clone)]
pub struct GridConfig {
    pub columns: u32,
    pub rows: u32,
    pub column_gap: f32,
    pub row_gap: f32,
    pub column_sizes: Option<Vec<GridTrackSize>>,
    pub row_sizes: Option<Vec<GridTrackSize>>,
    pub justify_items: GridAlignment,
    pub align_items: GridAlignment,
    pub justify_content: GridAlignment,
    pub align_content: GridAlignment,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: 2,
            rows: 0,
            column_gap: 8.0,
            row_gap: 8.0,
            column_sizes: None,
            row_sizes: None,
            justify_items: GridAlignment::Stretch,
            align_items: GridAlignment::Stretch,
            justify_content: GridAlignment::Start,
            align_content: GridAlignment::Start,
            max_width: None,
            max_height: None,
        }
    }
}

impl GridConfig {
    pub fn new(columns: u32) -> Self {
        Self {
            columns,
            ..Default::default()
        }
    }

    pub fn with_gaps(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }

    pub fn with_alignment(mut self, justify: GridAlignment, align: GridAlignment) -> Self {
        self.justify_items = justify;
        self.align_items = align;
        self
    }
}

/// Grid view controller
pub struct GridView {
    config: GridConfig,
    items: Vec<GridItemData>,
}

impl GridView {
    pub fn new(config: GridConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
        }
    }

    pub fn add_item(&mut self, widget_id: &str) {
        self.items.push(GridItemData::new(widget_id));
    }

    pub fn add_item_with_placement(&mut self, item: GridItemData) {
        self.items.push(item);
    }

    pub fn add_items(&mut self, widget_ids: &[String]) {
        for id in widget_ids {
            self.add_item(id);
        }
    }

    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }

    pub fn remove_item_by_id(&mut self, widget_id: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|i| i.widget_id == widget_id) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn update_placement(&mut self, widget_id: &str, placement: GridItemPlacement) -> bool {
        if let Some(item) = self.items.iter_mut().find(|i| i.widget_id == widget_id) {
            item.placement = placement;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn get_item(&self, index: usize) -> Option<&GridItemData> {
        self.items.get(index)
    }

    pub fn get_item_by_id(&self, widget_id: &str) -> Option<&GridItemData> {
        self.items.iter().find(|i| i.widget_id == widget_id)
    }

    pub fn set_columns(&mut self, count: u32) {
        self.config.columns = count;
    }

    pub fn set_gaps(&mut self, column_gap: f32, row_gap: f32) {
        self.config.column_gap = column_gap;
        self.config.row_gap = row_gap;
    }

    /// Calculate item position in grid
    pub fn calculate_position(&self, index: usize) -> (u32, u32) {
        let row = (index / self.config.columns as usize) as u32;
        let col = (index % self.config.columns as usize) as u32;
        (row, col)
    }

    /// Calculate item bounds
    pub fn calculate_bounds(&self, index: usize) -> GridBounds {
        let (row, col) = self.calculate_position(index);
        let item = self.get_item(index);
        
        let col_span = item
            .and_then(|i| i.placement.column_span.as_ref())
            .map(|s| s.span)
            .unwrap_or(1);
        
        let row_span = item
            .and_then(|i| i.placement.row_span.as_ref())
            .map(|s| s.span)
            .unwrap_or(1);
        
        GridBounds {
            row_start: row,
            row_end: row + row_span,
            column_start: col,
            column_end: col + col_span,
        }
    }
}

/// Grid bounds for an item
#[derive(Debug, Clone)]
pub struct GridBounds {
    pub row_start: u32,
    pub row_end: u32,
    pub column_start: u32,
    pub column_end: u32,
}

/// Lazy grid configuration
#[derive(Debug, Clone)]
pub struct LazyGridConfig {
    pub base: GridConfig,
    pub cache_extent: Option<f32>,
    pub estimated_item_height: f32,
    pub reverse: bool,
}

impl Default for LazyGridConfig {
    fn default() -> Self {
        Self {
            base: GridConfig::default(),
            cache_extent: None,
            estimated_item_height: 50.0,
            reverse: false,
        }
    }
}

/// Lazy grid controller
pub struct LazyGridView {
    config: LazyGridConfig,
    items: Vec<GridItemData>,
    visible_range: Option<(u32, u32)>,
}

impl LazyGridView {
    pub fn new(config: LazyGridConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
            visible_range: None,
        }
    }

    pub fn append_item(&mut self, widget_id: &str) {
        if self.config.reverse {
            self.items.insert(0, GridItemData::new(widget_id));
        } else {
            self.items.push(GridItemData::new(widget_id));
        }
    }

    pub fn prepend_item(&mut self, widget_id: &str) {
        if self.config.reverse {
            self.items.push(GridItemData::new(widget_id));
        } else {
            self.items.insert(0, GridItemData::new(widget_id));
        }
    }

    pub fn insert_item(&mut self, index: usize, widget_id: &str) {
        if index >= self.items.len() {
            self.items.push(GridItemData::new(widget_id));
        } else {
            self.items.insert(index, GridItemData::new(widget_id));
        }
    }

    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }

    pub fn move_item(&mut self, from_index: usize, to_index: usize) {
        if from_index < self.items.len() && to_index <= self.items.len() {
            let item = self.items.remove(from_index);
            let insert_at = if to_index > from_index { to_index - 1 } else { to_index };
            self.items.insert(insert_at, item);
        }
    }

    pub fn update_item(&mut self, index: usize, widget_id: &str) {
        if index < self.items.len() {
            self.items[index].widget_id = widget_id.to_string();
        }
    }

    pub fn set_item_count(&mut self, count: u32) {
        // Adjust items list to match count
        while self.items.len() < count as usize {
            self.items.push(GridItemData::new(""));
        }
        while self.items.len() > count as usize {
            self.items.pop();
        }
    }

    pub fn get_visible_range(&self) -> Option<(u32, u32)> {
        self.visible_range
    }

    pub fn set_visible_range(&mut self, start: u32, end: u32) {
        self.visible_range = Some((start, end));
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn invalidate_all(&mut self) {
        // Force rebuild all items
        self.visible_range = None;
    }

    pub fn invalidate_item(&mut self, _index: usize) {
        // Force rebuild specific item
        // Implementation would depend on caching strategy
    }

    pub fn calculate_visible_items(&self, viewport_height: f32, scroll_offset: f32) -> (u32, u32) {
        let item_height = self.config.estimated_item_height;
        let rows_visible = ((viewport_height / item_height).ceil() as u32) + 2; // Buffer rows
        
        let first_visible_row = ((scroll_offset / item_height).floor() as u32).saturating_sub(1);
        let last_visible_row = first_visible_row + rows_visible;
        
        (first_visible_row, last_visible_row.min(self.items.len() as u32))
    }
}

/// Grid event types
#[derive(Debug, Clone)]
pub enum GridEvent {
    ItemTapped { index: u32, widget_id: String },
    ItemLongPressed { index: u32, widget_id: String },
    VisibleRangeChanged { start: u32, end: u32 },
    ScrollPositionChanged { offset: f32 },
    ItemRemoved { index: u32, widget_id: String },
    ItemInserted { index: u32, widget_id: String },
}

/// Grid scroll behavior
#[derive(Debug, Clone, Copy)]
pub enum GridScrollBehavior {
    KeepSelectionVisible,
    ScrollToTop,
    ScrollToBottom,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_config() {
        let config = GridConfig::new(3);
        assert_eq!(config.columns, 3);
        
        let config = config.with_gaps(16.0, 16.0);
        assert_eq!(config.column_gap, 16.0);
        assert_eq!(config.row_gap, 16.0);
    }

    #[test]
    fn test_grid_position_calculation() {
        let mut grid = GridView::new(GridConfig::new(3));
        
        grid.add_item("item0");
        grid.add_item("item1");
        grid.add_item("item2");
        grid.add_item("item3");
        
        assert_eq!(grid.calculate_position(0), (0, 0));
        assert_eq!(grid.calculate_position(1), (0, 1));
        assert_eq!(grid.calculate_position(2), (0, 2));
        assert_eq!(grid.calculate_position(3), (1, 0));
    }

    #[test]
    fn test_grid_span() {
        let mut grid = GridView::new(GridConfig::new(3));
        
        grid.add_item_with_placement(
            GridItemData::new("item0").with_column_span(2)
        );
        grid.add_item("item1");
        
        let bounds = grid.calculate_bounds(0);
        assert_eq!(bounds.column_start, 0);
        assert_eq!(bounds.column_end, 2); // Span of 2
        assert_eq!(bounds.row_start, 0);
        assert_eq!(bounds.row_end, 1);
        
        // After span of 2, next item should be at column 2
        let bounds = grid.calculate_bounds(1);
        assert_eq!(bounds.column_start, 2);
    }

    #[test]
    fn test_lazy_grid() {
        let config = LazyGridConfig {
            base: GridConfig::new(2),
            cache_extent: Some(200.0),
            estimated_item_height: 50.0,
            reverse: false,
        };
        
        let mut grid = LazyGridView::new(config);
        
        for i in 0..10 {
            grid.append_item(&format!("item{}", i));
        }
        
        assert_eq!(grid.item_count(), 10);
        
        // Calculate visible items for a 300px viewport
        let (start, end) = grid.calculate_visible_items(300.0, 0.0);
        assert_eq!(start, 0);
        assert!(end <= 10);
    }
}
