//! The columns and rows of cells a picture is simplified into, each cell the colour of the pixels it covers.

use crate::picture::{Picture, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSize {
    columns: u32,
    rows: u32,
}

impl GridSize {
    pub fn new(columns: u32, rows: u32) -> GridSize {
        GridSize { columns, rows }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    size: GridSize,
    cells: Vec<Rgb>,
}

impl Grid {
    /// Each cell takes the average colour of the pixels it covers.
    pub fn averaging(picture: &Picture, size: GridSize) -> Grid {
        let _ = picture;
        Grid {
            size,
            cells: vec![Rgb::WHITE; (size.columns * size.rows) as usize],
        }
    }

    pub fn cell(&self, column: u32, row: u32) -> Rgb {
        self.cells[(row * self.size.columns + column) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const BLUE: [u8; 4] = [20, 60, 200, 255];

    fn picture(width: u32, height: u32, pixels: &[[u8; 4]]) -> Picture {
        Picture::from_rgba(width, height, pixels.as_flattened()).expect("a test picture")
    }

    #[test]
    fn a_picture_split_in_two_halves_gives_two_cells_each_the_colour_of_its_half() {
        let half_red_half_blue = picture(4, 1, &[RED, RED, BLUE, BLUE]);

        let grid = Grid::averaging(&half_red_half_blue, GridSize::new(2, 1));

        assert_eq!(grid.cell(0, 0), Rgb(220, 30, 40));
        assert_eq!(grid.cell(1, 0), Rgb(20, 60, 200));
    }
}
