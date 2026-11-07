//! The page printed for the child: the grid of numbered cells and its legend, drawn where the layout says.

use crate::colouring::Colouring;
use crate::layout::{Disc, GridPlace, Layout};
use crate::lettering;
use crate::picture::{Picture, Rgb};

const LINE_GREY: Rgb = Rgb(170, 170, 170);
const NUMBER_GREY: Rgb = Rgb(90, 90, 90);
const BLACK: Rgb = Rgb(0, 0, 0);
const OUTLINES_PER_BUBBLE_DIAMETER: u32 = 24;

/// Whether the legend's bubbles show the picture's colours, with a model in those colours or none,
/// or leave the child free to pick their own, which no model could show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legend {
    Coloured(Model),
    Freedom,
}

/// The coloured-in picture to copy from: none, the full sheet painted, or twinned with the sheet on one page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    None,
    Full,
    Twin,
}

/// Draws a colouring's sheet on white paper, as laid out.
pub fn draw(colouring: &Colouring, layout: &Layout, legend: Legend) -> Picture {
    let mut paper = Paper::blank(layout.page_width(), layout.page_height());
    let cells = match legend {
        Legend::Coloured(Model::Full) => Cells::ColouredIn,
        Legend::Coloured(Model::None | Model::Twin) | Legend::Freedom => Cells::ToColourIn,
    };
    for place in layout.grids() {
        outline_the_cells(&mut paper, colouring, layout, place);
        fill_in_the_cells(&mut paper, colouring, layout, place, cells);
    }
    draw_the_legend(&mut paper, colouring, layout, legend);
    paper.into_picture()
}

fn draw_the_legend(paper: &mut Paper, colouring: &Colouring, layout: &Layout, legend: Legend) {
    for (bubble, (number, colour)) in layout.bubbles().iter().zip(colouring.palette().numbered()) {
        let colour = match legend {
            Legend::Coloured(_) => colour,
            Legend::Freedom => Rgb::WHITE,
        };
        paper.disc(bubble, colour);
        let radius = bubble.diameter / 2;
        paper.write(
            &number.to_string(),
            layout.bubble_number_scale(),
            (bubble.centre_x - radius, bubble.centre_y - radius),
            (bubble.diameter, bubble.diameter),
            readable_on(colour),
        );
    }
}

/// Black on a light colour, white on a dark one, light meaning a luma of half or more, after ITU-R BT.709.
fn readable_on(Rgb(red, green, blue): Rgb) -> Rgb {
    let luma = 2126 * u32::from(red) + 7152 * u32::from(green) + 722 * u32::from(blue);
    if luma >= 10_000 * 128 {
        BLACK
    } else {
        Rgb::WHITE
    }
}

/// Whether a grid's cells wait for the child's colours, or show them already coloured in.
#[derive(Clone, Copy)]
enum Cells {
    ToColourIn,
    ColouredIn,
}

/// Writes each cell's number, in grey on blank cells, or readable on the cell's colour once coloured in.
fn fill_in_the_cells(
    paper: &mut Paper,
    colouring: &Colouring,
    layout: &Layout,
    place: &GridPlace,
    cells: Cells,
) {
    for (column, row, number) in colouring.numbered_cells() {
        let cell = place.cell(column, row);
        let (left, top) = (cell.left + layout.line(), cell.top + layout.line());
        let inside = cell.side - layout.line();
        let ink = match cells {
            Cells::ToColourIn => NUMBER_GREY,
            Cells::ColouredIn => {
                let colour = colouring.palette().colour_of(number);
                paper.fill(left, top, inside, inside, colour);
                readable_on(colour)
            }
        };
        paper.write(
            &number.to_string(),
            layout.cell_number_scale(),
            (left, top),
            (inside, inside),
            ink,
        );
    }
}

