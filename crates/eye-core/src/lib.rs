pub mod animation;
pub mod config;
pub mod nod;
pub mod outline;
pub mod renderer;

#[cfg(feature = "gui")]
pub mod gui;

pub use animation::BlinkAnimation;
pub use config::EyeConfig;
pub use nod::NodAnimation;
pub use outline::{BezierAnchor, BezierOutline, EyelashShape, EyeShape, EyebrowGuide, EyebrowOutline, EyebrowShape, IrisShape, PupilShape};
pub use renderer::{EyePairUniforms, EyeRenderer, EyeUniforms};
