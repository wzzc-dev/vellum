///|
/// Transition SDK for MoonBit GUI Framework
///|
/// Transition configuration
pub struct TransitionConfig {
  duration_ms : Int
  interactive : Bool
  curve : AnimationCurve
}

///|
/// Default transition configurations
pub fn default_push_config() -> TransitionConfig {
  {
    duration_ms: 300,
    interactive: true,
    curve: AnimationCurve::EaseInOut
  }
}

pub fn default_modal_config() -> TransitionConfig {
  {
    duration_ms: 350,
    interactive: true,
    curve: AnimationCurve::EaseOut
  }
}

///|
/// Transition animation style
pub enum TransitionStyle {
  Push
  Modal
  FullScreen
  PageCurl
  Flip
  Custom
}

///|
/// View visibility animation
pub enum VisibilityTransition {
  Fade
  Slide
  Scale
  Offset
}

///|
/// Helper functions to create transitions
pub fn fade_transition() -> VisibilityTransition {
  Fade
}

pub fn slide_transition() -> VisibilityTransition {
  Slide
}

pub fn scale_transition() -> VisibilityTransition {
  Scale
}

pub fn offset_transition() -> VisibilityTransition {
  Offset
}

///|
/// Helper functions to create transition styles
pub fn push_transition(config : TransitionConfig) -> TransitionStyle {
  Push
}

pub fn modal_transition(config : TransitionConfig) -> TransitionStyle {
  Modal
}

pub fn flip_transition(config : TransitionConfig) -> TransitionStyle {
  Flip
}
