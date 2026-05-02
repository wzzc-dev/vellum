//! Gesture Recognizer Implementation
//! 
//! This module provides a complete gesture recognition framework
//! modeled after Apple's UIKit gesture recognizers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

/// 2D point type
#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub fn distance(&self, other: &Point) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

/// Gesture recognizer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureState {
    /// Waiting for input
    Possible,
    /// Currently recognizing
    Began,
    /// Recognized gesture
    Changed,
    /// Recognized complete
    Ended,
    /// Recognized failed
    Failed,
    /// Canceled by system
    Cancelled,
}

/// Edge of screen for screen-edge gestures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// Direction for swipe
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Right,
    Left,
    Up,
    Down,
}

/// Type of gesture
#[derive(Debug, Clone)]
pub enum GestureType {
    Tap {
        taps_required: u32,
        touches_required: u32,
    },
    LongPress {
        minimum_duration_ms: u32,
        allow_movement: f32,
    },
    Pan {
        minimum_distance: f32,
        maximum_distance: Option<f32>,
    },
    Swipe {
        direction: SwipeDirection,
        minimum_distance: f32,
    },
    Pinch,
    Rotation,
    ScreenEdgePan {
        edges: Vec<Edge>,
    },
}

/// Single touch point
#[derive(Debug, Clone)]
pub struct TouchPoint {
    pub id: u64,
    pub position: Point,
    pub start_position: Point,
    pub timestamp_ms: u64,
    pub force: f32,
    pub radius: f32,
}

/// Gesture recognized result
#[derive(Debug, Clone)]
pub struct GestureResult {
    pub gesture_type: GestureType,
    pub state: GestureState,
    pub touches: Vec<TouchPoint>,
    pub velocity: Point,
    pub translation: Point,
    pub scale: Option<f32>,
    pub rotation: Option<f32>,
}

/// Touch event type
#[derive(Debug, Clone, Copy)]
pub enum TouchEventType {
    Began,
    Moved,
    Ended,
    Cancelled,
}

/// Touch event
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub event_type: TouchEventType,
    pub touches: Vec<TouchPoint>,
    pub timestamp: Instant,
}

