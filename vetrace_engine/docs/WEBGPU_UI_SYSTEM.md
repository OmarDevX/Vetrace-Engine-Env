# WebGPU UI System

A modern, high-performance UI system for the Vetrace Engine that renders using WebGPU shaders instead of EGUI. This system is designed specifically for game UI with advanced visual effects and JSON-based styling.

## 🎯 Key Features

### Advanced Visual Effects
- **Border Radius**: Smooth rounded corners with per-corner control
- **Glass Effects**: Background blur with frosted glass appearance
- **Gradients**: Linear, radial, and conic gradients
- **Shadows**: Drop shadows with blur and spread control
- **Transparency**: Full alpha blending support
- **Animations**: Smooth transitions with easing functions

### JSON-Based Styling
- **Style Assets**: Save and load UI styles from JSON files
- **Style Variants**: Multiple variations of the same base style
- **Hot Reloading**: Automatically reload styles when files change
- **Inheritance**: Override specific properties while keeping base styles

### Flexible Layout System
- **Flexbox**: CSS-like flexible layouts
- **Grid**: Fixed grid layouts
- **Manual**: Absolute positioning
- **Containers**: Nested layout hierarchies

### ECS Integration
- **Component-Based**: Full integration with the engine's ECS
- **Performance**: Efficient queries and updates
- **Modularity**: Mix and match UI components as needed

## 🚀 Quick Start

### 1. Initialize the UI System

```rust
use vetrace_engine::ui::game_ui::{UISystem, UIElement, UIElementType};
use glam::Vec2;

// Create UI system with style asset directory
let mut ui_system = UISystem::new("assets/ui_styles", Vec2::new(1920.0, 1080.0))?;
```

### 2. Create UI Elements

```rust
// Create a button entity
let button_entity = engine.world.spawn();

// Add UI element component with style
let ui_element = UIElement::new(UIElementType::Button)
    .with_style_asset("modern_button".to_string())
    .with_style_variant("primary".to_string());

engine.world.insert(button_entity, ui_element);

// Add button-specific component
let button_component = UIButtonComponent::new("Click Me!".to_string())
    .with_on_click("button_callback".to_string());

engine.world.insert(button_entity, button_component);
```

### 3. Register Event Callbacks

```rust
ui_system.register_callback("button_callback".to_string(), |event| {
    println!("Button clicked! Entity: {:?}", event.entity);
});
```

### 4. Update and Render

```rust
// In your main loop
ui_system.update_styles(&mut engine.world);
ui_system.update_layout(&mut engine.world);
ui_system.update_animations(&mut engine.world, delta_time);
ui_system.render(&engine.world, time, delta_time);
```

## 📁 Style Assets

### JSON Style Format

```json
{
  "name": "Modern Button",
  "version": "1.0.0",
  "description": "A modern button with gradient and effects",
  "style": {
    "background": {
      "color": [0.2, 0.4, 0.8, 1.0],
      "gradient": {
        "gradient_type": "Linear",
        "stops": [
          [0.0, [0.3, 0.5, 0.9, 1.0]],
          [1.0, [0.1, 0.3, 0.7, 1.0]]
        ],
        "angle": 90.0
      }
    },
    "border": {
      "width": 2.0,
      "color": [0.4, 0.6, 1.0, 1.0],
      "radius": {
        "top_left": 8.0,
        "top_right": 8.0,
        "bottom_right": 8.0,
        "bottom_left": 8.0
      }
    },
    "shadow": {
      "enabled": true,
      "offset": [0.0, 2.0],
      "blur_radius": 4.0,
      "color": [0.0, 0.0, 0.0, 0.3]
    },
    "glass": {
      "enabled": false
    },
    "animation": {
      "transition_duration": 0.2,
      "easing": "EaseInOut",
      "hover_animation": {
        "scale": [1.05, 1.05],
        "glow": {
          "color": [0.4, 0.6, 1.0, 0.8],
          "intensity": 0.5,
          "radius": 8.0
        }
      }
    }
  },
  "variants": {
    "danger": {
      "background": {
        "color": [0.8, 0.2, 0.2, 1.0]
      },
      "border": {
        "color": [1.0, 0.4, 0.4, 1.0]
      }
    }
  }
}
```

### Predefined Style Templates

