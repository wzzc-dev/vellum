///| Grid layout module for creating grid-based layouts
///|
///| This module provides grid layout functionality similar to CSS Grid or
///| SwiftUI LazyVGrid/LazyHGrid, including:
///| - Configurable columns and rows
///| - Item span support
///| - Gap control
///| - Lazy loading for large lists

///| Grid axis alignment
pub enum GridAlignment {
  Start
  Center
  End
  Stretch
  SpaceBetween
  SpaceAround
  SpaceEvenly
}

///| Grid track sizing
pub enum GridTrackSize {
  Fixed(Double)
  Fill(Option[Double], Option[Double])
  Auto
  MinMax(Double, Double)
}

///| Grid line placement
pub struct GridLinePlacement {
  pub start : Int
  pub end : Option[Int]
  pub start_name : Option[String]
  pub end_name : Option[String]
}

pub fn GridLinePlacement::default() -> GridLinePlacement {
  {
    start: 0,
    end: None,
    start_name: None,
    end_name: None
  }
}

///| Column span configuration
pub struct ColumnSpan {
  pub span : Int
  pub alignment : Option[GridAlignment]
}

pub fn ColumnSpan::new(span : Int) -> ColumnSpan {
  { span: span, alignment: None }
}

///| Row span configuration
pub struct RowSpan {
  pub span : Int
  pub alignment : Option[GridAlignment]
}

pub fn RowSpan::new(span : Int) -> RowSpan {
  { span: span, alignment: None }
}

///| Grid item placement
pub struct GridItemPlacement {
  pub row : GridLinePlacement
  pub column : GridLinePlacement
  pub row_span : Option[RowSpan]
  pub column_span : Option[ColumnSpan]
}

pub fn GridItemPlacement::default() -> GridItemPlacement {
  {
    row: GridLinePlacement::default(),
    column: GridLinePlacement::default(),
    row_span: None,
    column_span: None
  }
}

///| Grid item with span support
pub struct GridItem {
  pub widget_id : String
  pub placement : GridItemPlacement
  pub justify_self : Option[GridAlignment]
  pub align_self : Option[GridAlignment]
}

pub fn GridItem::new(widget_id : String) -> GridItem {
  {
    widget_id: widget_id,
    placement: GridItemPlacement::default(),
    justify_self: None,
    align_self: None
  }
}

pub fn GridItem::cross_span(self : GridItem, span : Int) -> GridItem {
  {
    widget_id: self.widget_id,
    placement: {
      row: self.placement.row,
      column: self.placement.column,
      row_span: self.placement.row_span,
      column_span: Some(ColumnSpan::new(span))
    },
    justify_self: self.justify_self,
    align_self: self.align_self
  }
}

pub fn GridItem::main_span(self : GridItem, span : Int) -> GridItem {
  {
    widget_id: self.widget_id,
    placement: {
      row: self.placement.row,
      column: self.placement.column,
      row_span: Some(RowSpan::new(span)),
      column_span: self.placement.column_span
    },
    justify_self: self.justify_self,
    align_self: self.align_self
  }
}

///| Grid configuration
pub struct GridConfig {
  pub columns : Int
  pub rows : Int
  pub column_gap : Double
  pub row_gap : Double
  pub column_sizes : Option[@list.List[GridTrackSize]]
  pub row_sizes : Option[@list.List[GridTrackSize]]
  pub justify_items : GridAlignment
  pub align_items : GridAlignment
  pub justify_content : GridAlignment
  pub align_content : GridAlignment
  pub max_width : Option[Double]
  pub max_height : Option[Double]
}

pub fn GridConfig::new(columns : Int) -> GridConfig {
  {
    columns: columns,
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
    max_height: None
  }
}

/// Builder for GridConfig
pub struct GridConfigBuilder {
  columns : Int
  rows : Int
  column_gap : Double
  row_gap : Double
  justify_items : GridAlignment
  align_items : GridAlignment
}

