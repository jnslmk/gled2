use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ColorChannels {
    Bgr,
    Brg,
    Gbr,
    Grb,
    Rbg,
    #[default]
    Rgb,
}

impl ColorChannels {
    pub fn correct(&self, triple: &mut [u8]) {
        let r = triple[0];
        let g = triple[1];
        let b = triple[2];
        match self {
            ColorChannels::Bgr => {
                triple[0] = b;
                triple[2] = r;
            }
            ColorChannels::Brg => {
                triple[0] = g;
                triple[1] = b;
                triple[2] = r;
            }
            ColorChannels::Gbr => {
                triple[0] = g;
                triple[1] = b;
                triple[2] = r;
            }
            ColorChannels::Grb => {
                triple[0] = g;
                triple[1] = r;
            }
            ColorChannels::Rbg => {
                triple[1] = b;
                triple[2] = g;
            }
            ColorChannels::Rgb => {}
        }
    }
}
