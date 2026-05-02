///| Animation module for declarative animations
///|
///| This module provides animation functionality similar to SwiftUI animations,
///| Compose animate APIs, or Flutter AnimationController, including:
///| - Basic tween animations
///| - Spring physics animations
///| - Keyframe animations
///| - Transition widgets

pub all type AnimationControllerImpl = @animation.AnimationController

///| Animation curve types
pub enum AnimationCurve {
  /// Linear animation (constant speed)
  Linear
  /// Ease-in: start slow, end fast
  EaseIn
  /// Ease-out: start fast, end slow
  EaseOut
  /// Ease-in-out: start slow, speed up, end slow
  EaseInOut
  /// Bounce effect at the end
  Bounce
  /// Elastic effect (overshoot and settle)
  Elastic
}

pub fn AnimationCurve::to_wit(self : AnimationCurve) -> @animation.AnimationCurve {
  match self {
    Linear => @animation.linear
    EaseIn => @animation.ease-in
    EaseOut => @animation.ease-out
    EaseInOut => @animation.ease-in-out
    Bounce => @animation.bounce
    Elastic => @animation.elastic
  }
}

///| Animation direction
pub enum AnimationDirection {
  /// Normal direction
  Normal
  /// Reverse direction
  Reverse
  /// Alternate between normal and reverse
  Alternate
  /// Alternate in reverse
  AlternateReverse
}

pub fn AnimationDirection::to_wit(self : AnimationDirection) -> @animation.AnimationDirection {
  match self {
    Normal => @animation.normal
    Reverse => @animation.reverse
    Alternate => @animation.alternate
    AlternateReverse => @animation.alternate-reverse
  }
}

///| Animation status
pub enum AnimationStatus {
  /// Animation is stopped
  Stopped
  /// Animation is running forward
  RunningForward
  /// Animation is running backward
  RunningBackward
  /// Animation is paused
  Paused
}

///| Basic animation specification
pub struct AnimationSpec {
  /// Animation duration in milliseconds
  pub duration_ms : Int
  /// Animation curve
  pub curve : AnimationCurve
  /// Delay before starting in milliseconds
  pub delay_ms : Int
  /// Whether to repeat the animation
  pub repeat : Bool
  /// Number of repetitions (0 = infinite)
  pub repeat_count : Int
  /// Animation direction
  pub direction : AnimationDirection
}

pub fn AnimationSpec::default() -> AnimationSpec {
  {
    duration_ms: 300,
    curve: AnimationCurve::EaseInOut,
    delay_ms: 0,
    repeat: false,
    repeat_count: 0,
    direction: AnimationDirection::Normal
  }
}

pub fn AnimationSpec::to_wit(self : AnimationSpec) -> @animation.AnimationSpec {
  {
    duration_ms: self.duration_ms,
    curve: self.curve.to_wit(),
    delay_ms: self.delay_ms,
    repeat: self.repeat,
    repeat_count: self.repeat_count,
    direction: self.direction.to_wit()
  }
}

///| Builder for AnimationSpec
pub struct AnimationSpecBuilder {
  duration_ms : Int
  curve : AnimationCurve
  delay_ms : Int
  repeat : Bool
  repeat_count : Int
  direction : AnimationDirection
}

pub fn AnimationSpec::builder() -> AnimationSpecBuilder {
  {
    duration_ms: 300,
    curve: AnimationCurve::EaseInOut,
    delay_ms: 0,
    repeat: false,
    repeat_count: 0,
    direction: AnimationDirection::Normal
  }
}

pub fn AnimationSpecBuilder::duration(self : AnimationSpecBuilder, ms : Int) -> AnimationSpecBuilder {
  { ..self, duration_ms: ms }
}

pub fn AnimationSpecBuilder::curve(self : AnimationSpecBuilder, curve : AnimationCurve) -> AnimationSpecBuilder {
  { ..self, curve: curve }
}

pub fn AnimationSpecBuilder::delay(self : AnimationSpecBuilder, ms : Int) -> AnimationSpecBuilder {
  { ..self, delay_ms: ms }
}

pub fn AnimationSpecBuilder::repeat(self : AnimationSpecBuilder, count : Int) -> AnimationSpecBuilder {
  { ..self, repeat: count > 0, repeat_count: count }
}