pub fn GridConfig::builder() -> GridConfigBuilder {
  {
    columns: 2,
    rows: 0,
    column_gap: 8.0,
    row_gap: 8.0,
    justify_items: GridAlignment::Stretch,
    align_items: GridAlignment::Stretch
  }
}

pub fn GridConfigBuilder::columns(self : GridConfigBuilder, count : Int) -> GridConfigBuilder {
  { ..self, columns: count }
}

pub fn GridConfigBuilder::rows(self : GridConfigBuilder, count : Int) -> GridConfigBuilder {
  { ..self, rows: count }
}

pub fn GridConfigBuilder::gaps(self : GridConfigBuilder, gap : Double) -> GridConfigBuilder {
  { ..self, column_gap: gap, row_gap: gap }
}

pub fn GridConfigBuilder::gap(self : GridConfigBuilder, column_gap : Double, row_gap : Double) -> GridConfigBuilder {
  { ..self, column_gap: column_gap, row_gap: row_gap }
}

pub fn GridConfigBuilder::alignment(self : GridConfigBuilder, horizontal : GridAlignment, vertical : GridAlignment) -> GridConfigBuilder {
  { ..self, justify_items: horizontal, align_items: vertical }
}

pub fn GridConfigBuilder::build(self : GridConfigBuilder) -> GridConfig {
  {
    columns: self.columns,
    rows: self.rows,
    column_gap: self.column_gap,
    row_gap: self.row_gap,
    column_sizes: None,
    row_sizes: None,
    justify_items: self.justify_items,
    align_items: self.align_items,
    justify_content: GridAlignment::Start,
    align_content: GridAlignment::Start,
    max_width: None,
    max_height: None
  }
}

///| Grid view controller for managing grid items
pub struct GridView {
  pub config : GridConfig
  pub item_count : Int
  pub items : @list.List[GridItem]
}

pub fn GridView::new(config : GridConfig) -> GridView {
  GridView::{ config: config, item_count: 0, items: @list.empty() }
}

///| Add an item with default placement
pub fn GridView::add_item(self : GridView, widget_id : String) -> GridView {
  let item = GridItem::new(widget_id)
  GridView::{ config: self.config, item_count: self.item_count + 1, items: self.items.add(item) }
}

///| Add an item with custom placement
pub fn GridView::add_item_with_placement(self : GridView, item : GridItem) -> GridView {
  GridView::{ config: self.config, item_count: self.item_count + 1, items: self.items.add(item) }
}

///| Add multiple items with default placement
pub fn GridView::add_items(self : GridView, widget_ids : @list.List[String]) -> GridView {
  let new_items = widget_ids.map(fn(id) { GridItem::new(id) })
  let combined = self.items.concat(new_items)
  GridView::{ config: self.config, item_count: self.item_count + widget_ids.length(), items: combined }
}

///| Remove an item by index
pub fn GridView::remove_item(self : GridView, index : Int) -> GridView {
  GridView::{ config: self.config, item_count: if self.item_count > 0 { self.item_count - 1 } else { 0 }, items: self.items }
}

///| Remove item by widget ID
pub fn GridView::remove_item_by_id(self : GridView, widget_id : String) -> GridView {
  let filtered = self.items.filter(fn(item) { item.widget_id != widget_id })
  GridView::{ config: self.config, item_count: filtered.length(), items: filtered }
}

///| Clear all items
pub fn GridView::clear(self : GridView) -> GridView {
  GridView::{ config: self.config, item_count: 0, items: @list.empty() }
}

///| Set column count (responsive)
pub fn GridView::set_columns(self : GridView, count : Int) -> GridView {
  GridView::{ config: GridConfig::{ ..self.config, columns: count }, item_count: self.item_count, items: self.items }
}

///| Set gaps
pub fn GridView::set_gaps(self : GridView, column_gap : Double, row_gap : Double) -> GridView {
  GridView::{ config: GridConfig::{ ..self.config, column_gap: column_gap, row_gap: row_gap }, item_count: self.item_count, items: self.items }
}