//! Animation rendering module for GPUI
//! 
//! This module provides GPUI integration for animation functionality,
//! including tween animations, spring physics, and transitions.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Animation curve types
#[derive(Debug, Clone, Copy)]
pub enum AnimationCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

impl AnimationCurve {
    /// Apply curve to a normalized value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            AnimationCurve::Linear => t,
            AnimationCurve::EaseIn => t * t,
            AnimationCurve::EaseOut => t * (2.0 - t),
            AnimationCurve::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            AnimationCurve::Bounce => {
                // Simple bounce approximation
                let t2 = t * t;
                if t < 0.5 {
                    4.0 * t2 * t2
                } else {
                    let t2 = t - 1.0;
                    1.0 + 4.0 * t2 * t2 * t2
                }
            }
            AnimationCurve::Elastic => {
                // Simple elastic approximation
                if t == 0.0 || t == 1.0 {
                    t
                } else {
                    let p = 0.3;
                    let s = p / 4.0;
                    let sin = ((2.0 * std::f32::consts::PI / p) * (t - 0.5)).sin();
                    -(0.5 * (2.0_f32).powf(-10.0 * t) * sin) + 1.0
                }
            }
        }
    }
}

/// Animation direction
#[derive(Debug, Clone, Copy)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

impl Default for AnimationDirection {
    fn default() -> Self {
        AnimationDirection::Normal
    }
}

/// Animation status
#[derive(Debug, Clone, Copy)]
pub enum AnimationStatus {
    Stopped,
    RunningForward,
    RunningBackward,
    Paused,
}

/// Basic animation specification
#[derive(Debug, Clone)]
pub struct AnimationSpec {
    pub duration_ms: u32,
    pub curve: AnimationCurve,
    pub delay_ms: u32,
    pub repeat: bool,
    pub repeat_count: u32,
    pub direction: AnimationDirection,
}

impl Default for AnimationSpec {
    fn default() -> Self {
        Self {
            duration_ms: 300,
            curve: AnimationCurve::EaseInOut,
            delay_ms: 0,
            repeat: false,
            repeat_count: 0,
            direction: AnimationDirection::Normal,
        }
    }
}

/// Spring animation configuration
#[derive(Debug, Clone)]
pub struct SpringConfig {
    pub damping_ratio: f32,
    pub stiffness: f32,
    pub mass: f32,
    pub initial_velocity: f32,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            damping_ratio: 0.5,
            stiffness: 100.0,
            mass: 1.0,
            initial_velocity: 0.0,
        }
    }
}

impl SpringConfig {
    pub fn bouncy() -> Self {
        Self {
            damping_ratio: 0.3,
            stiffness: 100.0,
            mass: 1.0,
            initial_velocity: 0.0,
        }
    }

    pub fn stiff() -> Self {
        Self {
            damping_ratio: 0.8,
            stiffness: 300.0,
            mass: 1.0,
            initial_velocity: 0.0,
        }
    }
}

/// Animated property
#[derive(Debug, Clone)]
pub struct AnimatedProperty {
    pub name: String,
    pub from: f32,
    pub to: f32,
}

/// Keyframe definition
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub progress: f32,
    pub value: f32,
    pub curve: Option<AnimationCurve>,
}

/// Individual animation state
#[derive(Debug, Clone)]
struct AnimationState {
    pub from: f32,
    pub to: f32,
    pub current: f32,
    pub velocity: f32,
    pub spec: AnimationSpec,
    pub status: AnimationStatus,
    pub start_time: Option<Instant>,
    pub repeat_count: u32,
    // Spring parameters
    pub is_spring: bool,
    pub spring_config: Option<SpringConfig>,
    pub damping_ratio: f32,
    pub omega: f32,
}

