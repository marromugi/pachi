pub mod outline;
pub mod renderer;

#[cfg(feature = "gui")]
pub mod gui;

pub use outline::{BezierAnchor, BezierOutline, EyeShape};
pub use renderer::{EyeRenderer, EyeUniforms};
