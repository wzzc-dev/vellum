//! Shared Element Transition Implementation
//! 
//! This module provides shared element transition functionality,
//! similar to Flutter's Hero animations or Android's shared element transitions.

use std::collections::HashMap;

use crate::animation_render::{AnimationCurve, AnimationSpec, SpringConfig};
use crate::transition_render::{Rect, Point, Transform, ViewId};

/// Shared element options
#[derive(Debug, Clone)]
pub struct SharedElementOptions {
    pub duration_ms: u32,
    pub curve: AnimationCurve,
    pub use_spring: bool,
    pub spring: Option<SpringConfig>,
}

impl Default for SharedElementOptions {
    fn default() -> Self {
        Self {
            duration_ms: 300,
            curve: AnimationCurve::EaseInOut,
            use_spring: false,
            spring: None,
        }
    }
}

/// Shared element configuration
#[derive(Debug, Clone)]
pub struct SharedElementConfig {
    pub share_id: String,
    pub bounds: Rect,
    pub options: SharedElementOptions,
}

/// Shared element state
#[derive(Debug, Clone)]
pub struct SharedElementState {
    pub from_view: ViewId,
    pub to_view: ViewId,
    pub from_bounds: Rect,
    pub to_bounds: Rect,
    pub from_corner_radius: f32,
    pub to_corner_radius: f32,
    pub from_transform: Transform,
    pub to_transform: Transform,
    pub snapshot: Option<Vec<u8>>,
    pub animation: Option<ElementAnimation>,
}

/// Element animation
#[derive(Debug, Clone)]
pub enum ElementAnimation {
    Tween {
        spec: AnimationSpec,
        start_time: std::time::Instant,
    },
    Spring {
        config: SpringConfig,
        start_time: std::time::Instant,
    },
}

/// Shared element manager
pub struct SharedElementManager {
    pub elements: HashMap<String, SharedElementState>,
}

impl SharedElementManager {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
        }
    }

    /// Capture shared element from source view
    pub fn capture_shared_element(
        &mut self,
        share_id: String,
        view: ViewId,
        bounds: Rect,
        corner_radius: f32,
        transform: Transform,
        snapshot: Option<Vec<u8>>,
    ) {
        let state = SharedElementState {
            from_view: view.clone(),
            to_view: view,
            from_bounds: bounds,
            to_bounds: bounds,
            from_corner_radius: corner_radius,
            to_corner_radius: corner_radius,
            from_transform: transform,
            to_transform: transform,
            snapshot,
            animation: None,
        };
        self.elements.insert(share_id, state);
    }

    /// Set destination for a shared element
    pub fn set_element_destination(
        &mut self,
        share_id: String,
        view: ViewId,
        bounds: Rect,
        corner_radius: f32,
        transform: Transform,
        options: SharedElementOptions,
    ) {
        if let Some(element) = self.elements.get_mut(&share_id) {
            element.to_view = view;
            element.to_bounds = bounds;
            element.to_corner_radius = corner_radius;
            element.to_transform = transform;

            // Set up animation
            element.animation = if options.use_spring {
                Some(ElementAnimation::Spring {
                    config: options.spring.unwrap_or_default(),
                    start_time: std::time::Instant::now(),
                })
            } else {
                Some(ElementAnimation::Tween {
                    spec: AnimationSpec {
                        duration_ms: options.duration_ms,
                        curve: options.curve,
                        delay_ms: 0,
                        repeat: false,
                        repeat_count: 0,
                        direction: crate::animation_render::AnimationDirection::Normal,
                    },
                    start_time: std::time::Instant::now(),
                })
            };
        }
    }

    /// Calculate interpolated state for a given progress
    pub fn interpolate_state(
        &self,
        share_id: &str,
        progress: f32,
    ) -> Option<InterpolatedElement> {
        let element = self.elements.get(share_id)?;

        let progress_clamped = progress.clamp(0.0, 1.0);
        let mut eased = progress_clamped;

        // Apply curve if we have an animation
        if let Some(ref animation) = element.animation {
            match animation {
                ElementAnimation::Tween { spec, .. } => {
                    eased = spec.curve.apply(progress_clamped);
                }
                ElementAnimation::Spring { config, .. } => {
                    // Simple spring easing approximation
                    eased = spring_approximation(progress_clamped, config.damping_ratio);
                }
            }
        }

        let interpolator = SharedElementInterpolator {
            from_rect: element.from_bounds,
            to_rect: element.to_bounds,
            from_corner_radius: element.from_corner_radius,
            to_corner_radius: element.to_corner_radius,
            from_transform: element.from_transform,
            to_transform: element.to_transform,
        };

        let (bounds, corner_radius, transform) = interpolator.interpolate(eased);

        Some(InterpolatedElement {
            share_id: share_id.to_string(),
            bounds,
            corner_radius,
            transform,
            alpha: 1.0,
        })
    }

    /// Render shared element at specific progress
    pub fn render_shared_element(
        &self,
        share_id: &str,
        progress: f32,
        _canvas: &mut (), // Placeholder for actual canvas type
    ) {
        if let Some(_interpolated) = self.interpolate_state(share_id, progress) {
            // Actual rendering logic would go here
            // Draw snapshot at interpolated bounds with transformed
        }
    }

    /// Clear all shared elements
    pub fn clear(&mut self) {
        self.elements.clear();
    }
}