```rust
use vetrace_engine::ui::game_ui::UIStyleTemplates;

// Create predefined styles
let modern_button = UIStyleTemplates::modern_button();
let glass_panel = UIStyleTemplates::glass_panel();
let dark_input = UIStyleTemplates::dark_input_field();
```

## 🎨 Visual Effects

### Glass Effect
```rust
let glass_style = UIStyle {
    glass: GlassStyle {
        enabled: true,
        blur_intensity: 15.0,
        tint: Vec4::new(1.0, 1.0, 1.0, 0.1),
        frost_intensity: 0.2,
        ..Default::default()
    },
    ..Default::default()
};
```

### Gradient Backgrounds
```rust
let gradient_style = UIStyle {
    background: BackgroundStyle {
        gradient: Some(GradientStyle {
            gradient_type: GradientType::Linear,
            stops: vec![
                (0.0, Vec4::new(1.0, 0.0, 0.0, 1.0)), // Red
                (1.0, Vec4::new(0.0, 0.0, 1.0, 1.0)), // Blue
            ],
            angle: 45.0, // Diagonal
            ..Default::default()
        }),
        ..Default::default()
    },
    ..Default::default()
};
```

### Border Radius
```rust
let rounded_style = UIStyle {
    border: BorderStyle {
        radius: BorderRadius {
            top_left: 10.0,
            top_right: 5.0,
            bottom_right: 10.0,
            bottom_left: 5.0,
        },
        ..Default::default()
    },
    ..Default::default()
};
```

## 📐 Layout System

### Flex Layout
```rust
let container = UIContainerComponent::new();
container.set_layout(LayoutType::Flex {
    direction: FlexDirection::Row,
    wrap: false,
    justify: JustifyContent::SpaceBetween,
    align: AlignItems::Center,
});
```

### Grid Layout
```rust
container.set_layout(LayoutType::Grid { columns: 3 });
```

## 🎭 Component Types

### UI Element Types
- **Panel**: Basic rectangular container
- **Text**: Text rendering with font support
- **Button**: Interactive button with click events
- **Image**: Texture/image display
- **Container**: Layout container for other elements

### ECS Components
- **UIElement**: Core component for all UI entities
- **UITextComponent**: Text content and styling
- **UIImageComponent**: Image/texture content
- **UIButtonComponent**: Button behavior and events
- **UIContainerComponent**: Layout and child management
- **UIInputComponent**: Input event handling
- **UIAnimationComponent**: Animation state and queue

## 🔧 Advanced Features

### Custom Animations
```rust
let animation = UIAnimation {
    property: AnimatedProperty::Scale,
    start_value: Vec4::new(1.0, 1.0, 0.0, 0.0),
    end_value: Vec4::new(1.2, 1.2, 0.0, 0.0),
    duration: 0.3,
    easing: EasingFunction::EaseOutBounce,
    looping: false,
    ..Default::default()
};

animation_component.start_animation(animation);
```

### Event Handling
```rust
// Register global callbacks
ui_system.register_callback("my_callback".to_string(), |event| {
    match event.event_type {
        UIInputEvent::Click => println!("Clicked!"),
        UIInputEvent::MouseEnter => println!("Hovered!"),
        _ => {}
    }
});

// Assign to UI elements
let input_component = UIInputComponent::new()
    .clickable()
    .hoverable()
    .with_handler(UIInputEvent::Click, "my_callback".to_string());
```

## 🎯 Performance

- **GPU Rendering**: All effects rendered on GPU using WebGPU shaders
- **Batched Drawing**: Efficient batching of UI elements
- **Cached Layouts**: Layout calculations cached and only updated when needed
- **Minimal CPU Usage**: Most work done on GPU, leaving CPU free for game logic

## 🔄 Separation from EGUI

- **EGUI**: Used only for editor UI (development tools)
- **WebGPU UI**: Used for all in-game UI elements
- **Clear Separation**: No dependencies between the two systems
- **Performance**: Game UI doesn't impact editor performance and vice versa

## 📝 Examples

See `examples/webgpu_ui_demo.rs` for a complete working example demonstrating all features of the WebGPU UI system.

## 🛠️ Shader Architecture

The UI system uses custom WGSL shaders:
- **Vertex Shader**: `src/ui/game_ui/shaders/ui_vertex.wgsl`
- **Fragment Shader**: `src/ui/game_ui/shaders/ui_fragment.wgsl`

These shaders handle all the advanced effects including border radius, glass blur, gradients, and shadows directly on the GPU for maximum performance.
