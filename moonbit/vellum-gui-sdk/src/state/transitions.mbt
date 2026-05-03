// MoonBit GUI Framework - Transition SDK
// Provides transition animation types and utilities for MoonBit extensions

// Transition configuration
pub struct TransitionConfig {
    pub duration_ms: U32,
    pub interactive: Bool,
    pub curve: AnimationCurve,
}

// Default transition configurations
pub fn default_push_config() -> TransitionConfig {
    TransitionConfig {
        duration_ms: 300,
        interactive: true,
        curve: AnimationCurve::EaseInOut,
    }
}

pub fn default_modal_config() -> TransitionConfig {
    TransitionConfig {
        duration_ms: 350,
        interactive: true,
        curve: AnimationCurve::EaseOut,
    }
}

// Transition animation style
pub enum TransitionStyle {
    Push,
    Modal,
    FullScreen,
    PageCurl,
    Flip,
    Custom,
}

// Slide edge for visibility transitions
pub enum SlideEdge {
    Top,
    Bottom,
    Left,
    Right,
}

// View visibility animation
pub enum VisibilityTransition {
    Fade,
    Slide { edge: SlideEdge },
    Scale { factor: Float },
    Offset { x: Float, y: Float },
}

// Animated container widget configuration
pub struct AnimatedContainerConfig {
    pub insert_transition: VisibilityTransition,
    pub remove_transition: VisibilityTransition,
    pub animation: AnimationSpec,
}

// Shared element configuration
pub struct SharedElementConfig {
    pub share_id: String,
    pub bounds: Rect,
    pub options: SharedElementOptions,
}

pub struct SharedElementOptions {
    pub duration_ms: U32,
    pub curve: AnimationCurve,
    pub use_spring: Bool,
    pub spring: Option<SpringConfig>,
}

impl SharedElementOptions {
    pub fn default() -> SharedElementOptions {
        SharedElementOptions::{
            duration_ms: 300,
            curve: AnimationCurve::EaseInOut,
            use_spring: false,
            spring: None,
        }
    }
}

// Helper functions to create transitions
pub fn fade_transition() -> VisibilityTransition {
    VisibilityTransition::Fade
}

pub fn slide_from_edge(edge: SlideEdge) -> VisibilityTransition {
    VisibilityTransition::Slide { edge }
}

pub fn scale_transition(factor: Float) -> VisibilityTransition {
    VisibilityTransition::Scale { factor }
}

pub fn offset_transition(x: Float, y: Float) -> VisibilityTransition {
    VisibilityTransition::Offset { x, y }
}

pub fn slide_from_left() -> VisibilityTransition {
    slide_from_edge(SlideEdge::Left)
}

pub fn slide_from_right() -> VisibilityTransition {
    slide_from_edge(SlideEdge::Right)
}

pub fn slide_from_top() -> VisibilityTransition {
    slide_from_edge(SlideEdge::Top)
}

pub fn slide_from_bottom() -> VisibilityTransition {
    slide_from_edge(SlideEdge::Bottom)
}

// Helper functions to create transition styles
pub fn push_transition(_config: TransitionConfig) -> TransitionStyle {
    TransitionStyle::Push
}

pub fn modal_transition(_config: TransitionConfig) -> TransitionStyle {
    TransitionStyle::Modal
}

pub fn flip_transition(_config: TransitionConfig) -> TransitionStyle {
    TransitionStyle::Flip
}