impl AnimationState {
    pub fn tween(from: f32, to: f32, spec: AnimationSpec) -> Self {
        Self {
            from,
            to,
            current: from,
            velocity: 0.0,
            spec,
            status: AnimationStatus::Stopped,
            start_time: None,
            repeat_count: 0,
            is_spring: false,
            spring_config: None,
            damping_ratio: 0.0,
            omega: 0.0,
        }
    }

    pub fn spring(from: f32, to: f32, config: SpringConfig) -> Self {
        let damping_ratio = config.damping_ratio;
        let omega = (config.stiffness / config.mass).sqrt();
        
        Self {
            from,
            to,
            current: from,
            velocity: config.initial_velocity,
            spec: AnimationSpec::default(),
            status: AnimationStatus::Stopped,
            start_time: None,
            repeat_count: 0,
            is_spring: true,
            spring_config: Some(config),
            damping_ratio,
            omega,
        }
    }

    pub fn start(&mut self, now: Instant) {
        self.start_time = Some(now);
        self.status = AnimationStatus::RunningForward;
        self.current = self.from;
        self.repeat_count = 0;
    }

    pub fn pause(&mut self) {
        if matches!(self.status, AnimationStatus::RunningForward | AnimationStatus::RunningBackward) {
            self.status = AnimationStatus::Paused;
        }
    }

    pub fn resume(&mut self, now: Instant) {
        if matches!(self.status, AnimationStatus::Paused) {
            self.start_time = Some(now);
            self.status = AnimationStatus::RunningForward;
        }
    }

    pub fn stop(&mut self) {
        self.status = AnimationStatus::Stopped;
        self.current = self.from;
        self.start_time = None;
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, AnimationStatus::RunningForward | AnimationStatus::RunningBackward)
    }
}

/// Animation controller for managing animations
pub struct AnimationController {
    animations: Arc<RwLock<HashMap<String, AnimationState>>>,
    listeners: Arc<RwLock<Vec<Box<dyn Fn(AnimationEvent) + Send + Sync>>>>,
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            animations: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn animate(&self, property: &str, from: f32, to: f32, spec: AnimationSpec) {
        let mut animations = self.animations.write().unwrap();
        
        let state = AnimationState::tween(from, to, spec);
        animations.insert(property.to_string(), state);
        
        self.notify(AnimationEvent::Started { property: property.to_string() });
    }

    pub fn spring_animate(&self, property: &str, to: f32, config: SpringConfig) {
        let mut animations = self.animations.write().unwrap();
        
        // Get current value or default
        let from = animations
            .get(property)
            .map(|s| s.current)
            .unwrap_or(0.0);
        
        let state = AnimationState::spring(from, to, config);
        animations.insert(property.to_string(), state);
        
        self.notify(AnimationEvent::Started { property: property.to_string() });
    }

    pub fn animate_all(&self, properties: Vec<AnimatedProperty>, spec: AnimationSpec) {
        for prop in properties {
            self.animate(&prop.name, prop.from, prop.to, spec.clone());
        }
    }

    pub fn stop(&self, property: &str) {
        let mut animations = self.animations.write().unwrap();
        if let Some(state) = animations.get_mut(property) {
            state.stop();
            self.notify(AnimationEvent::Cancelled { property: property.to_string() });
        }
    }

    pub fn stop_all(&self) {
        let animations = self.animations.write().unwrap();
        let properties: Vec<String> = animations.keys().cloned().collect();
        drop(animations);
        
        for prop in properties {
            self.stop(&prop);
        }
    }

    pub fn pause(&self, property: &str) {
        let mut animations = self.animations.write().unwrap();
        if let Some(state) = animations.get_mut(property) {
            state.pause();
        }
    }

    pub fn resume(&self, property: &str) {
        let mut animations = self.animations.write().unwrap();
        if let Some(state) = animations.get_mut(property) {
            state.resume(Instant::now());
        }
    }

    pub fn is_running(&self, property: &str) -> bool {
        let animations = self.animations.read().unwrap();
        animations.get(property).map(|s| s.is_running()).unwrap_or(false)
    }

