#[cfg(test)]
mod gesture_tests {
    use super::*;
    use std::time::Instant;

    fn create_touch_point(id: u64, x: f32, y: f32) -> TouchPoint {
        TouchPoint {
            id,
            position: Point::new(x, y),
            start_position: Point::new(x, y),
            timestamp_ms: 0,
            force: 1.0,
            radius: 5.0,
        }
    }

    fn create_touch_event(
        event_type: TouchEventType,
        touches: Vec<TouchPoint>,
    ) -> TouchEvent {
        TouchEvent {
            event_type,
            touches,
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn test_tap_gesture_recognition() {
        let mut recognizer = TapGestureRecognizer::new(1, 1);
        
        // Begin touch
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        assert_eq!(recognizer.base.state, GestureState::Possible);
        
        // End touch
        let touch_end = create_touch_event(
            TouchEventType::Ended,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_end);
        
        assert_eq!(recognizer.base.state, GestureState::Ended);
        assert!(recognizer.get_result().is_some());
    }

    #[test]
    fn test_pan_gesture_movement() {
        let mut recognizer = PanGestureRecognizer::new(5.0, None);
        
        // Begin touch
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        assert_eq!(recognizer.base.state, GestureState::Possible);
        
        // Move beyond threshold
        let touch_move = create_touch_event(
            TouchEventType::Moved,
            vec![create_touch_point(1, 110.0, 100.0)],
        );
        recognizer.handle_touch(&touch_move);
        
        assert_eq!(recognizer.base.state, GestureState::Began);
        
        // Move more
        let touch_move2 = create_touch_event(
            TouchEventType::Moved,
            vec![create_touch_point(1, 150.0, 100.0)],
        );
        recognizer.handle_touch(&touch_move2);
        
        assert_eq!(recognizer.base.state, GestureState::Changed);
        assert_eq!(recognizer.translation.x, 50.0);
    }

    #[test]
    fn test_swipe_right_gesture() {
        let mut recognizer = SwipeGestureRecognizer::new(SwipeDirection::Right, 20.0);
        
        // Begin touch
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        // End touch with right swipe
        let touch_end = create_touch_event(
            TouchEventType::Ended,
            vec![create_touch_point(1, 150.0, 105.0)],
        );
        recognizer.handle_touch(&touch_end);
        
        assert_eq!(recognizer.base.state, GestureState::Ended);
        assert!(recognizer.get_result().is_some());
    }

    #[test]
    fn test_swipe_wrong_direction_fails() {
        let mut recognizer = SwipeGestureRecognizer::new(SwipeDirection::Right, 20.0);
        
        // Begin touch
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        // End touch with left swipe
        let touch_end = create_touch_event(
            TouchEventType::Ended,
            vec![create_touch_point(1, 50.0, 100.0)],
        );
        recognizer.handle_touch(&touch_end);
        
        assert_eq!(recognizer.base.state, GestureState::Failed);
    }

    #[test]
    fn test_long_press_gesture() {
        let mut recognizer = LongPressGestureRecognizer::new(500, 10.0);
        
        // Begin touch
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        // Immediate fail (not enough time)
        let touch_end = create_touch_event(
            TouchEventType::Ended,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        recognizer.handle_touch(&touch_end);
        
        assert_eq!(recognizer.base.state, GestureState::Failed);
    }

    #[test]
    fn test_pinch_gesture_two_touches() {
        let mut recognizer = PinchGestureRecognizer::new();
        
        // Begin two touches
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![
                create_touch_point(1, 100.0, 100.0),
                create_touch_point(2, 120.0, 100.0),
            ],
        );
        recognizer.handle_touch(&touch_begin);
        
        assert_eq!(recognizer.initial_distance, 20.0);
        
        // Move touches to pinch in
        let touch_move = create_touch_event(
            TouchEventType::Moved,
            vec![
                create_touch_point(1, 100.0, 100.0),
                create_touch_point(2, 110.0, 100.0),
            ],
        );
        recognizer.handle_touch(&touch_move);
        
        assert!(recognizer.get_scale() < 1.0);
    }

    #[test]
    fn test_rotation_gesture() {
        let mut recognizer = RotationGestureRecognizer::new();
        
        // Begin two touches
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![
                create_touch_point(1, 100.0, 100.0),
                create_touch_point(2, 120.0, 100.0),
            ],
        );
        recognizer.handle_touch(&touch_begin);
        
        // Move touches to rotate
        let touch_move = create_touch_event(
            TouchEventType::Moved,
            vec![
                create_touch_point(1, 100.0, 100.0),
                create_touch_point(2, 120.0, 120.0),
            ],
        );
        recognizer.handle_touch(&touch_move);
        
        // Should detect rotation
        let result = recognizer.get_result();
        assert!(result.is_some());
        assert!(result.unwrap().rotation.is_some());
    }

    #[test]
    fn test_screen_edge_pan_left_edge() {
        let mut recognizer = ScreenEdgePanGestureRecognizer::new(Edge::Left, 20.0);
        recognizer.set_screen_size(400.0, 800.0);
        
        // Begin touch at left edge
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 10.0, 400.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        assert_eq!(recognizer.base.state, GestureState::Possible);
        
        // Move from edge
        let touch_move = create_touch_event(
            TouchEventType::Moved,
            vec![create_touch_point(1, 50.0, 400.0)],
        );
        recognizer.handle_touch(&touch_move);
        
        assert_eq!(recognizer.base.state, GestureState::Began);
    }

    #[test]
    fn test_screen_edge_pan_wrong_position() {
        let mut recognizer = ScreenEdgePanGestureRecognizer::new(Edge::Left, 20.0);
        recognizer.set_screen_size(400.0, 800.0);
        
        // Begin touch not at left edge
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 400.0)],
        );
        recognizer.handle_touch(&touch_begin);
        
        assert_eq!(recognizer.base.state, GestureState::Failed);
    }

    #[test]
    fn test_gesture_registry() {
        let mut registry = GestureRegistry::new();
        
        let tap = Arc::new(Mutex::new(TapGestureRecognizer::new(1, 1)));
        let pan = Arc::new(Mutex::new(PanGestureRecognizer::new(5.0, None)));
        
        registry.add_recognizer(tap.clone());
        registry.add_recognizer(pan.clone());
        
        assert_eq!(registry.recognizers.len(), 2);
        
        // Dispatch event
        let touch_begin = create_touch_event(
            TouchEventType::Began,
            vec![create_touch_point(1, 100.0, 100.0)],
        );
        registry.dispatch_touch(&touch_begin);
        
        // Verify state
        let tap_lock = tap.lock().unwrap();
        assert_eq!(tap_lock.base.state, GestureState::Possible);
    }
}
