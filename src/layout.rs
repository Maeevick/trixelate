//! Where everything lies on the sheet's paper, in dots at 300 dots per inch.

use crate::colouring::Colouring;
use crate::lettering;

/// The paper a sheet is drawn for, upright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFormat {
    A3,
    A4,
    A5,
}

impl PageFormat {
    fn millimetres(self) -> (u32, u32) {
        match self {
            PageFormat::A3 => (297, 420),
            PageFormat::A4 => (210, 297),
            PageFormat::A5 => (148, 210),
        }
    }
}

const DOTS_PER_INCH: u32 = 300;
const TENTHS_OF_MILLIMETRE_PER_INCH: u32 = 254;

fn dots(millimetres: u32) -> u32 {
    (millimetres * DOTS_PER_INCH * 10 + TENTHS_OF_MILLIMETRE_PER_INCH / 2)
        / TENTHS_OF_MILLIMETRE_PER_INCH
}

/// The blank paper around the sheet's content, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Margin(pub u32);

impl Margin {
    pub const AROUND_A_SHEET: Margin = Margin(20);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRoom {
    InsideTheMargin,
    ForTheNumbersInTheCells,
    ForTheNumbersInTheLegend,
}

/// A square on the page, from its top left corner, its side in dots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Square {
    pub left: u32,
    pub top: u32,
    pub side: u32,
}

/// A disc on the page, from its centre, its diameter in dots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disc {
    pub centre_x: u32,
    pub centre_y: u32,
    pub diameter: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    page_width: u32,
    page_height: u32,
    grid_left: u32,
    grid_top: u32,
    cell_side: u32,
    line: u32,
    cell_number_scale: u32,
    bubbles: Vec<Disc>,
    bubble_number_scale: u32,
}

const GAP_ABOVE_THE_LEGEND_MM: u32 = 10;
const WIDEST_BUBBLE_MM: u32 = 15;
const LINES_PER_CELL_SIDE: u32 = 16;

impl Layout {
    /// Lays out a colouring's grid and legend on the page, inside its margin.
    pub fn of(colouring: &Colouring, format: PageFormat, margin: Margin) -> Result<Layout, NoRoom> {
        let (width, height) = format.millimetres();
        let (page_width, page_height) = (dots(width), dots(height));
        let margin = dots(margin.0);
        let room_inside = |side: u32| side.checked_sub(2 * margin).filter(|&room| room > 0);
        let content_width = room_inside(page_width).ok_or(NoRoom::InsideTheMargin)?;
        let content_height = room_inside(page_height).ok_or(NoRoom::InsideTheMargin)?;
        let colours = colouring.palette().colours().len() as u32;
        let widest_bubble = dots(WIDEST_BUBBLE_MM).min(4 * content_width / (5 * colours - 1));
        let bubble = widest_bubble - (1 - widest_bubble % 2);
        let grid_room = content_height
            .checked_sub(dots(GAP_ABOVE_THE_LEGEND_MM) + bubble)
            .ok_or(NoRoom::InsideTheMargin)?;

        let size = colouring.size();
        let roomiest_side = (content_width / size.columns()).min(grid_room / size.rows());
        let line = (roomiest_side / LINES_PER_CELL_SIDE).max(1);
        let cell_side = (content_width.saturating_sub(line) / size.columns())
            .min(grid_room.saturating_sub(line) / size.rows());
        let widest_number = colours.to_string();
        let inside_a_cell = cell_side.saturating_sub(line);
        let cell_number_scale = lettering::largest_scale_fitting(
            &widest_number,
            inside_a_cell * 3 / 5,
            inside_a_cell / 2,
        )
        .ok_or(NoRoom::ForTheNumbersInTheCells)?;
        let bubble_number_scale =
            lettering::largest_scale_fitting(&widest_number, bubble * 11 / 20, bubble * 9 / 20)
                .ok_or(NoRoom::ForTheNumbersInTheLegend)?;
        let grid_width = cell_side * size.columns() + line;
        let grid_height = cell_side * size.rows() + line;

        let spacing = bubble / 4;
        let row_width = colours * bubble + (colours - 1) * spacing;
        let row_left = margin + (content_width - row_width) / 2;
        let bubbles = (0..colours)
            .map(|index| Disc {
                centre_x: row_left + index * (bubble + spacing) + bubble / 2,
                centre_y: page_height - margin - 1 - bubble / 2,
                diameter: bubble,
            })
            .collect();

        Ok(Layout {
            page_width,
            page_height,
            bubbles,
            grid_left: margin + (content_width - grid_width) / 2,
            grid_top: margin + (grid_room - grid_height) / 2,
            cell_side,
            line,
            cell_number_scale,
            bubble_number_scale,
        })
    }

    pub fn page_width(&self) -> u32 {
        self.page_width
    }

    pub fn page_height(&self) -> u32 {
        self.page_height
    }

    /// A cell's square, its outline's top and left lines included.
    pub fn cell(&self, column: u32, row: u32) -> Square {
        Square {
            left: self.grid_left + column * self.cell_side,
            top: self.grid_top + row * self.cell_side,
            side: self.cell_side,
        }
    }

    /// The legend's bubbles, in the order of their colour numbers.
    pub fn bubbles(&self) -> &[Disc] {
        &self.bubbles
    }

    /// The scale every cell's number is written at.
    pub fn cell_number_scale(&self) -> u32 {
        self.cell_number_scale
    }