pub fn AnimationSpecBuilder::direction(self : AnimationSpecBuilder, dir : AnimationDirection) -> AnimationSpecBuilder {
  { ..self, direction: dir }
}

pub fn AnimationSpecBuilder::build(self : AnimationSpecBuilder) -> AnimationSpec {
  {
    duration_ms: self.duration_ms,
    curve: self.curve,
    delay_ms: self.delay_ms,
    repeat: self.repeat,
    repeat_count: self.repeat_count,
    direction: self.direction
  }
}

///| Spring animation configuration
pub struct SpringConfig {
  /// Damping ratio (0.0 = no damping, 1.0 = critically damped)
  pub damping_ratio : Double
  /// Spring stiffness (higher = faster oscillation)
  pub stiffness : Double
  /// Mass of the animated object
  pub mass : Double
  /// Initial velocity
  pub initial_velocity : Double
}

pub fn SpringConfig::default() -> SpringConfig {
  {
    damping_ratio: 0.5,
    stiffness: 100.0,
    mass: 1.0,
    initial_velocity: 0.0
  }
}

pub fn SpringConfig::bouncy() -> SpringConfig {
  {
    damping_ratio: 0.3,
    stiffness: 100.0,
    mass: 1.0,
    initial_velocity: 0.0
  }
}

pub fn SpringConfig::stiff() -> SpringConfig {
  {
    damping_ratio: 0.8,
    stiffness: 300.0,
    mass: 1.0,
    initial_velocity: 0.0
  }
}

pub fn SpringConfig::to_wit(self : SpringConfig) -> @animation.SpringConfig {
  {
    damping_ratio: self.damping_ratio,
    stiffness: self.stiffness,
    mass: self.mass,
    initial_velocity: self.initial_velocity
  }
}

///| Animated property
pub struct AnimatedProperty {
  /// Property name (e.g., "opacity", "scale", "offset-x")
  pub name : String
  /// Start value
  pub from : Double
  /// End value
  pub to : Double
}

pub fn AnimatedProperty::new(name : String, from : Double, to : Double) -> AnimatedProperty {
  { name: name, from: from, to: to }
}

///| Keyframe definition
pub struct Keyframe {
  /// Progress point (0.0 to 1.0)
  pub progress : Double
  /// Value at this progress point
  pub value : Double
  /// Animation curve for this segment
  pub curve : Option[AnimationCurve]
}

pub fn Keyframe::new(progress : Double, value : Double) -> Keyframe {
  { progress: progress, value: value, curve: None }
}

pub fn Keyframe::with_curve(self : Keyframe, curve : AnimationCurve) -> Keyframe {
  { ..self, curve: Some(curve) }
}

///| Keyframe animation configuration
pub struct KeyframeAnimationConfig {
  /// Duration in milliseconds
  pub duration_ms : Int
  /// Keyframes
  pub keyframes : @list.List[Keyframe]
  /// Number of repetitions
  pub repeat_count : Int
  /// Whether to stay at final value after animation
  pub fill_mode : FillMode
}

///| Fill mode for animations
pub enum FillMode {
  /// Remove animation effect when complete
  None
  /// Keep the first frame until animation starts
  Backwards
  /// Keep the last frame after animation completes
  Forwards
  /// Keep both first and last frames
  Both
}

///| Animation controller for managing animations
pub struct AnimationController {
  controller : AnimationControllerImpl
}

pub fn AnimationController::new() -> AnimationController {
  { controller: @animation.animation_controller_new() }
}

///| Animate a property from one value to another
pub fn AnimationController::animate(
  self : AnimationController,
  property : String,
  from : Double,
  to : Double,
  spec : AnimationSpec
) -> Unit {
  @animation.animation_controller_animate(
    self.controller,
    property,
    from,
    to,
    spec.to_wit()
  )
}

///| Animate with duration and curve
pub fn AnimationController::animate_tween(
  self : AnimationController,
  property : String,
  from : Double,
  to : Double,
  duration_ms : Int,
  curve : AnimationCurve
) -> Unit {
  self.animate(property, from, to, {
    duration_ms: duration_ms,
    curve: curve,
    delay_ms: 0,
    repeat: false,
    repeat_count: 0,
    direction: AnimationDirection::Normal
  })
}

