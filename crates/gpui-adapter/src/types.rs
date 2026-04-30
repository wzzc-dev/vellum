use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }

    pub fn hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            Some(Self::rgb(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            Some(Self::rgba(r, g, b, a))
        } else {
            None
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::transparent()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= 0.0 && point.x < self.width && point.y >= 0.0 && point.y < self.height
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_size(size: Size) -> Self {
        Self::new(0.0, 0.0, size.width, size.height)
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);

        if right > x && bottom > y {
            Some(Rect::new(x, y, right - x, bottom - y))
        } else {
            None
        }
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Rect::new(x, y, right - x, bottom - y)
    }

    pub fn inflate(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x - dx, self.y - dy, self.width + dx * 2.0, self.height + dy * 2.0)
    }

    pub fn deflate(&self, dx: f32, dy: f32) -> Rect {
        self.inflate(-dx, -dy)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

impl Default for EdgeInsets {
    fn default() -> Self {
        Self::uniform(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlexParams {
    pub grow: f32,
    pub shrink: f32,
    pub basis: f32,
}

impl FlexParams {
    pub fn new(grow: f32, shrink: f32, basis: f32) -> Self {
        Self { grow, shrink, basis }
    }

    pub fn flex_none() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    pub fn flex_auto() -> Self {
        Self::new(1.0, 1.0, 0.0)
    }
}

impl Default for FlexParams {
    fn default() -> Self {
        Self::flex_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CrossAlignment {
    Start,
    Center,
    End,
    Stretch,
}

impl Default for CrossAlignment {
    fn default() -> Self {
        Self::Stretch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

impl Default for VerticalAlign {
    fn default() -> Self {
        Self::Top
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

impl FontWeight {
    pub fn to_numeric(&self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Normal => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDecoration {
    None,
    Underline,
    Overline,
    LineThrough,
}

impl Default for TextDecoration {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorShape {
    Default,
    Pointer,
    Text,
    Wait,
    Crosshair,
    Progress,
    Help,
    Move,
    NotAllowed,
    NoDrop,
    Grab,
    Grabbing,
}

impl Default for CursorShape {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl Default for Overflow {
    fn default() -> Self {
        Self::Visible
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    None,
    Solid,
    Dashed,
    Dotted,
    Double,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Border {
    pub width: f32,
    pub style: BorderStyle,
    pub color: Color,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            width: 0.0,
            style: BorderStyle::None,
            color: Color::transparent(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
}

impl BoxShadow {
    pub fn new(
        offset_x: f32,
        offset_y: f32,
        blur_radius: f32,
        spread_radius: f32,
        color: Color,
    ) -> Self {
        Self {
            offset_x,
            offset_y,
            blur_radius,
            spread_radius,
            color,
        }
    }
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            blur_radius: 0.0,
            spread_radius: 0.0,
            color: Color::transparent(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetDisplay {
    None,
    Flex,
    Block,
    Inline,
    InlineBlock,
    InlineFlex,
    Grid,
}

impl Default for WidgetDisplay {
    fn default() -> Self {
        Self::Flex
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl Default for FlexDirection {
    fn default() -> Self {
        Self::Row
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetPosition {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Default for WidgetPosition {
    fn default() -> Self {
        Self::Static
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Visible,
    Hidden,
    Collapse,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Visible
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_size: f32,
    pub font_family: String,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub color: Color,
    pub decoration: TextDecoration,
    pub letter_spacing: f32,
    pub line_height: f32,
    pub text_align: TextAlign,
}

impl TextStyle {
    pub fn new(
        font_size: f32,
        font_family: impl Into<String>,
        color: Color,
    ) -> Self {
        Self {
            font_size,
            font_family: font_family.into(),
            font_weight: FontWeight::default(),
            font_style: FontStyle::default(),
            color,
            decoration: TextDecoration::default(),
            letter_spacing: 0.0,
            line_height: 1.5,
            text_align: TextAlign::default(),
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new(14.0, "System", Color::black())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetLayout {
    pub display: WidgetDisplay,
    pub flex_direction: FlexDirection,
    pub flex_wrap: Wrap,
    pub justify_content: Alignment,
    pub align_items: CrossAlignment,
    pub align_content: Alignment,
    pub gap: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub position: WidgetPosition,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
}

impl Default for WidgetLayout {
    fn default() -> Self {
        Self {
            display: WidgetDisplay::default(),
            flex_direction: FlexDirection::default(),
            flex_wrap: Wrap::default(),
            justify_content: Alignment::default(),
            align_items: CrossAlignment::default(),
            align_content: Alignment::default(),
            gap: 0.0,
            row_gap: 0.0,
            column_gap: 0.0,
            position: WidgetPosition::default(),
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            overflow_x: Overflow::default(),
            overflow_y: Overflow::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Wrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl Default for Wrap {
    fn default() -> Self {
        Self::NoWrap
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    Light,
    Dark,
    System,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}
