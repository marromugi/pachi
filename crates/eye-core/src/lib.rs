pub mod animation;
pub mod outline;
pub mod renderer;

#[cfg(feature = "gui")]
pub mod gui;

pub use animation::BlinkAnimation;
pub use outline::{BezierAnchor, BezierOutline, EyelashShape, EyeShape, EyebrowShape};
pub use renderer::{EyePairUniforms, EyeRenderer, EyeUniforms};
