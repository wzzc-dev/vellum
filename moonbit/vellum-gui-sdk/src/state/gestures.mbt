// MoonBit GUI Framework - Gesture SDK
// Provides gesture recognition types and utilities for MoonBit extensions

// Gesture recognizer state
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

// Edge of screen for screen-edge gestures
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

// Direction for swipe
pub enum SwipeDirection {
    Right,
    Left,
    Up,
    Down,
}

// 2D Point type
pub struct Point {
    pub x: Float,
    pub y: Float,
}

pub impl Point {
    pub fn new(x: Float, y: Float) -> Self {
        Point { x, y }
    }

    pub fn zero() -> Self {
        Point { x: 0.0, y: 0.0 }
    }
}

// Touch point
pub struct TouchPoint {
    pub id: U64,
    pub position: Point,
    pub start_position: Point,
    pub timestamp_ms: U64,
    pub force: Float,
    pub radius: Float,
}

// Gesture type variants
pub enum GestureType {
    Tap { taps_required: U32, touches_required: U32 },
    LongPress { minimum_duration_ms: U32, allow_movement: Float },
    Pan { minimum_distance: Float, maximum_distance: Option<Float> },
    Swipe { direction: SwipeDirection, minimum_distance: Float },
    Pinch,
    Rotation,
    ScreenEdgePan { edges: List<Edge> },
}

// Gesture recognized result
pub struct GestureResult {
    pub gesture_type: GestureType,
    pub state: GestureState,
    pub touches: List<TouchPoint>,
    pub velocity: Point,
    pub translation: Point,
    pub scale: Option<Float>,
    pub rotation: Option<Float>,
}

// Helper functions to create gesture types
pub fn tap_gesture(taps: U32, touches: U32) -> GestureType {
    GestureType::Tap { taps_required: taps, touches_required: touches }
}

pub fn single_tap_gesture() -> GestureType {
    tap_gesture(1, 1)
}

pub fn double_tap_gesture() -> GestureType {
    tap_gesture(2, 1)
}

pub fn pan_gesture(min_distance: Float) -> GestureType {
    GestureType::Pan { minimum_distance: min_distance, maximum_distance: None }
}

pub fn long_press_gesture(duration_ms: U32, allow_movement: Float) -> GestureType {
    GestureType::LongPress { minimum_duration_ms: duration_ms, allow_movement }
}

pub fn swipe_gesture(direction: SwipeDirection, min_distance: Float) -> GestureType {
    GestureType::Swipe { direction, minimum_distance: min_distance }
}

pub fn swipe_right(min_distance: Float) -> GestureType {
    swipe_gesture(SwipeDirection::Right, min_distance)
}

pub fn swipe_left(min_distance: Float) -> GestureType {
    swipe_gesture(SwipeDirection::Left, min_distance)
}

pub fn pinch_gesture() -> GestureType {
    GestureType::Pinch
}

pub fn rotation_gesture() -> GestureType {
    GestureType::Rotation
}

pub fn screen_edge_pan_gesture(edge: Edge) -> GestureType {
    GestureType::ScreenEdgePan { edges: List::new(edge) }
}

pub fn left_edge_pan() -> GestureType {
    screen_edge_pan_gesture(Edge::Left)
}

pub fn right_edge_pan() -> GestureType {
    screen_edge_pan_gesture(Edge::Right)
}

// Gesture event callback type
pub type GestureHandler = (GestureType, GestureState, GestureResult) -> Unit;