/// Interpolated element for rendering
#[derive(Debug, Clone)]
pub struct InterpolatedElement {
    pub share_id: String,
    pub bounds: Rect,
    pub corner_radius: f32,
    pub transform: Transform,
    pub alpha: f32,
}

/// Interpolator for shared element properties
pub struct SharedElementInterpolator {
    pub from_rect: Rect,
    pub to_rect: Rect,
    pub from_corner_radius: f32,
    pub to_corner_radius: f32,
    pub from_transform: Transform,
    pub to_transform: Transform,
}

impl SharedElementInterpolator {
    pub fn interpolate(&self, progress: f32) -> (Rect, f32, Transform) {
        (
            self.interpolate_rect(progress),
            lerp(self.from_corner_radius, self.to_corner_radius, progress),
            self.interpolate_transform(progress),
        )
    }

    fn interpolate_rect(&self, progress: f32) -> Rect {
        Rect {
            x: lerp(self.from_rect.x, self.to_rect.x, progress),
            y: lerp(self.from_rect.y, self.to_rect.y, progress),
            width: lerp(self.from_rect.width, self.to_rect.width, progress),
            height: lerp(self.from_rect.height, self.to_rect.height, progress),
        }
    }

    fn interpolate_transform(&self, progress: f32) -> Transform {
        Transform {
            translation: Point::new(
                lerp(
                    self.from_transform.translation.x,
                    self.to_transform.translation.x,
                    progress,
                ),
                lerp(
                    self.from_transform.translation.y,
                    self.to_transform.translation.y,
                    progress,
                ),
            ),
            scale: Point::new(
                lerp(self.from_transform.scale.x, self.to_transform.scale.x, progress),
                lerp(self.from_transform.scale.y, self.to_transform.scale.y, progress),
            ),
            rotation: lerp(
                self.from_transform.rotation,
                self.to_transform.rotation,
                progress,
            ),
        }
    }
}

/// Simple linear interpolation
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Spring approximation for easing
fn spring_approximation(t: f32, damping: f32) -> f32 {
    // A simple spring-like easing function
    let amplitude = 1.0;
    let period = 0.3;
    let shift = (t - 0.5) * 2.0;
    let decay = damping * t;

    let sine = ((t - 0.5) * (std::f32::consts::PI * 2.0) / period).sin();
    let spring = amplitude * (-decay).exp() * sine;

    t + spring * (1.0 - t)
}
