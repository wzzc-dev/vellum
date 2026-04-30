use crate::error::{AdapterError, Result};
use crate::types::{Color, Point, Rect, Size, TextStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaintKind {
    Fill,
    Stroke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub width: f32,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
    pub miter_limit: f32,
    pub dash_pattern: Vec<f32>,
    pub dash_offset: f32,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            miter_limit: 4.0,
            dash_pattern: Vec::new(),
            dash_offset: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaintStyle {
    pub kind: PaintKind,
    pub color: Color,
    pub stroke_width: f32,
    pub stroke_cap: StrokeCap,
    pub stroke_join: StrokeJoin,
    pub stroke_miter_limit: f32,
    pub blend_mode: BlendMode,
}

impl Default for PaintStyle {
    fn default() -> Self {
        Self {
            kind: PaintKind::Fill,
            color: Color::black(),
            stroke_width: 1.0,
            stroke_cap: StrokeCap::Butt,
            stroke_join: StrokeJoin::Miter,
            stroke_miter_limit: 4.0,
            blend_mode: BlendMode::Normal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Image {
    pub fn new(id: String, width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            id,
            width,
            height,
            data,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(AdapterError::InvalidArgument(
                "Image data too small".to_string(),
            ));
        }
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            width: 0,
            height: 0,
            data: bytes.to_vec(),
        })
    }
}

#[derive(Default)]
pub struct ImageCache {
    images: HashMap<String, Image>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, id: &str, image: Image) {
        self.images.insert(id.to_string(), image);
    }

    pub fn get(&self, id: &str) -> Option<&Image> {
        self.images.get(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Image> {
        self.images.remove(id)
    }
}

#[derive(Default)]
pub struct Canvas {
    pub width: f32,
    pub height: f32,
    pub commands: Vec<PaintCommand>,
}

impl Canvas {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            commands: Vec::new(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.commands.push(PaintCommand::Clear(color));
    }

    pub fn draw_rect(&mut self, rect: Rect, style: PaintStyle) {
        self.commands.push(PaintCommand::DrawRect(rect, style));
    }

    pub fn draw_rounded_rect(&mut self, rect: Rect, radius: f32, style: PaintStyle) {
        self.commands
            .push(PaintCommand::DrawRoundedRect(rect, radius, style));
    }

    pub fn draw_circle(&mut self, center: Point, radius: f32, style: PaintStyle) {
        self.commands
            .push(PaintCommand::DrawCircle(center, radius, style));
    }

    pub fn draw_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, style: PaintStyle) {
        self.commands
            .push(PaintCommand::DrawEllipse(center, radius_x, radius_y, style));
    }

    pub fn draw_line(&mut self, p1: Point, p2: Point, style: StrokeStyle) {
        self.commands.push(PaintCommand::DrawLine(p1, p2, style));
    }

    pub fn draw_text(&mut self, text: &str, position: Point, style: TextStyle) {
        self.commands
            .push(PaintCommand::DrawText(text.to_string(), position, style));
    }

    pub fn draw_image(&mut self, image: &Image, dest: Rect, source: Option<Rect>) {
        self.commands.push(PaintCommand::DrawImage {
            image_id: image.id.clone(),
            dest,
            source,
        });
    }

    pub fn save(&mut self) {
        self.commands.push(PaintCommand::Save);
    }

    pub fn restore(&mut self) {
        self.commands.push(PaintCommand::Restore);
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        self.commands.push(PaintCommand::Translate(x, y));
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        self.commands.push(PaintCommand::Scale(x, y));
    }

    pub fn rotate(&mut self, angle: f32) {
        self.commands.push(PaintCommand::Rotate(angle));
    }

    pub fn clip_rect(&mut self, rect: Rect) {
        self.commands.push(PaintCommand::ClipRect(rect));
    }

    pub fn reset_clip(&mut self) {
        self.commands.push(PaintCommand::ResetClip);
    }

    pub fn measure_text(&self, text: &str, style: &TextStyle) -> Size {
        let char_width = style.font_size * 0.6;
        let width = text.len() as f32 * char_width;
        let height = style.font_size * style.line_height;
        Size::new(width, height)
    }

    pub fn clear_commands(&mut self) {
        self.commands.clear();
    }

    pub fn get_commands(&self) -> &Vec<PaintCommand> {
        &self.commands
    }
}

#[derive(Debug, Clone)]
pub enum PaintCommand {
    Clear(Color),
    DrawRect(Rect, PaintStyle),
    DrawRoundedRect(Rect, f32, PaintStyle),
    DrawCircle(Point, f32, PaintStyle),
    DrawEllipse(Point, f32, f32, PaintStyle),
    DrawLine(Point, Point, StrokeStyle),
    DrawText(String, Point, TextStyle),
    DrawImage {
        image_id: String,
        dest: Rect,
        source: Option<Rect>,
    },
    Save,
    Restore,
    Translate(f32, f32),
    Scale(f32, f32),
    Rotate(f32),
    ClipRect(Rect),
    ResetClip,
}

#[derive(Default)]
pub struct PaintState {
    pub canvas: Canvas,
    pub image_cache: ImageCache,
}

impl PaintState {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            canvas: Canvas::new(width, height),
            image_cache: ImageCache::new(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.canvas.clear(color);
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.canvas.width = width;
        self.canvas.height = height;
    }
}
