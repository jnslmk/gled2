use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "GridLocationString", into = "GridLocationString")]
pub struct GridLocation {
    pub col: usize,
    pub row: usize,
}

impl GridLocation {
    pub fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }
}

#[derive(Serialize, Deserialize)]
struct GridLocationString(String);

impl From<GridLocationString> for GridLocation {
    fn from(value: GridLocationString) -> Self {
        let parts: Vec<&str> = value.0.split(':').collect();
        if parts.len() != 2 {
            return GridLocation::default();
        }
        let col = parts[0].trim().parse::<usize>().unwrap_or(0);
        let row = parts[1].trim().parse::<usize>().unwrap_or(0);
        GridLocation { col, row }
    }
}

impl From<GridLocation> for GridLocationString {
    fn from(value: GridLocation) -> Self {
        GridLocationString(format!("{}:{}", value.col, value.row))
    }
}
