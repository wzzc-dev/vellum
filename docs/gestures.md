# MoonBit GUI 手势识别系统

## 概述

MoonBit GUI 框架提供了完整的手势识别系统，支持多种常用手势类型。

## 支持的手势类型

### 1. Tap（点击）
- 单指点击
- 多指点击
- 可配置点击次数

```moonbit
// 创建手势类型
let single_tap = single_tap_gesture();
let double_tap = double_tap_gesture();
```

### 2. Pan（拖拽/平移）
- 触摸滑动
- 可配置最小识别距离
- 提供当前平移和速度

```moonbit
let pan = pan_gesture(5.0);
```

### 3. Swipe（滑动）
- 支持四个方向
- Right, Left, Up, Down
- 快速滑动识别

```moonbit
let swipe_right = swipe_right(20.0);
let swipe_down = swipe_gesture(SwipeDirection::Down, 15.0);
```

### 4. Long Press（长按）
- 可配置持续时间
- 可配置移动容忍度

```moonbit
let long_press = long_press_gesture(500, 10.0);
```

### 5. Pinch（捏合缩放）
- 双指捏合
- 提供缩放比例

```moonbit
let pinch = pinch_gesture();
```

### 6. Rotation（旋转）
- 双指旋转
- 提供旋转角度

```moonbit
let rotation = rotation_gesture();
```

### 7. Screen Edge Pan（边缘拖拽）
- 屏幕边缘识别
- iOS 风格的滑动返回

```moonbit
let left_edge = left_edge_pan();
let right_edge = right_edge_pan();
```

## 手势状态生命周期

```
Possible → Began → Changed → Ended
           ↓
         Failed/Cancelled
```

- `Possible`: 等待触发
- `Began`: 手势开始识别
- `Changed`: 手势状态更新
- `Ended`: 手势成功完成
- `Failed`: 识别失败
- `Cancelled`: 系统取消

## 使用示例

### MoonBit 扩展中的使用

```moonbit
import from state {
    GestureType,
    GestureState,
    GestureResult,
    single_tap_gesture,
    pan_gesture,
    on_gesture,
}

// 创建手势
let tap = single_tap_gesture();
let pan = pan_gesture(3.0);

// 处理手势事件
pub fn handle_gesture_event(
    gesture_type: GestureType, 
    state: GestureState, 
    result: GestureResult
) {
    match state {
        GestureState::Began => {
            println!("手势开始");
        }
        GestureState::Changed => {
            println!("平移: {}, {}", 
                result.translation.x, result.translation.y);
        }
        GestureState::Ended => {
            println!("手势完成！");
        }
        _ => { }
    }
}
```

## 手势识别结果

```moonbit
pub struct GestureResult {
    pub gesture_type: GestureType,
    pub state: GestureState,
    pub touches: List<TouchPoint>,
    pub velocity: Point,
    pub translation: Point,
    pub scale: Option<Float>,    // Pinch
    pub rotation: Option<Float>, // Rotation
}
```

## 最佳实践

1. **开始时检查**：总是先处理 `Began` 状态
2. **及时更新 UI**：在 `Changed` 状态更新
3. **状态清理**：在 `Ended` 或 `Failed` 清理
4. **注意取消**：处理 `Cancelled` 状态
5. **性能优化**：避免在手势处理中做太复杂的操作

## 错误处理

- 手势识别在无法识别时会进入 `Failed` 状态
- 系统打断时会进入 `Cancelled` 状态
- 建议在这些状态下清理资源并重置 UI

---

更多信息参考 [转场系统文档](./transitions.md)
