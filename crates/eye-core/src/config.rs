use serde::{Deserialize, Serialize};

use crate::outline::{
    BezierAnchor, BezierOutline, EyeShape, EyebrowGuide, EyebrowOutline, EyebrowShape,
    EyelashShape, IrisShape, PupilShape,
};

#[cfg(feature = "gui")]
use crate::gui::{EyeSideState, SectionLink, Side};

// ============================================================
// Serializable config types
// ============================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyeConfig {
    pub version: u32,
    pub left: EyeSideConfig,
    pub right: EyeSideConfig,
    pub global: GlobalConfig,
    pub links: LinkConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyeSideConfig {
    // Colors
    pub sclera_color: [f32; 3],
    pub iris_color: [f32; 3],
    pub pupil_color: [f32; 3],

    // Scalar parameters (from uniforms)
    pub eyelid_close: f32,
    pub iris_radius: f32,
    pub iris_follow: f32,
    pub pupil_radius: f32,
    pub highlight_offset: [f32; 2],
    pub highlight_radius: f32,
    pub highlight_intensity: f32,
    pub look_x: f32,
    pub look_y: f32,

    // Shapes
    pub eye_shape: EyeShapeConfig,
    pub eyebrow_shape: EyebrowShapeConfig,
    pub eyelash_shape: EyelashShapeConfig,
    pub iris_shape: BezierOutlineConfig,
    pub pupil_shape: BezierOutlineConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyeShapeConfig {
    pub open: BezierOutlineConfig,
    pub closed: BezierOutlineConfig,
    pub close_arch: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BezierOutlineConfig {
    pub anchors: [BezierAnchorConfig; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BezierAnchorConfig {
    pub position: [f32; 2],
    pub handle_in: [f32; 2],
    pub handle_out: [f32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyebrowShapeConfig {
    pub outline: EyebrowOutlineConfig,
    #[serde(default = "default_eyebrow_thickness")]
    pub thickness: [f32; 3],
    #[serde(default = "default_tip_round")]
    pub tip_round: [bool; 2],
    pub base_y: f32,
    pub follow: f32,
    pub color: [f32; 3],
}

fn default_eyebrow_thickness() -> [f32; 3] {
    [0.004, 0.031, 0.004]
}

fn default_tip_round() -> [bool; 2] {
    [true, true]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyebrowOutlineConfig {
    pub anchors: [BezierAnchorConfig; 6],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EyelashShapeConfig {
    pub color: [f32; 3],
    pub thickness: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub bg_color: [f32; 3],
    pub eye_separation: f32,
    pub max_angle: f32,
    pub eye_angle: f32,
    pub focus_distance: f32,
    pub auto_blink: bool,
    pub follow_mouse: bool,
    pub show_highlight: bool,
    pub show_eyebrow: bool,
    pub show_eyelash: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkConfig {
    pub shape: SectionLinkConfig,
    pub iris: SectionLinkConfig,
    pub eyebrow: SectionLinkConfig,
    pub eyelash: SectionLinkConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionLinkConfig {
    pub linked: bool,
    pub active: String,
}

// ============================================================
// Conversions: runtime types → config types
// ============================================================

impl From<&BezierAnchor> for BezierAnchorConfig {
    fn from(a: &BezierAnchor) -> Self {
        Self {
            position: a.position,
            handle_in: a.handle_in,
            handle_out: a.handle_out,
        }
    }
}

impl From<&BezierAnchorConfig> for BezierAnchor {
    fn from(c: &BezierAnchorConfig) -> Self {
        Self {
            position: c.position,
            handle_in: c.handle_in,
            handle_out: c.handle_out,
        }
    }
}

impl From<&BezierOutline> for BezierOutlineConfig {
    fn from(o: &BezierOutline) -> Self {
        Self {
            anchors: [
                BezierAnchorConfig::from(&o.anchors[0]),
                BezierAnchorConfig::from(&o.anchors[1]),
                BezierAnchorConfig::from(&o.anchors[2]),
                BezierAnchorConfig::from(&o.anchors[3]),
            ],
        }
    }
}

impl From<&BezierOutlineConfig> for BezierOutline {
    fn from(c: &BezierOutlineConfig) -> Self {
        Self {
            anchors: [
                BezierAnchor::from(&c.anchors[0]),
                BezierAnchor::from(&c.anchors[1]),
                BezierAnchor::from(&c.anchors[2]),
                BezierAnchor::from(&c.anchors[3]),
            ],
        }
    }
}

impl From<&EyebrowOutline> for EyebrowOutlineConfig {
    fn from(o: &EyebrowOutline) -> Self {
        Self {
            anchors: [
                BezierAnchorConfig::from(&o.anchors[0]),
                BezierAnchorConfig::from(&o.anchors[1]),
                BezierAnchorConfig::from(&o.anchors[2]),
                BezierAnchorConfig::from(&o.anchors[3]),
                BezierAnchorConfig::from(&o.anchors[4]),
                BezierAnchorConfig::from(&o.anchors[5]),
            ],
        }
    }
}

impl From<&EyebrowOutlineConfig> for EyebrowOutline {
    fn from(c: &EyebrowOutlineConfig) -> Self {
        Self {
            anchors: [
                BezierAnchor::from(&c.anchors[0]),
                BezierAnchor::from(&c.anchors[1]),
                BezierAnchor::from(&c.anchors[2]),
                BezierAnchor::from(&c.anchors[3]),
                BezierAnchor::from(&c.anchors[4]),
                BezierAnchor::from(&c.anchors[5]),
            ],
        }
    }
}

impl From<&EyeShape> for EyeShapeConfig {
    fn from(s: &EyeShape) -> Self {
        Self {
            open: BezierOutlineConfig::from(&s.open),
            closed: BezierOutlineConfig::from(&s.closed),
            close_arch: s.close_arch,
        }
    }
}

impl From<&EyeShapeConfig> for EyeShape {
    fn from(c: &EyeShapeConfig) -> Self {
        Self {
            open: BezierOutline::from(&c.open),
            closed: BezierOutline::from(&c.closed),
            close_arch: c.close_arch,
        }
    }
}

impl From<&EyebrowShape> for EyebrowShapeConfig {
    fn from(s: &EyebrowShape) -> Self {
        Self {
            outline: EyebrowOutlineConfig::from(&s.outline),
            thickness: s.thickness,
            tip_round: s.tip_round,
            base_y: s.base_y,
            follow: s.follow,
            color: s.color,
        }
    }
}

impl From<&EyebrowShapeConfig> for EyebrowShape {
    fn from(c: &EyebrowShapeConfig) -> Self {
        let outline = EyebrowOutline::from(&c.outline);
        let guide = EyebrowGuide::from_outline(&outline);
        Self {
            outline,
            guide,
            thickness: c.thickness,
            tip_round: c.tip_round,
            base_y: c.base_y,
            follow: c.follow,
            color: c.color,
        }
    }
}

impl From<&EyelashShape> for EyelashShapeConfig {
    fn from(s: &EyelashShape) -> Self {
        Self {
            color: s.color,
            thickness: s.thickness,
        }
    }
}

impl From<&EyelashShapeConfig> for EyelashShape {
    fn from(c: &EyelashShapeConfig) -> Self {
        Self {
            color: c.color,
            thickness: c.thickness,
        }
    }
}

impl From<&IrisShape> for BezierOutlineConfig {
    fn from(s: &IrisShape) -> Self {
        BezierOutlineConfig::from(&s.outline)
    }
}

impl From<&PupilShape> for BezierOutlineConfig {
    fn from(s: &PupilShape) -> Self {
        BezierOutlineConfig::from(&s.outline)
    }
}

#[cfg(feature = "gui")]
impl From<&SectionLink> for SectionLinkConfig {
    fn from(l: &SectionLink) -> Self {
        Self {
            linked: l.linked,
            active: match l.active {
                Side::Left => "left".to_string(),
                Side::Right => "right".to_string(),
            },
        }
    }
}

#[cfg(feature = "gui")]
impl SectionLinkConfig {
    pub fn to_section_link(&self) -> SectionLink {
        SectionLink {
            linked: self.linked,
            active: if self.active == "right" {
                Side::Right
            } else {
                Side::Left
            },
        }
    }
}

// ============================================================
// EyeSideConfig: per-eye state extraction
// ============================================================

#[cfg(feature = "gui")]
impl From<&EyeSideState> for EyeSideConfig {
    fn from(s: &EyeSideState) -> Self {
        Self {
            sclera_color: s.uniforms.sclera_color,
            iris_color: s.uniforms.iris_color,
            pupil_color: s.uniforms.pupil_color,
            eyelid_close: s.uniforms.eyelid_close,
            iris_radius: s.uniforms.iris_radius,
            iris_follow: s.uniforms.iris_follow,
            pupil_radius: s.uniforms.pupil_radius,
            highlight_offset: s.uniforms.highlight_offset,
            highlight_radius: s.uniforms.highlight_radius,
            highlight_intensity: s.uniforms.highlight_intensity,
            look_x: s.uniforms.look_x,
            look_y: s.uniforms.look_y,
            eye_shape: EyeShapeConfig::from(&s.eye_shape),
            eyebrow_shape: EyebrowShapeConfig::from(&s.eyebrow_shape),
            eyelash_shape: EyelashShapeConfig::from(&s.eyelash_shape),
            iris_shape: BezierOutlineConfig::from(&s.iris_shape),
            pupil_shape: BezierOutlineConfig::from(&s.pupil_shape),
        }
    }
}

#[cfg(feature = "gui")]
impl EyeSideConfig {
    pub fn apply_to(&self, s: &mut EyeSideState) {
        s.uniforms.sclera_color = self.sclera_color;
        s.uniforms.iris_color = self.iris_color;
        s.uniforms.pupil_color = self.pupil_color;
        s.uniforms.eyelid_close = self.eyelid_close;
        s.uniforms.iris_radius = self.iris_radius;
        s.uniforms.iris_follow = self.iris_follow;
        s.uniforms.pupil_radius = self.pupil_radius;
        s.uniforms.highlight_offset = self.highlight_offset;
        s.uniforms.highlight_radius = self.highlight_radius;
        s.uniforms.highlight_intensity = self.highlight_intensity;
        s.uniforms.look_x = self.look_x;
        s.uniforms.look_y = self.look_y;
        s.eye_shape = EyeShape::from(&self.eye_shape);
        s.eyebrow_shape = EyebrowShape::from(&self.eyebrow_shape);
        s.eyelash_shape = EyelashShape::from(&self.eyelash_shape);
        s.iris_shape = IrisShape {
            outline: BezierOutline::from(&self.iris_shape),
        };
        s.pupil_shape = PupilShape {
            outline: BezierOutline::from(&self.pupil_shape),
        };
    }
}

// ============================================================
// EyeConfig: top-level config
// ============================================================

impl EyeConfig {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(feature = "gui")]
impl EyeConfig {
    pub fn from_state(
        left: &EyeSideState,
        right: &EyeSideState,
        link_shape: &SectionLink,
        link_iris: &SectionLink,
        link_eyebrow: &SectionLink,
        link_eyelash: &SectionLink,
        auto_blink: bool,
        follow_mouse: bool,
        show_highlight: bool,
        show_eyebrow: bool,
        show_eyelash: bool,
        focus_distance: f32,
    ) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            left: EyeSideConfig::from(left),
            right: EyeSideConfig::from(right),
            global: GlobalConfig {
                bg_color: left.uniforms.bg_color,
                eye_separation: left.uniforms.eye_separation,
                max_angle: left.uniforms.max_angle,
                eye_angle: left.uniforms.eye_angle,
                focus_distance,
                auto_blink,
                follow_mouse,
                show_highlight,
                show_eyebrow,
                show_eyelash,
            },
            links: LinkConfig {
                shape: SectionLinkConfig::from(link_shape),
                iris: SectionLinkConfig::from(link_iris),
                eyebrow: SectionLinkConfig::from(link_eyebrow),
                eyelash: SectionLinkConfig::from(link_eyelash),
            },
        }
    }

    pub fn apply_to_state(
        &self,
        left: &mut EyeSideState,
        right: &mut EyeSideState,
        link_shape: &mut SectionLink,
        link_iris: &mut SectionLink,
        link_eyebrow: &mut SectionLink,
        link_eyelash: &mut SectionLink,
        auto_blink: &mut bool,
        follow_mouse: &mut bool,
        show_highlight: &mut bool,
        show_eyebrow: &mut bool,
        show_eyelash: &mut bool,
        focus_distance: &mut f32,
    ) {
        // Preserve runtime-only fields
        let aspect = left.uniforms.aspect_ratio;
        let time = left.uniforms.time;

        self.left.apply_to(left);
        self.right.apply_to(right);

        // Restore runtime-only fields
        left.uniforms.aspect_ratio = aspect;
        left.uniforms.time = time;
        right.uniforms.aspect_ratio = aspect;
        right.uniforms.time = time;

        // Global params
        left.uniforms.bg_color = self.global.bg_color;
        left.uniforms.eye_separation = self.global.eye_separation;
        left.uniforms.max_angle = self.global.max_angle;
        left.uniforms.eye_angle = self.global.eye_angle;
        right.uniforms.bg_color = self.global.bg_color;
        right.uniforms.eye_separation = self.global.eye_separation;
        right.uniforms.max_angle = self.global.max_angle;
        right.uniforms.eye_angle = self.global.eye_angle;

        *auto_blink = self.global.auto_blink;
        *follow_mouse = self.global.follow_mouse;
        *show_highlight = self.global.show_highlight;
        *show_eyebrow = self.global.show_eyebrow;
        *show_eyelash = self.global.show_eyelash;
        *focus_distance = self.global.focus_distance;

        // Links
        *link_shape = self.links.shape.to_section_link();
        *link_iris = self.links.iris.to_section_link();
        *link_eyebrow = self.links.eyebrow.to_section_link();
        *link_eyelash = self.links.eyelash.to_section_link();
    }
}