///| Animate with spring physics
pub fn AnimationController::spring_animate(
  self : AnimationController,
  property : String,
  to : Double,
  config : SpringConfig
) -> Unit {
  @animation.animation_controller_spring_animate(
    self.controller,
    property,
    to,
    config.to_wit()
  )
}

///| Animate with default spring
pub fn AnimationController::spring(
  self : AnimationController,
  property : String,
  to : Double
) -> Unit {
  self.spring_animate(property, to, SpringConfig::default())
}

///| Animate multiple properties at once
pub fn AnimationController::animate_all(
  self : AnimationController,
  properties : @list.List[AnimatedProperty],
  spec : AnimationSpec
) -> Unit {
  let wit_props = @list.map(properties, fn(p : AnimatedProperty) -> @animation.AnimatedProperty {
    { name: p.name, from: p.from, to: p.to }
  })
  @animation.animation_controller_animate_all(
    self.controller,
    wit_props,
    spec.to_wit()
  )
}

///| Create a keyframe animation
pub fn AnimationController::keyframe_animate(
  self : AnimationController,
  property : String,
  duration_ms : Int,
  keyframes : @list.List[Keyframe]
) -> Unit {
  let config : @animation.KeyframeAnimationConfig = {
    duration_ms: duration_ms,
    keyframes: @list.map(keyframes, fn(k : Keyframe) -> @animation.Keyframe {
      {
        progress: k.progress,
        value: k.value,
        curve: match k.curve {
          Some(c) => Some(c.to_wit())
          None => None
        }
      }
    }),
    repeat_count: 0,
    fill_mode: @animation.forwards
  }
  @animation.animation_controller_keyframe_animate(self.controller, property, config)
}

///| Stop a specific animation
pub fn AnimationController::stop(self : AnimationController, property : String) -> Unit {
  @animation.animation_controller_stop(self.controller, property)
}

///| Stop all animations
pub fn AnimationController::stop_all(self : AnimationController) -> Unit {
  @animation.animation_controller_stop_all(self.controller)
}

///| Pause a specific animation
pub fn AnimationController::pause(self : AnimationController, property : String) -> Unit {
  @animation.animation_controller_pause(self.controller, property)
}

///| Resume a paused animation
pub fn AnimationController::resume(self : AnimationController, property : String) -> Unit {
  @animation.animation_controller_resume(self.controller, property)
}

///| Check if an animation is running
pub fn AnimationController::is_running(self : AnimationController, property : String) -> Bool {
  @animation.animation_controller_is_running(self.controller, property)
}

///| Get current value of a property
pub fn AnimationController::get_value(self : AnimationController, property : String) -> Option[Double] {
  @animation.animation_controller_get_value(self.controller, property)
}

///| Animation event types
pub enum AnimationEvent {
  /// Animation started
  Started(property : String)
  /// Animation completed
  Completed(property : String)
  /// Animation cancelled
  Cancelled(property : String)
  /// Animation value changed
  ValueChanged(property : String, value : Double)
  /// Animation repeated
  Repeated(property : String, repeat_count : Int)
  /// Spring animation update
  SpringUpdate(property : String, value : Double, velocity : Double)
  /// Spring animation settled
  SpringSettled(property : String, value : Double)
}

///| Slide direction
pub enum SlideDirection {
  FromLeft
  FromRight
  FromTop
  FromBottom
}

///| Page transition types
pub enum PageTransition {
  /// Fade transition
  Fade(duration_ms : Int)
  /// Slide transition
  Slide(direction : SlideDirection, duration_ms : Int)
  /// Scale transition
  Scale(duration_ms : Int)
  /// Combined slide and fade
  SlideAndFade(direction : SlideDirection, duration_ms : Int)
}

pub fn PageTransition::fade() -> PageTransition { Fade(300) }
pub fn PageTransition::slide_from_right() -> PageTransition { Slide(SlideDirection::FromRight, 300) }
pub fn PageTransition::slide_from_bottom() -> PageTransition { Slide(SlideDirection::FromBottom, 300) }