fn outline_the_cells(paper: &mut Paper, colouring: &Colouring, layout: &Layout, place: &GridPlace) {
    let (columns, rows) = (colouring.size().columns(), colouring.size().rows());
    let top_left = place.cell(0, 0);
    let bottom_right = place.cell(columns, rows);
    let grid_width = bottom_right.left - top_left.left + layout.line();
    let grid_height = bottom_right.top - top_left.top + layout.line();
    for column in 0..=columns {
        let left = place.cell(column, 0).left;
        paper.fill(left, top_left.top, layout.line(), grid_height, LINE_GREY);
    }
    for row in 0..=rows {
        let top = place.cell(0, row).top;
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

    /// Fills a disc with a colour, outlined in black.
    fn disc(&mut self, disc: &Disc, colour: Rgb) {
        let radius = disc.diameter / 2;
        let outline = (disc.diameter / OUTLINES_PER_BUBBLE_DIAMETER).max(1);
        let (centre_x, centre_y) = (disc.centre_x, disc.centre_y);
        for y in centre_y - radius..=centre_y + radius {
            for x in centre_x - radius..=centre_x + radius {
                let distance = x.abs_diff(centre_x).pow(2) + y.abs_diff(centre_y).pow(2);
                if distance <= radius.pow(2) {
                    let ink = if distance > radius.saturating_sub(outline).pow(2) {
                        BLACK
                    } else {
                        colour
                    };
                    self.fill(x, y, 1, 1, ink);
                }
            }
        }
    }

    /// Writes a number centred in a box given by its top left corner and its size.
    fn write(
        &mut self,
        number: &str,
        scale: u32,
        (left, top): (u32, u32),
        (width, height): (u32, u32),
        colour: Rgb,
    ) {
        let (number_width, number_height) = lettering::size(number, scale);
        let number_left = left + width.saturating_sub(number_width) / 2;
        let number_top = top + height.saturating_sub(number_height) / 2;
        for (x, y) in lettering::ink(number, scale) {
            self.fill(number_left + x, number_top + y, 1, 1, colour);
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
    use crate::layout::{Grids, Margin, Orientation, PageFormat};
    use crate::lettering;
    use crate::palette::ColourCount;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const YELLOW: [u8; 4] = [250, 220, 60, 255];
    const LINE_GREY: Rgb = Rgb(170, 170, 170);
    const NUMBER_GREY: Rgb = Rgb(90, 90, 90);
    const BLACK: Rgb = Rgb(0, 0, 0);

    fn red_red_yellow() -> (Colouring, Layout) {
        red_red_yellow_laid_out(Grids::Alone)
    }

    fn red_red_yellow_laid_out(grids: Grids) -> (Colouring, Layout) {
        let picture =
            Picture::from_rgba(3, 1, [RED, RED, YELLOW].as_flattened()).expect("a picture");
        let colouring = Colouring::of(
            &picture,
            GridSize::new(3, 1).expect("a grid"),
            ColourCount::new(2).expect("a count"),
        );
        let layout = Layout::of(
            &colouring,
            PageFormat::A5,
            Orientation::Portrait,
            Margin::AROUND_A_SHEET,
            grids,
        )
        .expect("room");
        (colouring, layout)
    }

    #[test]
    fn a_sheet_fills_its_page_with_its_margin_blank_and_every_cell_outlined() {
        let (colouring, layout) = red_red_yellow();
        let margin = 236;

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::None));

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
        let corners = (0..3).map(|column| layout.grids()[0].cell(column, 0));
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

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::None));

        for column in 0..3 {
            let cell = layout.grids()[0].cell(column, 0);
            let (left, top) = (cell.left + layout.line(), cell.top + layout.line());
            let inside = cell.side - layout.line();
            let number = colouring
                .numbered_cells()
                .nth(column as usize)
                .map(|(_, _, number)| number.to_string())
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

    #[test]
    fn each_bubble_is_filled_with_its_palette_colour_and_outlined_in_black() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::None));

        let bubbles = layout.bubbles().iter().zip(colouring.palette().colours());
        for (bubble, &colour) in bubbles {
            let radius = bubble.diameter / 2;
            let (x, y) = (bubble.centre_x, bubble.centre_y);
            assert_eq!(sheet.pixel(x, y + radius * 3 / 4), colour);
            assert_eq!(sheet.pixel(x - radius + 1, y), BLACK);
            assert_eq!(sheet.pixel(x - radius - 2, y), Rgb::WHITE);
        }
    }

    #[test]
    fn a_full_model_paints_each_cell_in_its_colour_with_its_number_readable_on_it() {
        let (colouring, layout) = red_red_yellow();
        let [red, yellow] = [RED, YELLOW].map(|[r, g, b, _]| Rgb(r, g, b));

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::Full));

        for (column, colour, ink) in [
            (0, red, Rgb::WHITE),
            (1, red, Rgb::WHITE),
            (2, yellow, BLACK),
        ] {
            let cell = layout.grids()[0].cell(column, 0);
            let (left, top) = (cell.left + layout.line(), cell.top + layout.line());
            let inside = cell.side - layout.line();
            let number = colouring
                .numbered_cells()
                .nth(column as usize)
                .map(|(_, _, number)| number.to_string())
                .unwrap_or_default();
            let (width, height) = lettering::size(&number, layout.cell_number_scale());
            let (number_left, number_top) =
                (left + (inside - width) / 2, top + (inside - height) / 2);
            let inks: std::collections::BTreeSet<Rgb> =
                lettering::ink(&number, layout.cell_number_scale())
                    .into_iter()
                    .map(|(x, y)| sheet.pixel(number_left + x, number_top + y))
                    .collect();
            assert_eq!(sheet.pixel(cell.left, cell.top), LINE_GREY, "cell {column}");
            assert_eq!(sheet.pixel(left, top), colour, "cell {column}");
            assert_eq!(
                sheet.pixel(left + inside - 1, top + inside - 1),
                colour,
                "cell {column}"
            );
            assert_eq!(inks.into_iter().collect::<Vec<_>>(), [ink], "cell {column}");
        }
    }

    #[test]
    fn a_twin_model_paints_the_first_grid_and_leaves_the_second_for_the_child_to_colour_in() {
        let (colouring, layout) = red_red_yellow_laid_out(Grids::Twinned);
        let [red, green, blue, _] = RED;

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::Twin));

        let insides = layout.grids().iter().map(|place| {
            let cell = place.cell(0, 0);
            sheet.pixel(cell.left + layout.line(), cell.top + layout.line())
        });
        assert_eq!(
            insides.collect::<Vec<_>>(),
            [Rgb(red, green, blue), Rgb::WHITE]
        );
    }

    #[test]
    fn a_free_legend_leaves_its_bubbles_white_outlined_in_black_with_their_numbers_in_black() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout, Legend::Freedom);

        let bubbles = layout.bubbles().iter().zip(["1", "2"]);
        for (bubble, number) in bubbles {
            let radius = bubble.diameter / 2;
            let (x, y) = (bubble.centre_x, bubble.centre_y);
            let (width, height) = lettering::size(number, layout.bubble_number_scale());
            let (left, top) = (x - width / 2, y - height / 2);
            let inks: std::collections::BTreeSet<Rgb> =
                lettering::ink(number, layout.bubble_number_scale())
                    .into_iter()
                    .map(|(dx, dy)| sheet.pixel(left + dx, top + dy))
                    .collect();
            assert_eq!(sheet.pixel(x, y + radius * 3 / 4), Rgb::WHITE);
            assert_eq!(sheet.pixel(x - radius + 1, y), BLACK);
            assert_eq!(
                inks.into_iter().collect::<Vec<_>>(),
                [BLACK],
                "bubble {number}"
            );
        }
    }

    #[test]
    fn a_bubbles_number_is_written_in_white_on_a_dark_colour_and_in_black_on_a_light_one() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::None));

        let inks = layout
            .bubbles()
            .iter()
            .zip(["1", "2"])
            .map(|(bubble, number)| {
                let (width, height) = lettering::size(number, layout.bubble_number_scale());
                let (left, top) = (bubble.centre_x - width / 2, bubble.centre_y - height / 2);
                let dots = lettering::ink(number, layout.bubble_number_scale());
                let colours: std::collections::BTreeSet<Rgb> = dots
                    .into_iter()
                    .map(|(x, y)| sheet.pixel(left + x, top + y))
                    .collect();
                colours.into_iter().collect::<Vec<_>>()
            });

        assert_eq!(inks.collect::<Vec<_>>(), [vec![Rgb::WHITE], vec![BLACK]]);
    }

    #[test]
    fn light_red_and_light_blue_bubbles_count_every_channel_and_write_their_numbers_in_black() {
        let red_pink_light_blue = [220, 30, 40, 255, 255, 110, 110, 255, 100, 140, 255, 255];
        let picture = Picture::from_rgba(3, 1, &red_pink_light_blue).expect("a picture");
        let colouring = Colouring::of(
            &picture,
            GridSize::new(3, 1).expect("a grid"),
            ColourCount::new(3).expect("a count"),
        );
        let layout = Layout::of(
            &colouring,
            PageFormat::A5,
            Orientation::Portrait,
            Margin::AROUND_A_SHEET,
            Grids::Alone,
        )
        .expect("room");

        let sheet = draw(&colouring, &layout, Legend::Coloured(Model::None));

        let inks = layout
            .bubbles()
            .iter()
            .zip(["1", "2", "3"])
            .map(|(bubble, number)| {
                let (width, height) = lettering::size(number, layout.bubble_number_scale());
                let (left, top) = (bubble.centre_x - width / 2, bubble.centre_y - height / 2);
                let colours: std::collections::BTreeSet<Rgb> =
                    lettering::ink(number, layout.bubble_number_scale())
                        .into_iter()
                        .map(|(x, y)| sheet.pixel(left + x, top + y))
                        .collect();
                colours.into_iter().collect::<Vec<_>>()
            });
        assert_eq!(
            colouring.palette().colours(),
            [Rgb(100, 140, 255), Rgb(220, 30, 40), Rgb(255, 110, 110)]
        );
        assert_eq!(
            inks.collect::<Vec<_>>(),
            [vec![BLACK], vec![Rgb::WHITE], vec![BLACK]]
        );
    }
}