/// Base gesture recognizer
pub struct GestureRecognizer {
    pub state: GestureState,
    pub enabled: bool,
    pub min_touches: u32,
    pub max_touches: u32,
    pub dependencies: Vec<Weak<Mutex<GestureRecognizer>>>,
    pub simultaneous: Vec<Weak<Mutex<GestureRecognizer>>>,
    pub active_touches: HashMap<u64, TouchPoint>,
    pub start_time: Option<Instant>,
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self {
            state: GestureState::Possible,
            enabled: true,
            min_touches: 1,
            max_touches: 1,
            dependencies: Vec::new(),
            simultaneous: Vec::new(),
            active_touches: HashMap::new(),
            start_time: None,
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        // Update active touches
        match event.event_type {
            TouchEventType::Began => {
                for touch in &event.touches {
                    self.active_touches.insert(touch.id, touch.clone());
                }
                self.start_time = Some(event.timestamp);
            }
            TouchEventType::Moved => {
                for touch in &event.touches {
                    self.active_touches.insert(touch.id, touch.clone());
                }
            }
            TouchEventType::Ended | TouchEventType::Cancelled => {
                for touch in &event.touches {
                    self.active_touches.remove(&touch.id);
                }
                if self.active_touches.is_empty() {
                    self.start_time = None;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = GestureState::Possible;
        self.active_touches.clear();
        self.start_time = None;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.reset();
        }
    }
}

/// Tap gesture recognizer
pub struct TapGestureRecognizer {
    pub base: GestureRecognizer,
    pub taps_required: u32,
    pub touches_required: u32,
    tap_count: u32,
    last_tap_time: Option<Instant>,
}

impl TapGestureRecognizer {
    pub fn new(taps_required: u32, touches_required: u32) -> Self {
        Self {
            base: GestureRecognizer::new(),
            taps_required,
            touches_required,
            tap_count: 0,
            last_tap_time: None,
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        match event.event_type {
            TouchEventType::Ended => {
                // Simple tap detection
                if self.base.active_touches.is_empty() {
                    self.tap_count += 1;
                    self.last_tap_time = Some(event.timestamp);

                    if self.tap_count >= self.taps_required {
                        self.base.state = GestureState::Ended;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state != GestureState::Ended {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::Tap {
                taps_required: self.taps_required,
                touches_required: self.touches_required,
            },
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: Point::zero(),
            translation: Point::zero(),
            scale: None,
            rotation: None,
        })
    }
}

/// Pan/drag gesture recognizer
pub struct PanGestureRecognizer {
    pub base: GestureRecognizer,
    pub minimum_distance: f32,
    pub maximum_distance: Option<f32>,
    start_location: Option<Point>,
    pub translation: Point,
    pub velocity: Point,
    last_location: Option<Point>,
    last_time: Option<Instant>,
}

impl PanGestureRecognizer {
    pub fn new(minimum_distance: f32, maximum_distance: Option<f32>) -> Self {
        Self {
            base: GestureRecognizer::new(),
            minimum_distance,
            maximum_distance,
            start_location: None,
            translation: Point::zero(),
            velocity: Point::zero(),
            last_location: None,
            last_time: None,
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        // Get primary touch
        let touch = event.touches.first();
        if touch.is_none() {
            return;
        }
        let touch = touch.unwrap();

        match event.event_type {
            TouchEventType::Began => {
                self.start_location = Some(touch.position);
                self.last_location = Some(touch.position);
                self.last_time = Some(event.timestamp);
                self.translation = Point::zero();
                self.velocity = Point::zero();
                self.base.state = GestureState::Possible;
            }
            TouchEventType::Moved => {
                if let (Some(start), Some(last), Some(last_t)) = (
                    self.start_location,
                    self.last_location,
                    self.last_time,
                ) {
                    let translation = Point::new(
                        touch.position.x - start.x,
                        touch.position.y - start.y,
                    );
                    self.translation = translation;

                    // Calculate velocity
                    let time_diff = event.timestamp.duration_since(last_t);
                    let time_sec = time_diff.as_secs_f32();
                    if time_sec > 0.0 {
                        self.velocity = Point::new(
                            (touch.position.x - last.x) / time_sec,
                            (touch.position.y - last.y) / time_sec,
                        );
                    }

                    // Check minimum distance to recognize
                    if self.base.state == GestureState::Possible {
                        let distance = start.distance(&touch.position);
                        if distance >= self.minimum_distance {
                            self.base.state = GestureState::Began;
                        }
                    } else {
                        self.base.state = GestureState::Changed;
                    }

                    self.last_location = Some(touch.position);
                    self.last_time = Some(event.timestamp);
                }
            }
            TouchEventType::Ended | TouchEventType::Cancelled => {
                if self.base.state == GestureState::Began || self.base.state == GestureState::Changed {
                    self.base.state = GestureState::Ended;
                } else {
                    self.base.state = GestureState::Failed;
                }
            }
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state == GestureState::Possible || self.base.state == GestureState::Failed {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::Pan {
                minimum_distance: self.minimum_distance,
                maximum_distance: self.maximum_distance,
            },
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: self.velocity,
            translation: self.translation,
            scale: None,
            rotation: None,
        })
    }
}

/// Long press gesture recognizer
pub struct LongPressGestureRecognizer {
    pub base: GestureRecognizer,
    pub minimum_duration_ms: u32,
    pub allow_movement: f32,
    start_location: Option<Point>,
}

impl LongPressGestureRecognizer {
    pub fn new(minimum_duration_ms: u32, allow_movement: f32) -> Self {
        Self {
            base: GestureRecognizer::new(),
            minimum_duration_ms,
            allow_movement,
            start_location: None,
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        match event.event_type {
            TouchEventType::Began => {
                let touch = event.touches.first();
                if let Some(t) = touch {
                    self.start_location = Some(t.position);
                }
                self.base.state = GestureState::Possible;
            }
            TouchEventType::Moved => {
                if let (Some(start), Some(touch)) = (self.start_location, event.touches.first()) {
                    let distance = start.distance(&touch.position);
                    if distance > self.allow_movement {
                        self.base.state = GestureState::Failed;
                    }
                }
            }
            TouchEventType::Ended | TouchEventType::Cancelled => {
                if let Some(start_time) = self.base.start_time {
                    let duration = event.timestamp.duration_since(start_time);
                    if duration.as_millis() as u32 >= self.minimum_duration_ms {
                        if self.base.state == GestureState::Possible {
                            self.base.state = GestureState::Ended;
                        }
                    } else {
                        self.base.state = GestureState::Failed;
                    }
                }
            }
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state != GestureState::Ended {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::LongPress {
                minimum_duration_ms: self.minimum_duration_ms,
                allow_movement: self.allow_movement,
            },
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: Point::zero(),
            translation: Point::zero(),
            scale: None,
            rotation: None,
        })
    }
}

/// Swipe gesture recognizer
pub struct SwipeGestureRecognizer {
    pub base: GestureRecognizer,
    pub direction: SwipeDirection,
    pub minimum_distance: f32,
    start_location: Option<Point>,
}

impl SwipeGestureRecognizer {
    pub fn new(direction: SwipeDirection, minimum_distance: f32) -> Self {
        Self {
            base: GestureRecognizer::new(),
            direction,
            minimum_distance,
            start_location: None,
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        match event.event_type {
            TouchEventType::Began => {
                let touch = event.touches.first();
                if let Some(t) = touch {
                    self.start_location = Some(t.position);
                }
            }
            TouchEventType::Ended => {
                if let (Some(start), Some(touch)) = (self.start_location, event.touches.first()) {
                    let dx = touch.position.x - start.x;
                    let dy = touch.position.y - start.y;

                    let recognized = match self.direction {
                        SwipeDirection::Right => dx >= self.minimum_distance && dy.abs() < dx.abs(),
                        SwipeDirection::Left => dx <= -self.minimum_distance && dy.abs() < dx.abs(),
                        SwipeDirection::Down => dy >= self.minimum_distance && dx.abs() < dy.abs(),
                        SwipeDirection::Up => dy <= -self.minimum_distance && dx.abs() < dy.abs(),
                    };

                    if recognized {
                        self.base.state = GestureState::Ended;
                    } else {
                        self.base.state = GestureState::Failed;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state != GestureState::Ended {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::Swipe {
                direction: self.direction,
                minimum_distance: self.minimum_distance,
            },
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: Point::zero(),
            translation: Point::zero(),
            scale: None,
            rotation: None,
        })
    }
}

/// Pinch gesture recognizer for scaling
pub struct PinchGestureRecognizer {
    pub base: GestureRecognizer,
    initial_distance: f32,
    current_distance: f32,
    start_points: HashMap<u64, Point>,
}

impl PinchGestureRecognizer {
    pub fn new() -> Self {
        Self {
            base: GestureRecognizer::new(),
            initial_distance: 0.0,
            current_distance: 0.0,
            start_points: HashMap::new(),
        }
    }

    fn calculate_distance(&self) -> f32 {
        let points: Vec<_> = self.start_points.values().collect();
        if points.len() >= 2 {
            points[0].distance(points[1])
        } else {
            0.0
        }
    }

    pub fn get_scale(&self) -> f32 {
        if self.initial_distance > 0.0 {
            self.current_distance / self.initial_distance
        } else {
            1.0
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        match event.event_type {
            TouchEventType::Began => {
                for touch in &event.touches {
                    self.start_points.insert(touch.id, touch.position);
                }
                if self.start_points.len() >= 2 {
                    self.initial_distance = self.calculate_distance();
                    self.current_distance = self.initial_distance;
                }
            }
            TouchEventType::Moved => {
                for touch in &event.touches {
                    self.start_points.insert(touch.id, touch.position);
                }
                if self.start_points.len() >= 2 {
                    let new_distance = self.calculate_distance();
                    self.current_distance = new_distance;
                    
                    let scale = self.get_scale();
                    if self.base.state == GestureState::Possible {
                        if scale > 1.1 || scale < 0.9 {
                            self.base.state = GestureState::Began;
                        }
                    } else {
                        self.base.state = GestureState::Changed;
                    }
                }
            }
            TouchEventType::Ended | TouchEventType::Cancelled => {
                if self.base.state == GestureState::Began || self.base.state == GestureState::Changed {
                    self.base.state = GestureState::Ended;
                } else {
                    self.base.state = GestureState::Failed;
                }
            }
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state == GestureState::Possible || self.base.state == GestureState::Failed {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::Pinch,
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: Point::zero(),
            translation: Point::zero(),
            scale: Some(self.get_scale()),
            rotation: None,
        })
    }
}

/// Rotation gesture recognizer
pub struct RotationGestureRecognizer {
    pub base: GestureRecognizer,
    initial_angle: f32,
    current_angle: f32,
    start_points: HashMap<u64, Point>,
}

impl RotationGestureRecognizer {
    pub fn new() -> Self {
        Self {
            base: GestureRecognizer::new(),
            initial_angle: 0.0,
            current_angle: 0.0,
            start_points: HashMap::new(),
        }
    }

    fn calculate_angle(&self) -> f32 {
        let points: Vec<_> = self.start_points.values().collect();
        if points.len() >= 2 {
            let dx = points[1].x - points[0].x;
            let dy = points[1].y - points[0].y;
            dy.atan2(dx)
        } else {
            0.0
        }
    }

    pub fn get_rotation(&self) -> f32 {
        self.current_angle - self.initial_angle
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        match event.event_type {
            TouchEventType::Began => {
                for touch in &event.touches {
                    self.start_points.insert(touch.id, touch.position);
                }
                if self.start_points.len() >= 2 {
                    self.initial_angle = self.calculate_angle();
                    self.current_angle = self.initial_angle;
                }
            }
            TouchEventType::Moved => {
                for touch in &event.touches {
                    self.start_points.insert(touch.id, touch.position);
                }
                if self.start_points.len() >= 2 {
                    self.current_angle = self.calculate_angle();
                    
                    let rotation = self.get_rotation().abs();
                    if self.base.state == GestureState::Possible {
                        if rotation > 0.2 {
                            self.base.state = GestureState::Began;
                        }
                    } else {
                        self.base.state = GestureState::Changed;
                    }
                }
            }
            TouchEventType::Ended | TouchEventType::Cancelled => {
                if self.base.state == GestureState::Began || self.base.state == GestureState::Changed {
                    self.base.state = GestureState::Ended;
                } else {
                    self.base.state = GestureState::Failed;
                }
            }
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state == GestureState::Possible || self.base.state == GestureState::Failed {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::Rotation,
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: Point::zero(),
            translation: Point::zero(),
            scale: None,
            rotation: Some(self.get_rotation()),
        })
    }
}

/// Screen edge pan gesture recognizer
pub struct ScreenEdgePanGestureRecognizer {
    pub base: GestureRecognizer,
    pub edge: Edge,
    pub edge_margin: f32,
    start_point: Option<Point>,
    pub translation: Point,
    pub screen_size: Option<(f32, f32)>,
}

impl ScreenEdgePanGestureRecognizer {
    pub fn new(edge: Edge, edge_margin: f32) -> Self {
        Self {
            base: GestureRecognizer::new(),
            edge,
            edge_margin,
            start_point: None,
            translation: Point::zero(),
            screen_size: None,
        }
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_size = Some((width, height));
    }

    fn is_at_edge(&self, point: Point) -> bool {
        if let Some((w, h)) = self.screen_size {
            match self.edge {
                Edge::Left => point.x <= self.edge_margin,
                Edge::Right => point.x >= w - self.edge_margin,
                Edge::Top => point.y <= self.edge_margin,
                Edge::Bottom => point.y >= h - self.edge_margin,
            }
        } else {
            false
        }
    }

    pub fn handle_touch(&mut self, event: &TouchEvent) {
        self.base.handle_touch(event);

        if !self.base.enabled {
            return;
        }

        // Get primary touch
        let touch = event.touches.first();
        if touch.is_none() {
            return;
        }
        let touch = touch.unwrap();

        match event.event_type {
            TouchEventType::Began => {
                if self.is_at_edge(touch.position) {
                    self.start_point = Some(touch.position);
                    self.translation = Point::zero();
                    self.base.state = GestureState::Possible;
                } else {
                    self.base.state = GestureState::Failed;
                }
            }
            TouchEventType::Moved => {
                if self.base.state != GestureState::Failed {
                    if let Some(start) = self.start_point {
                        self.translation = Point::new(
                            touch.position.x - start.x,
                            touch.position.y - start.y,
                        );
                        
                        if self.base.state == GestureState::Possible {
                            let distance = start.distance(&touch.position);
                            if distance >= 5.0 {
                                self.base.state = GestureState::Began;
                            }
                        } else {
                            self.base.state = GestureState::Changed;
                        }
                    }
                }
            }
            TouchEventType::Ended | TouchEventType::Cancelled => {
                if self.base.state == GestureState::Began || self.base.state == GestureState::Changed {
                    self.base.state = GestureState::Ended;
                } else {
                    self.base.state = GestureState::Failed;
                }
            }
        }
    }

    pub fn get_result(&self) -> Option<GestureResult> {
        if self.base.state == GestureState::Possible || self.base.state == GestureState::Failed {
            return None;
        }

        Some(GestureResult {
            gesture_type: GestureType::ScreenEdgePan {
                edges: vec![self.edge],
            },
            state: self.base.state,
            touches: self.base.active_touches.values().cloned().collect(),
            velocity: Point::zero(),
            translation: self.translation,
            scale: None,
            rotation: None,
        })
    }
}

/// Gesture registry for managing multiple recognizers
pub struct GestureRegistry {
    pub recognizers: Vec<Arc<Mutex<GestureRecognizer>>>,
}

impl GestureRegistry {
    pub fn new() -> Self {
        Self {
            recognizers: Vec::new(),
        }
    }

    pub fn add_recognizer(&mut self, recognizer: Arc<Mutex<GestureRecognizer>>) {
        self.recognizers.push(recognizer);
    }

    pub fn dispatch_touch(&mut self, event: &TouchEvent) {
        // Dispatch to all recognizers
        for recognizer in &self.recognizers {
            if let Ok(mut r) = recognizer.lock() {
                r.handle_touch(event);
            }
        }
    }
}
