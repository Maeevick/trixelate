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
        let palette = Palette::dominant(&grid, colours).used_by(grid.cells());
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

    pub fn size(&self) -> GridSize {
        self.size
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Each cell's column, row and colour number, row by row from the top left.
    pub fn numbered_cells(&self) -> impl Iterator<Item = (u32, u32, ColourNumber)> + '_ {
        let columns = self.size.columns();
        (0..)
            .zip(&self.numbers)
            .map(move |(index, &number)| (index % columns, index / columns, number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::picture::Rgb;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const BLUE: [u8; 4] = [20, 60, 200, 255];

    #[test]
    fn each_cell_carries_the_number_of_its_colour_in_the_pictures_palette() {
        let red_red_over_blue_red =
            Picture::from_rgba(2, 2, [RED, RED, BLUE, RED].as_flattened()).expect("a picture");
        let two_by_two = GridSize::new(2, 2).expect("a grid");
        let two_colours = ColourCount::new(2).expect("a count");

        let colouring = Colouring::of(&red_red_over_blue_red, two_by_two, two_colours);

        let numbers = colouring
            .numbered_cells()
            .map(|(column, row, number)| (column, row, number.to_string()));
        assert_eq!(
            numbers.collect::<Vec<_>>(),
            [
                (0, 0, "1".into()),
                (1, 0, "1".into()),
                (0, 1, "2".into()),
                (1, 1, "1".into())
            ]
        );
    }

    #[test]
    fn every_palette_colour_numbers_at_least_one_cell_the_most_used_first() {
        let cells = [
            [30, 120, 30, 255],
            [120, 255, 60, 255],
            [120, 0, 90, 255],
            [160, 30, 30, 255],
            [120, 255, 120, 255],
            [0, 160, 0, 255],
        ];
        let picture = Picture::from_rgba(6, 1, cells.as_flattened()).expect("a picture");
        let six_columns = GridSize::new(6, 1).expect("a grid");
        let four_colours = ColourCount::new(4).expect("a count");

        let colouring = Colouring::of(&picture, six_columns, four_colours);

        let numbers = colouring
            .numbered_cells()
            .map(|(_, _, number)| number.to_string());
        assert_eq!(
            colouring.palette().colours(),
            [Rgb(30, 120, 30), Rgb(120, 255, 120), Rgb(140, 15, 60)]
        );
        assert_eq!(numbers.collect::<Vec<_>>(), ["1", "2", "3", "3", "2", "1"]);
    }
}