    /// How thick the lines outlining the cells are, in dots.
    pub fn line(&self) -> u32 {
        self.line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridSize;
    use crate::palette::ColourCount;
    use crate::picture::Picture;

    fn colouring(columns: u32, rows: u32, colours: usize) -> Colouring {
        let rainbow: Vec<u8> = (0..columns * rows)
            .flat_map(|cell| {
                let shade = (cell * 37 % 256) as u8;
                [shade, 255 - shade, shade / 2, 255]
            })
            .collect();
        let picture = Picture::from_rgba(columns, rows, &rainbow).expect("a picture");
        Colouring::of(
            &picture,
            GridSize::new(columns, rows).expect("a grid"),
            ColourCount::new(colours).expect("a count"),
        )
    }

    #[test]
    fn a_page_counts_its_millimetres_in_dots_at_300_dots_per_inch() {
        let small = colouring(4, 4, 2);

        let pages = [PageFormat::A3, PageFormat::A4, PageFormat::A5]
            .map(|format| Layout::of(&small, format, Margin::AROUND_A_SHEET))
            .map(|layout| layout.map(|layout| (layout.page_width(), layout.page_height())));

        assert_eq!(
            pages,
            [Ok((3508, 4961)), Ok((2480, 3508)), Ok((1748, 2480))]
        );
    }

    #[test]
    fn a_wide_grid_spans_the_page_inside_its_margin_with_square_cells_centred_across()
    -> Result<(), NoRoom> {
        let ten_by_five = colouring(10, 5, 2);
        let margin = 236;

        let layout = Layout::of(&ten_by_five, PageFormat::A4, Margin::AROUND_A_SHEET)?;

        let first = layout.cell(0, 0);
        let next_across = layout.cell(1, 0);
        let next_down = layout.cell(0, 1);
        let last = layout.cell(9, 4);
        let right_edge = last.left + last.side + layout.line();
        assert!(first.side > 0 && layout.line() > 0);
        assert_eq!(
            (next_across.left - first.left, next_down.top - first.top),
            (first.side, first.side)
        );
        assert!(first.left >= margin && first.top >= margin && right_edge <= 2480 - margin);
        assert!((first.left - margin).abs_diff(2480 - margin - right_edge) <= 1);
        assert!(2480 - 2 * margin - (right_edge - first.left) < 10);
        Ok(())
    }

    #[test]
    fn the_legend_lines_up_one_bubble_per_colour_on_the_bottom_margins_edge_below_a_tall_grid()
    -> Result<(), NoRoom> {
        let tall = colouring(5, 10, 12);
        let margin = 236;

        let layout = Layout::of(&tall, PageFormat::A4, Margin::AROUND_A_SHEET)?;

        let bubbles = layout.bubbles();
        assert_eq!(bubbles.len(), 12);
        let last_cell = layout.cell(4, 9);
        let grid_bottom = last_cell.top + last_cell.side + layout.line();
        let first = bubbles[0];
        let last = bubbles[bubbles.len() - 1];
        let (row_left, row_right) = (
            first.centre_x - first.diameter / 2,
            last.centre_x + last.diameter / 2,
        );
        assert_eq!(bubbles.len(), 12);
        assert!(
            bubbles.iter().all(
                |bubble| bubble.diameter == first.diameter && bubble.centre_y == first.centre_y
            )
        );
        assert!(
            bubbles
                .windows(2)
                .all(|pair| pair[1].centre_x - pair[0].centre_x > first.diameter)
        );
        assert!(row_left >= margin && row_right < 2480 - margin);
        assert!((row_left - margin).abs_diff(2480 - margin - 1 - row_right) <= 1);
        assert_eq!(first.diameter % 2, 1, "a bubble has a centre dot");
        assert_eq!(first.centre_y + first.diameter / 2, 3508 - margin - 1);
        assert!(grid_bottom < first.centre_y - first.diameter / 2);
        Ok(())
    }

    #[test]
    fn a_margin_leaving_no_room_for_the_grid_and_its_legend_is_refused() {
        let small = colouring(4, 4, 2);

        let layouts =
            [Margin(74), Margin(75)].map(|margin| Layout::of(&small, PageFormat::A5, margin));

        assert_eq!(
            layouts,
            [Err(NoRoom::InsideTheMargin), Err(NoRoom::InsideTheMargin)]
        );
    }

    #[test]
    fn numbers_too_big_for_their_cells_or_their_bubbles_are_refused_even_on_a_sliver_of_paper() {
        let three_hundred_columns = colouring(300, 2, 2);
        let thirty_colours = colouring(10, 5, 30);
        let one_column_of_thirty_colours = colouring(1, 30, 30);

        let layouts = [
            Layout::of(
                &three_hundred_columns,
                PageFormat::A5,
                Margin::AROUND_A_SHEET,
            ),
            Layout::of(&thirty_colours, PageFormat::A5, Margin(60)),
            Layout::of(&one_column_of_thirty_colours, PageFormat::A5, Margin(73)),
        ];

        assert_eq!(
            layouts.map(|layout| layout.err()),
            [
                Some(NoRoom::ForTheNumbersInTheCells),
                Some(NoRoom::ForTheNumbersInTheLegend),
                Some(NoRoom::ForTheNumbersInTheLegend)
            ]
        );
    }
}
