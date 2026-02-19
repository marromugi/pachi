pub mod animation;
pub mod outline;
pub mod renderer;

#[cfg(feature = "gui")]
pub mod gui;

pub use animation::BlinkAnimation;
pub use outline::{BezierAnchor, BezierOutline, EyeShape, EyebrowShape};
pub use renderer::{EyeRenderer, EyeUniforms};
