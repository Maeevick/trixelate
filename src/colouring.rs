//! A grid's cells, each with its colour number, and the palette they are numbered from.

use crate::grid::{Grid, GridSize};
use crate::palette::{ColourCount, ColourNumber, Palette};
use crate::picture::Picture;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colouring {
    size: GridSize,
    numbers: Vec<ColourNumber>,
    palette: Palette,
}

impl Colouring {
    /// Simplifies a picture into a grid, finds its dominant colours and numbers every cell after the nearest.
    pub fn of(picture: &Picture, size: GridSize, colours: ColourCount) -> Colouring {
        let grid = Grid::averaging(picture, size);
        let palette = Palette::dominant(&grid, colours);
        let numbers = grid
            .cells()
            .iter()
            .map(|&cell| palette.number_for(cell))
            .collect();
        Colouring {
            size: grid.size(),
            numbers,
            palette,
        }
    }

    pub fn number_at(&self, column: u32, row: u32) -> Option<ColourNumber> {
        self.numbers
            .get((row * self.size.columns() + column) as usize)
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const BLUE: [u8; 4] = [20, 60, 200, 255];

    #[test]
    fn each_cell_carries_the_number_of_its_colour_in_the_pictures_palette() {
        let red_red_blue =
            Picture::from_rgba(3, 1, [RED, RED, BLUE].as_flattened()).expect("a picture");
        let three_columns = GridSize::new(3, 1).expect("a grid");
        let two_colours = ColourCount::new(2).expect("a count");

        let colouring = Colouring::of(&red_red_blue, three_columns, two_colours);

        let numbers = (0..3).map(|column| {
            colouring
                .number_at(column, 0)
                .map(|number| number.to_string())
        });
        assert_eq!(
            numbers.collect::<Vec<_>>(),
            [Some("1".into()), Some("1".into()), Some("2".into())]
        );
    }
}
