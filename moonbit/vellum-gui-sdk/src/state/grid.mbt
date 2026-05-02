///| Grid layout module for creating grid-based layouts
///|
///| This module provides grid layout functionality similar to CSS Grid or
///| SwiftUI LazyVGrid/LazyHGrid, including:
///| - Configurable columns and rows
///| - Item span support
///| - Gap control
///| - Lazy loading for large lists

pub all type GridViewImpl = @grid.GridView
pub all type LazyGridViewImpl = @grid.LazyGridView

///| Grid axis alignment
pub enum GridAlignment {
  /// Align to start
  Start
  /// Align to center
  Center
  /// Align to end
  End
  /// Stretch to fill space
  Stretch
  /// Space items evenly
  SpaceBetween
  /// Space items evenly with space around
  SpaceAround
  /// Space items evenly with space equally
  SpaceEvenly
}

pub fn GridAlignment::to_wit(self : GridAlignment) -> @grid.GridAlignment {
  match self {
    Start => @grid.start
    Center => @grid.center
    End => @grid.end
    Stretch => @grid.stretch
    SpaceBetween => @grid.space-between
    SpaceAround => @grid.space-around
    SpaceEvenly => @grid.space-evenly
  }
}

///| Grid track sizing
pub enum GridTrackSize {
  /// Fixed size in pixels
  Fixed(Double)
  /// Fill remaining space
  Fill { min_size : Option[Double], max_size : Option[Double] }
  /// Size based on content
  Auto
  /// Minimum size
  MinMax { min : Double, max : Double }
}

///| Grid line placement
pub struct GridLinePlacement {
  /// Start line (1-indexed)
  pub start : Int
  /// End line (inclusive)
  pub end : Option[Int]
  /// Named line start
  pub start_name : Option[String]
  /// Named line end
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
  /// Number of columns to span
  pub span : Int
  /// Alignment within the span
  pub alignment : Option[GridAlignment]
}

pub fn ColumnSpan::new(span : Int) -> ColumnSpan {
  { span: span, alignment: None }
}

///| Row span configuration
pub struct RowSpan {
  /// Number of rows to span
  pub span : Int
  /// Alignment within the span
  pub alignment : Option[GridAlignment]
}

pub fn RowSpan::new(span : Int) -> RowSpan {
  { span: span, alignment: None }
}