    pub fn get_value(&self, property: &str) -> Option<f32> {
        let animations = self.animations.read().unwrap();
        animations.get(property).map(|s| s.current)
    }

    pub fn get_status(&self, property: &str) -> AnimationStatus {
        let animations = self.animations.read().unwrap();
        animations.get(property)
            .map(|s| s.status)
            .unwrap_or(AnimationStatus::Stopped)
    }

    /// Update all animations and return changed properties
    pub fn update(&self, now: Instant) -> Vec<(String, f32)> {
        let mut animations = self.animations.write().unwrap();
        let mut changes = Vec::new();
        
        let mut to_remove: Vec<String> = Vec::new();
        
        for (name, state) in animations.iter_mut() {
            if !state.is_running() {
                continue;
            }
            
            let start_time = match state.start_time {
                Some(t) => t,
                None => continue,
            };
            
            let elapsed = now.duration_since(start_time);
            let delay = Duration::from_millis(state.spec.delay_ms as u64);
            
            if elapsed < delay {
                continue;
            }
            
            let elapsed_since_delay = elapsed - delay;
            let duration = Duration::from_millis(state.spec.duration_ms as u64);
            
            if state.is_spring {
                // Spring animation update
                let (new_value, new_velocity, settled) = self.update_spring(state, elapsed_since_delay);
                state.current = new_value;
                state.velocity = new_velocity;
                
                changes.push((name.clone(), new_value));
                self.notify(AnimationEvent::SpringUpdate {
                    property: name.clone(),
                    value: new_value,
                    velocity: new_velocity,
                });
                
                if settled {
                    state.stop();
                    self.notify(AnimationEvent::SpringSettled {
                        property: name.clone(),
                        value: new_value,
                    });
                    self.notify(AnimationEvent::Completed { property: name.clone() });
                }
            } else {
                // Tween animation update
                let t = (elapsed_since_delay.as_millis() as f32) / (duration.as_millis() as f32);
                let t_clamped = t.min(1.0);
                
                let direction_factor = match state.spec.direction {
                    AnimationDirection::Normal => 1.0,
                    AnimationDirection::Reverse => -1.0,
                    AnimationDirection::Alternate | AnimationDirection::AlternateReverse => {
                        if state.repeat_count % 2 == 0 { 1.0 } else { -1.0 }
                    }
                };
                
                let progress = (t_clamped * direction_factor).abs();
                let curved = state.spec.curve.apply(progress);
                
                let new_value = state.from + (state.to - state.from) * curved;
                state.current = new_value;
                
                changes.push((name.clone(), new_value));
                self.notify(AnimationEvent::ValueChanged {
                    property: name.clone(),
                    value: new_value,
                });
                
                if t >= 1.0 {
                    state.current = state.to;
                    
                    if state.spec.repeat && (state.spec.repeat_count == 0 || state.repeat_count < state.spec.repeat_count) {
                        // Repeat
                        state.repeat_count += 1;
                        state.start_time = Some(now);
                        self.notify(AnimationEvent::Repeated {
                            property: name.clone(),
                            repeat_count: state.repeat_count,
                        });
                    } else {
                        // Complete
                        state.stop();
                        self.notify(AnimationEvent::Completed { property: name.clone() });
                    }
                }
            }
        }
        
        // Remove stopped animations
        for name in to_remove {
            animations.remove(&name);
        }
        
        changes
    }

