# pachi

A real-time 3D eye renderer built with Rust and wgpu.

https://github.com/user-attachments/assets/05a7ae26-2fb3-4914-93df-e12aed7d2aa3


## Features

- **Bezier outline** - Cubic Bezier curve-based eye shape with morphing between predefined shapes (circle, ellipse, slit)
- **3D perspective** - Sphere-projected iris with gaze-following behavior
- **Blink animation** - Keyframe-driven blink with velocity-based squash & stretch deformation
- **Iris rendering** - Configurable iris with color, radius, and follow intensity
- **Interactive GUI** - egui control panel for real-time parameter tweaking (shape editing, 3D look angle, colors, etc.)

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2021)

### Run

```sh
cargo run --example desktop
```

## Project Structure

```
crates/
  eye-core/     # Core library (rendering, animation, outline, GUI)
examples/
  desktop.rs    # Desktop demo with interactive controls
```

## License

MIT OR Apache-2.0
