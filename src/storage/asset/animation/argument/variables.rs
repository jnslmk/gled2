use serde::{Deserialize, Serialize};
use std::ops::AddAssign;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VariablesCount {
    pub u32: usize,
    pub f32: usize,
}

impl AddAssign for VariablesCount {
    fn add_assign(&mut self, rhs: Self) {
        self.u32 += rhs.u32;
        self.f32 += rhs.f32;
    }
}

impl VariablesCount {
    pub fn possible(&self) -> bool {
        self.u32 <= 3 && self.f32 <= 6
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Variables {
    U32,
    F32,
    Vec2F32,
}

impl Variables {
    pub fn count(&self) -> VariablesCount {
        match self {
            Self::U32 => VariablesCount { u32: 1, f32: 0 },
            Self::F32 => VariablesCount { u32: 0, f32: 1 },
            Self::Vec2F32 => VariablesCount { u32: 0, f32: 2 },
        }
    }

    pub fn function(&self, name: &str) -> String {
        match self {
            Self::U32 => format!("fn {name}() -> u32;"),
            Self::F32 => format!("fn {name}() -> f32;"),
            Self::Vec2F32 => format!("fn {name}() -> vec2<f32>;"),
        }
    }

    pub fn shader_code_for_getter(&self, name: &str, count: VariablesCount) -> String {
        match self {
            Self::U32 => format!(
                r#"
                    fn {name}() -> u32 {{
                        return uniforms.u32_{};
                    }}
                "#,
                count.u32,
            ),
            Self::F32 => format!(
                r#"
                    fn {name}() -> f32 {{
                        return uniforms.f32_{};
                    }}
                "#,
                count.f32
            ),
            Self::Vec2F32 => {
                format!(
                    r#"
                        fn {name}() -> vec2<f32> {{
                            return vec2<f32>(uniforms.f32_{}, uniforms.f32_{});
                        }}
                    "#,
                    count.f32,
                    count.f32 + 1
                )
            }
        }
    }
}
