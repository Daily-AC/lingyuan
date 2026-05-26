use crate::coord::TileCoord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid<T> {
    pub width: u16,
    pub height: u16,
    cells: Vec<T>,
}

impl<T: Clone> Grid<T> {
    pub fn filled(width: u16, height: u16, value: T) -> Self {
        Self {
            width,
            height,
            cells: vec![value; (width as usize) * (height as usize)],
        }
    }

    fn index(&self, c: TileCoord) -> Option<usize> {
        if c.x < 0 || c.y < 0 || c.x >= self.width as i16 || c.y >= self.height as i16 {
            return None;
        }
        Some(c.y as usize * self.width as usize + c.x as usize)
    }

    pub fn in_bounds(&self, c: TileCoord) -> bool {
        self.index(c).is_some()
    }

    pub fn get(&self, c: TileCoord) -> Option<&T> {
        self.index(c).map(|i| &self.cells[i])
    }

    pub fn set(&mut self, c: TileCoord, v: T) -> bool {
        if let Some(i) = self.index(c) {
            self.cells[i] = v;
            true
        } else {
            false
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (TileCoord, &T)> + '_ {
        let w = self.width as i16;
        self.cells.iter().enumerate().map(move |(i, v)| {
            let x = (i as i16) % w;
            let y = (i as i16) / w;
            (TileCoord::new(x, y), v)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::TileCoord;

    #[test]
    fn out_of_bounds_returns_none() {
        let g = Grid::filled(10, 10, 0u8);
        assert!(g.get(TileCoord::new(-1, 5)).is_none());
        assert!(g.get(TileCoord::new(5, 10)).is_none());
        assert!(g.get(TileCoord::new(0, 0)).is_some());
    }

    #[test]
    fn set_then_get_roundtrip() {
        let mut g = Grid::filled(5, 5, 0u8);
        g.set(TileCoord::new(2, 3), 7);
        assert_eq!(g.get(TileCoord::new(2, 3)), Some(&7));
    }

    #[test]
    fn iter_visits_all_cells_with_coords() {
        let g = Grid::filled(3, 2, 1u8);
        let cells: Vec<_> = g.iter().collect();
        assert_eq!(cells.len(), 6);
        assert!(cells.iter().any(|(c, _)| *c == TileCoord::new(0, 0)));
        assert!(cells.iter().any(|(c, _)| *c == TileCoord::new(2, 1)));
    }
}
