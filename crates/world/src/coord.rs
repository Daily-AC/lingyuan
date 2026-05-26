use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileCoord {
    pub x: i16,
    pub y: i16,
}

impl TileCoord {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
    pub fn manhattan(self, other: TileCoord) -> u16 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as u16
    }
    pub fn step(self, dir: Direction) -> TileCoord {
        let (dx, dy) = dir.delta();
        TileCoord::new(self.x + dx, self.y + dy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ];
    pub fn delta(self) -> (i16, i16) {
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manhattan_basic() {
        assert_eq!(TileCoord::new(0, 0).manhattan(TileCoord::new(3, 4)), 7);
    }

    #[test]
    fn step_north_decreases_y() {
        assert_eq!(
            TileCoord::new(5, 5).step(Direction::North),
            TileCoord::new(5, 4)
        );
    }
}
