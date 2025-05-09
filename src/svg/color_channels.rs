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
        match self {
            ColorChannels::Bgr => {
                triple.swap(0, 2);
            }
            ColorChannels::Brg => {
                triple.swap(0, 1); // grb
                triple.swap(0, 2);
            }
            ColorChannels::Gbr => {
                triple.swap(0, 1); // grb
                triple.swap(1, 2);
            }
            ColorChannels::Grb => {
                triple.swap(0, 1);
            }
            ColorChannels::Rbg => {
                triple.swap(1, 2);
            }
            ColorChannels::Rgb => {}
        }
    }
}
