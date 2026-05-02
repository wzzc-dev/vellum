// MoonBit GUI Framework - Gesture Demo
// Comprehensive gesture recognition demonstration
// Demonstrates all gesture types: tap, pan, swipe, pinch, rotation, screen edge

pub fn main() {
    println!("🌙 MoonBit GUI - Gesture Demo");
    println!("=======================");
    println!();
    
    // Gesture types available:");
    println!("   - Tap & Double Tap");
    println!("   - Pan & Drag");
    println!("   - Swipe (4 directions)");
    println!("   - Pinch to zoom");
    println!("   - Rotation");
    println!("   - Screen edge pan");
    println!();
    println!("Try them in a running app!");
    println!("You can use these helper functions:");
    println!();
    println!("single_tap_gesture()");
    println!("double_tap_gesture()");
    println!("pan_gesture(5.0)");
    println!("swipe_right(20.0)");
    println!("pinch_gesture()");
    println!("rotation_gesture()");
    println!("left_edge_pan()");
    println!();
}

// Example of creating gesture handler
pub fn on_gesture(
    gesture_type: GestureType, 
    state: GestureState, 
    result: GestureResult
) {
    match (gesture_type {
        GestureType::Tap { .. } => {
            match state {
                GestureState::Ended => {
                    println!("✓ Tap gesture ended!");
                }
                _ => { }
            }
        }
        
        GestureType::Pan { .. } => {
            match state {
                GestureState::Began => {
                    println!("→ Pan began at: {}, {}", 
                        result.translation.x, result.translation.y);
                }
                GestureState::Changed => {
                    println!("→ Pan moved! Translation: {}, {}", 
                        result.translation.x, result.translation.y);
                }
                _ => { }
            }
        }
        
        GestureType::Pinch => {
            if let Some(scale) = result.scale {
                println!("🔍 Pinch! Scale: {:.2}", scale);
            }
        }
        
        GestureType::Rotation => {
            if let Some(rotation) = result.rotation {
                println!("🔄 Rotation! Angle: {:.2} degrees", rotation.to_degrees());
            }
        }
        
        _ => {
            println!("✨ Gesture event received!");
        }
    }
}
