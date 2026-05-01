// vellum:gui/widget 类型绑定

pub struct WidgetLayout {
  display: WidgetDisplay,
  flex_direction: FlexDirection,
  flex_wrap: Wrap,
  justify_content: Alignment,
  align_items: CrossAlignment,
  align_content: Alignment,
  gap: f32,
  row_gap: f32,
  column_gap: f32,
  position: WidgetPosition,
  min_width: Option[f32],
  max_width: Option[f32],
  min_height: Option[f32],
  max_height: Option[f32],
  overflow_x: Overflow,
  overflow_y: Overflow,
} derive(Debug)

pub enum WidgetDisplay {
  None,
  Flex,
  Block,
  Inline,
  InlineBlock,
  InlineFlex,
  Grid,
}

pub enum FlexDirection {
  Row,
  RowReverse,
  Column,
  ColumnReverse,
}

pub enum WidgetPosition {
  StaticPosition,
  Relative,
  Absolute,
  Fixed,
  Sticky,
}
