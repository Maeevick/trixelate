//! The page printed for the child: the grid of numbered cells and its legend, drawn where the layout says.

use crate::colouring::Colouring;
use crate::layout::Layout;
use crate::picture::{Picture, Rgb};

const LINE_GREY: Rgb = Rgb(170, 170, 170);

/// Draws a colouring's sheet on white paper, as laid out.
pub fn draw(colouring: &Colouring, layout: &Layout) -> Picture {
    let mut paper = Paper::blank(layout.page_width(), layout.page_height());
    outline_the_cells(&mut paper, colouring, layout);
    paper.into_picture()
}

fn outline_the_cells(paper: &mut Paper, colouring: &Colouring, layout: &Layout) {
    let (columns, rows) = (colouring.size().columns(), colouring.size().rows());
    let top_left = layout.cell(0, 0);
    let bottom_right = layout.cell(columns, rows);
    let grid_width = bottom_right.left - top_left.left + layout.line();
    let grid_height = bottom_right.top - top_left.top + layout.line();
    for column in 0..=columns {
        let left = layout.cell(column, 0).left;
        paper.fill(left, top_left.top, layout.line(), grid_height, LINE_GREY);
    }
    for row in 0..=rows {
        let top = layout.cell(0, row).top;
        paper.fill(top_left.left, top, grid_width, layout.line(), LINE_GREY);
    }
}

struct Paper {
    width: u32,
    height: u32,
    dots: Vec<Rgb>,
}

impl Paper {
    fn blank(width: u32, height: u32) -> Paper {
        Paper {
            width,
            height,
            dots: vec![Rgb::WHITE; width as usize * height as usize],
        }
    }

    fn fill(&mut self, left: u32, top: u32, width: u32, height: u32, colour: Rgb) {
        for y in top..(top + height).min(self.height) {
            for x in left..(left + width).min(self.width) {
                self.dots[(y * self.width + x) as usize] = colour;
            }
        }
    }

    fn into_picture(self) -> Picture {
        Picture::painted(self.width, self.height, self.dots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridSize;
    use crate::layout::{Margin, PageFormat};
    use crate::lettering;
    use crate::palette::ColourCount;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const YELLOW: [u8; 4] = [250, 220, 60, 255];
    const LINE_GREY: Rgb = Rgb(170, 170, 170);
    const NUMBER_GREY: Rgb = Rgb(90, 90, 90);

    fn red_red_yellow() -> (Colouring, Layout) {
        let picture =
            Picture::from_rgba(3, 1, [RED, RED, YELLOW].as_flattened()).expect("a picture");
        let colouring = Colouring::of(
            &picture,
            GridSize::new(3, 1).expect("a grid"),
            ColourCount::new(2).expect("a count"),
        );
        let layout = Layout::of(&colouring, PageFormat::A5, Margin::AROUND_A_SHEET).expect("room");
        (colouring, layout)
    }

    #[test]
    fn a_sheet_fills_its_page_with_its_margin_blank_and_every_cell_outlined() {
        let (colouring, layout) = red_red_yellow();
        let margin = 236;

        let sheet = draw(&colouring, &layout);

        let (width, height) = (layout.page_width(), layout.page_height());
        assert_eq!((sheet.width(), sheet.height()), (width, height));
        let in_the_margin = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                x < margin || y < margin || x >= width - margin || y >= height - margin
            });
        assert!(
            in_the_margin
                .into_iter()
                .all(|(x, y)| sheet.pixel(x, y) == Rgb::WHITE)
        );
        let corners = (0..3).map(|column| layout.cell(column, 0));
        assert!(
            corners
                .into_iter()
                .all(|cell| sheet.pixel(cell.left, cell.top) == LINE_GREY
                    && sheet.pixel(cell.left + cell.side, cell.top + cell.side) == LINE_GREY)
        );
    }

    #[test]
    fn each_cell_shows_its_colour_number_in_dark_grey_centred_inside_its_outline() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout);

        for column in 0..3 {
            let cell = layout.cell(column, 0);
            let (left, top) = (cell.left + layout.line(), cell.top + layout.line());
            let inside = cell.side - layout.line();
            let number = colouring
                .number_at(column, 0)
                .map(|number| number.to_string())
                .unwrap_or_default();
            let (width, height) = lettering::size(&number, layout.cell_number_scale());
            let (number_left, number_top) =
                (left + (inside - width) / 2, top + (inside - height) / 2);
            let mut expected: Vec<(u32, u32)> = lettering::ink(&number, layout.cell_number_scale())
                .into_iter()
                .map(|(x, y)| (number_left + x, number_top + y))
                .collect();
            let mut inked: Vec<(u32, u32)> = (top..top + inside)
                .flat_map(|y| (left..left + inside).map(move |x| (x, y)))
                .filter(|&(x, y)| sheet.pixel(x, y) == NUMBER_GREY)
                .collect();
            expected.sort();
            inked.sort();

            assert_eq!(inked, expected, "the number in cell {column}");
        }
    }
}
