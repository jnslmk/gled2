//! Renders to a texture

mod blob;
mod gradient;
mod opposing_lines;
mod random_circles;
mod renderer;
mod rotating_square;
mod set_center;
mod spiral;
mod stripes;

use gled_proc_macros::Animation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::{Display, EnumIter};

pub use blob::Blob;
pub use gradient::Gradient;
pub use opposing_lines::OpposingLines;
pub use random_circles::RandomCircles;
pub use renderer::AnimationRenderer;
pub use rotating_square::RotatingSquare;
pub use spiral::Spiral;
pub use stripes::Stripes;
