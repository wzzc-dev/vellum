// MoonBit GUI Framework - Transition & Shared Element Demo
// Demonstrates navigation transitions and shared element animations

pub fn main() {
    println!("🌙 MoonBit GUI - Transitions Demo");
    println!("==============================");
    println!();
    println!("📱 Navigation transitions:");
    println!("   - Push / Pop");
    println!("   - Modal presentation");
    println!("   - Flip");
    println!("   - Shared element animations");
    println!();
    println!("Try them in a running app!");
    println!();
    println!("Useful transition helpers:");
    println!("  - default_push_config()");
    println!("  - default_modal_config()");
    println!("  - fade_transition()");
    println!("  - slide_from_right()");
    println!("  - scale_transition(0.5)");
    println!();
}

pub fn on_transition(
    from_screen: String, 
    to_screen: String, 
    progress: Float, 
    is_complete: Bool
) {
    if is_complete {
        println!("✓ Transition complete: {} → {}", from_screen, to_screen);
    } else {
        println!("⟳ Transition in progress: {} → {}", from_screen, to_screen);
        println!("   Progress: {:.0}%", progress * 100.0);
    }
}
