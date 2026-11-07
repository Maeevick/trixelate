//! The columns and rows of cells a picture is simplified into, each cell the colour of the pixels it covers.

use crate::picture::{Picture, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSize {
    columns: u32,
    rows: u32,
}

impl GridSize {
    pub fn new(columns: u32, rows: u32) -> Result<GridSize, NoCell> {
        (columns > 0 && rows > 0)
            .then_some(GridSize { columns, rows })
            .ok_or(NoCell)
    }

    /// As many columns as asked, and as many rows as keep the picture's proportions with square cells:
    /// the nearest count, and at least one.
    pub fn keeping_the_proportions_of(picture: &Picture, columns: u32) -> Result<GridSize, NoCell> {
        let _ = picture;
        GridSize::new(columns, columns)
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
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
        let cells = (0..size.rows)
            .flat_map(|row| (0..size.columns).map(move |column| (column, row)))
            .map(|(column, row)| {
                let xs = pixels_under(column, size.columns, picture.width());
                let ys = pixels_under(row, size.rows, picture.height());
                average(ys.flat_map(|y| xs.clone().map(move |x| picture.pixel(x, y))))
            })
            .collect();
        Grid { size, cells }
    }

    /// The cells' colours, row after row.
    pub fn cells(&self) -> &[Rgb] {
        &self.cells
    }

    pub fn size(&self) -> GridSize {
        self.size
    }
}

fn pixels_under(cell: u32, cells: u32, pixels: u32) -> std::ops::Range<u32> {
    let first = first_pixel_of(cell, cells, pixels);
    first..first_pixel_of(cell + 1, cells, pixels).max(first + 1)
}

fn average(colours: impl Iterator<Item = Rgb>) -> Rgb {
    let (count, red, green, blue) =
        colours.fold((0, 0, 0, 0), |(count, red, green, blue), Rgb(r, g, b)| {
            (
                count + 1,
                red + u64::from(r),
                green + u64::from(g),
                blue + u64::from(b),
            )
        });
    let mean = |sum: u64| ((sum + count / 2) / count) as u8;
    Rgb(mean(red), mean(green), mean(blue))
}

fn first_pixel_of(cell: u32, cells: u32, pixels: u32) -> u32 {
    (u64::from(cell) * u64::from(pixels) / u64::from(cells)) as u32
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
    fn a_picture_split_in_two_halves_gives_two_cells_each_the_colour_of_its_half()
    -> Result<(), NoCell> {
        let half_red_half_blue = picture(4, 1, &[RED, RED, BLUE, BLUE]);

        let grid = Grid::averaging(&half_red_half_blue, GridSize::new(2, 1)?);

        assert_eq!(grid.cells(), [Rgb(220, 30, 40), Rgb(20, 60, 200)]);
        Ok(())
    }

    #[test]
    fn a_cell_averages_every_pixel_it_covers_rounded_to_the_nearest() -> Result<(), NoCell> {
        let red_shades = picture(
            2,
            2,
            &[
                [200, 0, 0, 255],
                [101, 0, 0, 255],
                [200, 0, 0, 255],
                [101, 0, 0, 255],
            ],
        );

        let grid = Grid::averaging(&red_shades, GridSize::new(1, 1)?);

        assert_eq!(grid.cells(), [Rgb(151, 0, 0)]);
        Ok(())
    }

    #[test]
    fn a_grid_finer_than_its_picture_repeats_the_pixels_over_several_cells() -> Result<(), NoCell> {
        let one_red_pixel = picture(1, 1, &[RED]);

        let grid = Grid::averaging(&one_red_pixel, GridSize::new(2, 2)?);

        assert_eq!(grid.cells(), [Rgb(220, 30, 40); 4]);
        Ok(())
    }

    #[test]
    fn a_grid_as_wide_as_asked_takes_the_rows_that_keep_its_pictures_proportions_at_least_one() {
        let blank = |width: u32, height: u32| {
            Picture::from_rgba(width, height, &vec![255; (width * height * 4) as usize])
                .expect("a test picture")
        };
        let asked = [
            (1000, 1000, 32),
            (4, 3, 8),
            (3, 4, 6),
            (3, 2, 4),
            (1000, 10, 10),
            (4, 3, 0),
        ];

        let sizes = asked.map(|(width, height, columns)| {
            GridSize::keeping_the_proportions_of(&blank(width, height), columns)
                .map(|size| (size.columns(), size.rows()))
        });

        assert_eq!(
            sizes,
            [
                Ok((32, 32)),
                Ok((8, 6)),
                Ok((6, 8)),
                Ok((4, 3)),
                Ok((10, 1)),
                Err(NoCell)
            ]
        );
    }

    #[test]
    fn a_grid_without_column_or_without_row_has_no_cell() {
        let refusals = [GridSize::new(0, 12), GridSize::new(16, 0)];

        assert_eq!(refusals, [Err(NoCell), Err(NoCell)]);
    }
}
