///| Gesture recognizer state
pub enum GestureState {
  Possible
  Began
  Changed
  Ended
  Failed
  Cancelled
}

///| Edge of screen for screen-edge gestures
pub enum Edge {
  Left
  Right
  Top
  Bottom
}

///| Direction for swipe
pub enum SwipeDirection {
  Right
  Left
  Up
  Down
}

///| 2D Point type
pub struct Point {
  pub x : Double
  pub y : Double
}

pub fn Point::new(x : Double, y : Double) -> Point {
  Point::{ x, y }
}

pub fn Point::zero() -> Point {
  Point::{ x: 0.0, y: 0.0 }
}

///| Touch point
pub struct TouchPoint {
  pub id : UInt64
  pub position : Point
  pub start_position : Point
  pub timestamp_ms : UInt64
  pub force : Double
  pub radius : Double
}

///| Gesture type variants
pub enum GestureType {
  Tap(Int, Int)
  LongPress(Int, Double)
  Pan(Double, Option[Double])
  Swipe(SwipeDirection, Double)
  Pinch
  Rotation
  ScreenEdgePan(@list.List[Edge])
}

///| Gesture recognized result
pub struct GestureResult {
  pub gesture_type : GestureType
  pub state : GestureState
  pub touches : @list.List[TouchPoint]
  pub velocity : Point
  pub translation : Point
  pub scale : Option[Double]
  pub rotation : Option[Double]
}

///| Helper functions to create gesture types
pub fn tap_gesture(taps : Int, touches : Int) -> GestureType {
  GestureType::Tap(taps, touches)
}

pub fn single_tap_gesture() -> GestureType {
  tap_gesture(1, 1)
}

pub fn double_tap_gesture() -> GestureType {
  tap_gesture(2, 1)
}

pub fn pan_gesture(min_distance : Double) -> GestureType {
  GestureType::Pan(min_distance, None)
}

pub fn long_press_gesture(duration_ms : Int, allow_movement : Double) -> GestureType {
  GestureType::LongPress(duration_ms, allow_movement)
}

pub fn swipe_gesture(direction : SwipeDirection, min_distance : Double) -> GestureType {
  GestureType::Swipe(direction, min_distance)
}

pub fn swipe_right(min_distance : Double) -> GestureType {
  swipe_gesture(SwipeDirection::Right, min_distance)
}

pub fn swipe_left(min_distance : Double) -> GestureType {
  swipe_gesture(SwipeDirection::Left, min_distance)
}

pub fn pinch_gesture() -> GestureType {
  GestureType::Pinch
}

pub fn rotation_gesture() -> GestureType {
  GestureType::Rotation
}

pub fn screen_edge_pan_gesture(edge : Edge) -> GestureType {
  GestureType::ScreenEdgePan(@list.empty().add(edge))
}

pub fn left_edge_pan() -> GestureType {
  screen_edge_pan_gesture(Edge::Left)
}

pub fn right_edge_pan() -> GestureType {
  screen_edge_pan_gesture(Edge::Right)
}

///| Gesture event callback type
pub type GestureHandler = (GestureType, GestureState, GestureResult) -> Unit