///| Grid item placement
pub struct GridItemPlacement {
  /// Row placement
  pub row : GridLinePlacement
  /// Column placement
  pub column : GridLinePlacement
  /// Row span
  pub row_span : Option[RowSpan]
  /// Column span
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
  /// Widget identifier
  widget_id : String
  /// Placement configuration
  pub placement : GridItemPlacement
  /// Custom alignment override
  pub justify_self : Option[GridAlignment]
  /// Custom alignment override
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
  /// Number of columns (for grid-template-columns)
  pub columns : Int
  /// Number of rows (for grid-template-rows, 0 = auto)
  pub rows : Int
  /// Column gap
  pub column_gap : Double
  /// Row gap
  pub row_gap : Double
  /// Column sizing rules
  pub column_sizes : Option[@list.List[GridTrackSize]]
  /// Row sizing rules
  pub row_sizes : Option[@list.List[GridTrackSize]]
  /// Default horizontal alignment
  pub justify_items : GridAlignment
  /// Default vertical alignment
  pub align_items : GridAlignment
  /// Horizontal alignment of the entire grid
  pub justify_content : GridAlignment
  /// Vertical alignment of the entire grid
  pub align_content : GridAlignment
  /// Maximum width
  pub max_width : Option[Double]
  /// Maximum height
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

pub fn GridConfig::to_wit(self : GridConfig) -> @grid.GridConfig {
  {
    columns: self.columns,
    rows: self.rows,
    column_gap: self.column_gap,
    row_gap: self.row_gap,
    column_sizes: match self.column_sizes {
      Some(sizes) => Some(@list.map(sizes, fn(s : GridTrackSize) -> @grid.GridTrackSize {
        match s {
          Fixed(size) => @grid.fixed({ size: size })
          Fill { min_size, max_size } => @grid.fill({ min_size: min_size, max_size: max_size })
          Auto => @grid.auto
          MinMax { min, max } => @grid.minmax({ min: min, max: max })
        }
      }))
      None => None
    },
    row_sizes: match self.row_sizes {
      Some(sizes) => Some(@list.map(sizes, fn(s : GridTrackSize) -> @grid.GridTrackSize {
        match s {
          Fixed(size) => @grid.fixed({ size: size })
          Fill { min_size, max_size } => @grid.fill({ min_size: min_size, max_size: max_size })
          Auto => @grid.auto
          MinMax { min, max } => @grid.minmax({ min: min, max: max })
        }
      }))
      None => None
    },
    justify_items: self.justify_items.to_wit(),
    align_items: self.align_items.to_wit(),
    justify_content: self.justify_content.to_wit(),
    align_content: self.align_content.to_wit(),
    max_width: self.max_width,
    max_height: self.max_height
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
  controller : GridViewImpl
  /// Grid configuration
  pub config : GridConfig
  /// Number of items
  pub item_count : Int
}

pub fn GridView::new(config : GridConfig) -> GridView {
  let controller = @grid.grid_view_new(config.to_wit())
  { controller: controller, config: config, item_count: 0 }
}

///| Add an item with default placement
pub fn GridView::add_item(self : GridView, widget_id : String) -> Unit {
  @grid.grid_view_add_item(self.controller, widget_id)
  { controller: self.controller, config: self.config, item_count: self.item_count + 1 }
}

///| Add an item with custom placement
pub fn GridView::add_item_with_placement(self : GridView, item : GridItem) -> Unit {
  let placement : @grid.GridItemPlacement = {
    row: {
      start: item.placement.row.start,
      end: item.placement.row.end,
      start_name: item.placement.row.start_name,
      end_name: item.placement.row.end_name
    },
    column: {
      start: item.placement.column.start,
      end: item.placement.column.end,
      start_name: item.placement.column.start_name,
      end_name: item.placement.column.end_name
    },
    row_span: match item.placement.row_span {
      Some(rs) => Some({ span: rs.span, alignment: match rs.alignment {
        Some(a) => Some(a.to_wit())
        None => None
      }})
      None => None
    },
    column_span: match item.placement.column_span {
      Some(cs) => Some({ span: cs.span, alignment: match cs.alignment {
        Some(a) => Some(a.to_wit())
        None => None
      }})
      None => None
    },
    justify_self: match item.justify_self {
      Some(a) => Some(a.to_wit())
      None => None
    },
    align_self: match item.align_self {
      Some(a) => Some(a.to_wit())
      None => None
    }
  }
  @grid.grid_view_add_item_with_placement(self.controller, { widget_id: item.widget_id, placement: placement })
  { controller: self.controller, config: self.config, item_count: self.item_count + 1 }
}

///| Add multiple items with default placement
pub fn GridView::add_items(self : GridView, widget_ids : @list.List[String]) -> Unit {
  for widget_id in widget_ids {
    @grid.grid_view_add_item(self.controller, widget_id)
  }
  { controller: self.controller, config: self.config, item_count: self.item_count + @list.length(widget_ids) }
}

///| Remove an item by index
pub fn GridView::remove_item(self : GridView, index : Int) -> Unit {
  @grid.grid_view_remove_item(self.controller, index)
  { controller: self.controller, config: self.config, item_count: @int.max(0, self.item_count - 1) }
}

///| Remove item by widget ID
pub fn GridView::remove_item_by_id(self : GridView, widget_id : String) -> Bool {
  let result = @grid.grid_view_remove_item_by_id(self.controller, widget_id)
  if result {
    { controller: self.controller, config: self.config, item_count: @int.max(0, self.item_count - 1) }
  } else {
    self
  }
  result
}

///| Clear all items
pub fn GridView::clear(self : GridView) -> Unit {
  @grid.grid_view_clear(self.controller)
  { controller: self.controller, config: self.config, item_count: 0 }
}

///| Set column count (responsive)
pub fn GridView::set_columns(self : GridView, count : Int) -> GridView {
  @grid.grid_view_set_columns(self.controller, count)
  { controller: self.controller, config: { ..self.config, columns: count }, item_count: self.item_count }
}

///| Set gaps
pub fn GridView::set_gaps(self : GridView, column_gap : Double, row_gap : Double) -> GridView {
  @grid.grid_view_set_gaps(self.controller, column_gap, row_gap)
  { controller: self.controller, config: { ..self.config, column_gap: column_gap, row_gap: row_gap }, item_count: self.item_count }
}