    fn update_spring(&self, state: &mut AnimationState, elapsed: Duration) -> (f32, f32, bool) {
        let t = elapsed.as_secs_f32();
        let omega = state.omega;
        let zeta = state.damping_ratio;
        
        let displacement = state.to - state.from;
        
        if (zeta - 1.0).abs() < f32::EPSILON {
            // Critically damped
            let c1 = displacement;
            let c2 = state.velocity + omega * displacement;
            let value = displacement * (1.0 + omega * t) * (-omega * t).exp();
            let velocity = value * (-omega) + c2 * (-omega * t).exp();
            let settled = (value - state.to).abs() < 0.001 && velocity.abs() < 0.001;
            (value, velocity, settled)
        } else if zeta < 1.0 {
            // Underdamped
            let omega_d = omega * (1.0 - zeta * zeta).sqrt();
            let value = (-zeta * omega * t).exp() * (
                displacement * (omega_d * t).cos() +
                ((state.velocity + zeta * omega * displacement) / omega_d) * (omega_d * t).sin()
            ) + state.to;
            let velocity = value * (-zeta * omega) + 
                (-zeta * omega * t).exp() * 
                ((state.velocity + zeta * omega * displacement) * (omega_d * t).cos() - 
                 displacement * omega_d * (omega_d * t).sin());
            let settled = (value - state.to).abs() < 0.001 && velocity.abs() < 0.001;
            (value, velocity, settled)
        } else {
            // Overdamped
            let r1 = -omega * (zeta + (zeta * zeta - 1.0).sqrt());
            let r2 = -omega * (zeta - (zeta * zeta - 1.0).sqrt());
            let c2 = (state.velocity - r1 * displacement) / (r2 - r1);
            let c1 = displacement - c2;
            let value = c1 * (r1 * t).exp() + c2 * (r2 * t).exp() + state.to;
            let velocity = c1 * r1 * (r1 * t).exp() + c2 * r2 * (r2 * t).exp();
            let settled = (value - state.to).abs() < 0.001 && velocity.abs() < 0.001;
            (value, velocity, settled)
        }
    }

    pub fn add_listener<F>(&self, callback: F)
    where
        F: Fn(AnimationEvent) + Send + Sync + 'static,
    {
        self.listeners.write().unwrap().push(Box::new(callback));
    }

    fn notify(&self, event: AnimationEvent) {
        let listeners = self.listeners.read().unwrap();
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation event types
#[derive(Debug, Clone)]
pub enum AnimationEvent {
    Started { property: String },
    Completed { property: String },
    Cancelled { property: String },
    ValueChanged { property: String, value: f32 },
    Repeated { property: String, repeat_count: u32 },
    SpringUpdate { property: String, value: f32, velocity: f32 },
    SpringSettled { property: String, value: f32 },
}

/// Page transition types
#[derive(Debug, Clone, Copy)]
pub enum PageTransition {
    Fade { duration_ms: u32 },
    Slide { direction: SlideDirection, duration_ms: u32 },
    Scale { duration_ms: u32 },
    SlideAndFade { direction: SlideDirection, duration_ms: u32 },
}

#[derive(Debug, Clone, Copy)]
pub enum SlideDirection {
    FromLeft,
    FromRight,
    FromTop,
    FromBottom,
}

impl Default for PageTransition {
    fn default() -> Self {
        PageTransition::Fade { duration_ms: 300 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_curve() {
        assert_eq!(AnimationCurve::Linear.apply(0.0), 0.0);
        assert_eq!(AnimationCurve::Linear.apply(1.0), 1.0);
        
        let ease_in = AnimationCurve::EaseIn.apply(0.5);
        assert!(ease_in < 0.5); // Ease-in should be slower at start
    }

    #[test]
    fn test_animation_controller() {
        let controller = AnimationController::new();
        
        controller.animate("opacity", 0.0, 1.0, AnimationSpec::default());
        
        assert!(controller.is_running("opacity"));
        assert_eq!(controller.get_value("opacity"), Some(0.0));
        
        controller.stop("opacity");
        assert!(!controller.is_running("opacity"));
    }

    #[test]
    fn test_spring_animation() {
        let controller = AnimationController::new();
        
        controller.spring_animate("scale", 1.0, SpringConfig::default());
        
        assert!(controller.is_running("scale"));
        
        // Verify spring config is applied
        let state = controller.animations.read().unwrap();
        let spring_state = state.get("scale").unwrap();
        assert!(spring_state.is_spring);
    }
}
