//! The page printed for the child: the grid of numbered cells and its legend, drawn where the layout says.

use crate::colouring::Colouring;
use crate::layout::{CREDIT, Disc, Layout};
use crate::lettering;
use crate::picture::{Picture, Rgb};
use crate::png_file::{self, CannotWritePng};

const LINE_GREY: Rgb = Rgb(170, 170, 170);
const NUMBER_GREY: Rgb = Rgb(90, 90, 90);
const BLACK: Rgb = Rgb(0, 0, 0);
const OUTLINES_PER_BUBBLE_DIAMETER: u32 = 24;

/// The page printed for the child, or its preview: its dots, and how finely they are printed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sheet {
    page: Picture,
    dots_per_inch: u32,
}

impl Sheet {
    pub fn width(&self) -> u32 {
        self.page.width()
    }

    pub fn height(&self) -> u32 {
        self.page.height()
    }

    pub fn pixel(&self, x: u32, y: u32) -> Rgb {
        self.page.pixel(x, y)
    }

    /// The sheet as PNG bytes that tell printers its dots per inch, so it prints at its paper's size.
    pub fn png(&self) -> Result<Vec<u8>, CannotWritePng> {
        png_file::write(&self.page, self.dots_per_inch)
    }

    /// The sheet as PNG bytes encoded five times faster and four times bigger, for a screen to show rather than a
    /// printer to keep.
    pub fn png_to_show(&self) -> Result<Vec<u8>, CannotWritePng> {
        png_file::write_to_show(&self.page, self.dots_per_inch)
    }
}

/// Draws a colouring's sheet on white paper, as laid out.
pub fn draw(colouring: &Colouring, layout: &Layout) -> Sheet {
    drawn_with(number_the_cells, colouring, layout)
}

/// Draws a colouring's sheet as it looks once coloured in: every cell filled with its colour inside its outline.
pub fn draw_coloured_in(colouring: &Colouring, layout: &Layout) -> Sheet {
    drawn_with(colour_the_cells, colouring, layout)
}

/// A sheet on white paper, its cells outlined then filled as asked, its miniature, legend, credit and title drawn over.
fn drawn_with(
    fill_the_cells: fn(&mut Paper, &Colouring, &Layout),
    colouring: &Colouring,
    layout: &Layout,
) -> Sheet {
    let mut paper = Paper::blank(layout.page_width(), layout.page_height());
    outline_the_cells(&mut paper, colouring, layout);
    fill_the_cells(&mut paper, colouring, layout);
    draw_the_miniature(&mut paper, colouring, layout);
    draw_the_legend(&mut paper, colouring, layout);
    write_the_credit(&mut paper, layout);
    write_the_title(&mut paper, layout);
    Sheet {
        page: paper.into_picture(),
        dots_per_inch: layout.dots_per_inch(),
    }
}

/// Fills each cell with its colour inside its outline, as a child who coloured it in would.
fn colour_the_cells(paper: &mut Paper, colouring: &Colouring, layout: &Layout) {
    let place = layout.grid();
    for (column, row, number) in colouring.numbered_cells() {
        let cell = place.cell(column, row);
        let inside = cell.side - layout.line();
        paper.fill(
            cell.left + layout.line(),
            cell.top + layout.line(),
            inside,
            inside,
            colouring.palette().colour_of(number),
        );
    }
}

/// Paints the model's miniature, a block of colour per cell, framed like the grid's lines.
fn draw_the_miniature(paper: &mut Paper, colouring: &Colouring, layout: &Layout) {
    let miniature = layout.miniature();
    let side = miniature.cell_side;
    let (width, height) = (
        colouring.size().columns() * side,
        colouring.size().rows() * side,
    );
    let line = layout.line();
    paper.fill(
        miniature.left - line,
        miniature.top - line,
        width + 2 * line,
        height + 2 * line,
        LINE_GREY,
    );
    for (column, row, number) in colouring.numbered_cells() {
        paper.fill(
            miniature.left + column * side,
            miniature.top + row * side,
            side,
            side,
            colouring.palette().colour_of(number),
        );
    }
}

fn write_the_credit(paper: &mut Paper, layout: &Layout) {
    let credit = layout.credit();
    write_small(paper, CREDIT, (credit.left, credit.top), credit.scale);
}

fn write_the_title(paper: &mut Paper, layout: &Layout) {
    if let Some(title) = layout.title() {
        write_small(paper, &title.text, (title.left, title.top), title.scale);
    }
}

/// Writes a line in grey in a margin, from the top left corner of its first character.
fn write_small(paper: &mut Paper, text: &str, (left, top): (u32, u32), scale: u32) {
    for (x, y) in lettering::ink(text, scale) {
        paper.fill(left + x, top + y, 1, 1, NUMBER_GREY);
    }
}

fn draw_the_legend(paper: &mut Paper, colouring: &Colouring, layout: &Layout) {
    for (bubble, (number, colour)) in layout.bubbles().iter().zip(colouring.palette().numbered()) {
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

/// Black or white, whichever contrasts more with a colour, after WCAG 2.2's relative luminance, so a bubble's number
/// reads at no less than about 4.6 to 1.
fn readable_on(Rgb(red, green, blue): Rgb) -> Rgb {
    let linear = |channel: u8| {
        let share = f64::from(channel) / 255.0;
        if share <= 0.04045 {
            share / 12.92
        } else {
            ((share + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue);
    let (with_black, with_white) = ((luminance + 0.05) / 0.05, 1.05 / (luminance + 0.05));
    if with_black >= with_white {
        BLACK
    } else {
        Rgb::WHITE
    }
}

/// Writes each cell's number in grey, centred inside its outline.
fn number_the_cells(paper: &mut Paper, colouring: &Colouring, layout: &Layout) {
    let place = layout.grid();
    for (column, row, number) in colouring.numbered_cells() {
        let cell = place.cell(column, row);
        let inside = cell.side - layout.line();
        paper.write(
            &number.to_string(),
            layout.cell_number_scale(),
            (cell.left + layout.line(), cell.top + layout.line()),
            (inside, inside),
            NUMBER_GREY,
        );
    }
}

fn outline_the_cells(paper: &mut Paper, colouring: &Colouring, layout: &Layout) {
    let place = layout.grid();
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
    use crate::layout::{CREDIT, Margin, Orientation, PageFormat};
    use crate::lettering;
    use crate::palette::ColourCount;
    use std::collections::BTreeSet;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const YELLOW: [u8; 4] = [250, 220, 60, 255];
    const LINE_GREY: Rgb = Rgb(170, 170, 170);
    const NUMBER_GREY: Rgb = Rgb(90, 90, 90);
    const BLACK: Rgb = Rgb(0, 0, 0);

    fn red_red_yellow() -> (Colouring, Layout) {
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
            Margin::new(20).expect("a margin"),
        )
        .expect("room");
        (colouring, layout)
    }

    #[test]
    fn a_sheet_encoded_to_show_reads_back_as_the_same_page_at_the_same_dots_per_inch() {
        let (colouring, layout) = red_red_yellow();
        let sheet = draw(&colouring, &layout);

        let [kept, shown] = [sheet.png(), sheet.png_to_show()].map(Result::unwrap_or_default);

        let read = |bytes: &[u8]| {
            let pixels = png_file::read(bytes).ok();
            let dots_per_metre = png::Decoder::new(std::io::Cursor::new(bytes))
                .read_info()
                .ok()
                .and_then(|reader| reader.info().pixel_dims)
                .map(|dims| dims.xppu);
            (pixels, dots_per_metre)
        };
        assert!(read(&kept).0.is_some());
        assert_eq!(read(&shown), read(&kept));
    }

    #[test]
    fn a_sheet_fills_its_page_its_margin_blank_but_for_its_grey_credit_and_every_cell_outlined() {
        let (colouring, layout) = red_red_yellow();
        let margin = 236;

        let sheet = draw(&colouring, &layout);

        let (width, height) = (layout.page_width(), layout.page_height());
        assert_eq!((sheet.width(), sheet.height()), (width, height));
        let inked_in_the_margin: BTreeSet<(u32, u32, Rgb)> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                x < margin || y < margin || x >= width - margin || y >= height - margin
            })
            .map(|(x, y)| (x, y, sheet.pixel(x, y)))
            .filter(|&(_, _, colour)| colour != Rgb::WHITE)
            .collect();
        let credit = layout.credit();
        let credit: BTreeSet<(u32, u32, Rgb)> = lettering::ink(CREDIT, credit.scale)
            .into_iter()
            .map(|(x, y)| (credit.left + x, credit.top + y, NUMBER_GREY))
            .collect();
        assert!(!credit.is_empty());
        assert_eq!(inked_in_the_margin, credit);
        let corners = (0..3).map(|column| layout.grid().cell(column, 0));
        assert!(
            corners
                .into_iter()
                .all(|cell| sheet.pixel(cell.left, cell.top) == LINE_GREY
                    && sheet.pixel(cell.left + cell.side, cell.top + cell.side) == LINE_GREY)
        );
    }

    #[test]
    fn a_sheet_writes_its_title_in_grey_in_its_top_margin() {
        let (colouring, layout) = red_red_yellow();
        let layout = layout.titled("red-red-yellow.png");
        let margin = 236;

        let sheet = draw(&colouring, &layout);

        let inked_in_the_top_margin: BTreeSet<(u32, u32, Rgb)> = (0..margin)
            .flat_map(|y| (0..layout.page_width()).map(move |x| (x, y)))
            .map(|(x, y)| (x, y, sheet.pixel(x, y)))
            .filter(|&(_, _, colour)| colour != Rgb::WHITE)
            .collect();
        let title: BTreeSet<(u32, u32, Rgb)> = layout
            .title()
            .map(|title| {
                lettering::ink(&title.text, title.scale)
                    .into_iter()
                    .map(|(x, y)| (title.left + x, title.top + y, NUMBER_GREY))
                    .collect()
            })
            .unwrap_or_default();
        assert!(!title.is_empty());
        assert_eq!(inked_in_the_top_margin, title);
    }

    #[test]
    fn each_cell_shows_its_colour_number_in_dark_grey_centred_inside_its_outline() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout);

        for column in 0..3 {
            let cell = layout.grid().cell(column, 0);
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
    fn a_coloured_in_sheet_is_its_sheet_with_every_cell_filled_with_its_colour_over_its_number() {
        let (colouring, layout) = red_red_yellow();
        let line = layout.line();
        let colour_inside = |x: u32, y: u32| {
            colouring
                .numbered_cells()
                .find_map(|(column, row, number)| {
                    let cell = layout.grid().cell(column, row);
                    let inside = |from: u32, at: u32| (from + line..from + cell.side).contains(&at);
                    (inside(cell.left, x) && inside(cell.top, y))
                        .then(|| colouring.palette().colour_of(number))
                })
        };
        let sheet = draw(&colouring, &layout);

        let coloured_in = draw_coloured_in(&colouring, &layout);

        let (width, height) = (layout.page_width(), layout.page_height());
        assert_eq!((coloured_in.width(), coloured_in.height()), (width, height));
        let unlike_the_sheet_coloured_in = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let expected = colour_inside(x, y).unwrap_or_else(|| sheet.pixel(x, y));
                coloured_in.pixel(x, y) != expected
            })
            .count();
        assert_eq!(unlike_the_sheet_coloured_in, 0);
    }

    #[test]
    fn each_bubble_is_filled_with_its_palette_colour_and_outlined_in_black() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout);

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
    fn a_miniature_shows_each_cell_in_its_colour_without_number_or_line_framed_in_grey() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout);

        let miniature = layout.miniature();
        let side = miniature.cell_side;
        let colour_of = |column| {
            colouring
                .numbered_cells()
                .find(|&(cell_column, _, _)| cell_column == column)
                .map(|(_, _, number)| colouring.palette().colour_of(number))
        };
        let inked = |column: u32| -> BTreeSet<Rgb> {
            let left = miniature.left + column * side;
            (miniature.top..miniature.top + side)
                .flat_map(|y| (left..left + side).map(move |x| (x, y)))
                .map(|(x, y)| sheet.pixel(x, y))
                .collect()
        };
        assert_eq!(
            (0..3).map(inked).collect::<Vec<_>>(),
            (0..3)
                .map(|column| colour_of(column).into_iter().collect())
                .collect::<Vec<BTreeSet<Rgb>>>()
        );
        let line = layout.line();
        let frame = [
            (miniature.left - line, miniature.top),
            (miniature.left + 3 * side, miniature.top),
            (miniature.left, miniature.top - line),
            (miniature.left, miniature.top + side),
        ];
        assert!(frame.iter().all(|&(x, y)| sheet.pixel(x, y) == LINE_GREY));
    }

    /// The inks each bubble's number is written in, bubble by bubble, the numbers counting from 1.
    fn inks_of_the_numbers_in_the_bubbles(sheet: &Sheet, layout: &Layout) -> Vec<Vec<Rgb>> {
        let scale = layout.bubble_number_scale();
        (1..)
            .zip(layout.bubbles())
            .map(|(number, bubble): (usize, &Disc)| {
                let number = number.to_string();
                let (width, height) = lettering::size(&number, scale);
                let (left, top) = (bubble.centre_x - width / 2, bubble.centre_y - height / 2);
                let colours: BTreeSet<Rgb> = lettering::ink(&number, scale)
                    .into_iter()
                    .map(|(x, y)| sheet.pixel(left + x, top + y))
                    .collect();
                colours.into_iter().collect()
            })
            .collect()
    }

    /// A one-row picture of these colours, coloured in as many, laid out upright on A5.
    fn laid_out(colours: &[[u8; 4]]) -> (Colouring, Layout) {
        let count = colours.len();
        let picture =
            Picture::from_rgba(count as u32, 1, colours.as_flattened()).expect("a picture");
        let colouring = Colouring::of(
            &picture,
            GridSize::new(count as u32, 1).expect("a grid"),
            ColourCount::new(count).expect("a count"),
        );
        let layout = Layout::of(
            &colouring,
            PageFormat::A5,
            Orientation::Portrait,
            Margin::new(20).expect("a margin"),
        )
        .expect("room");
        (colouring, layout)
    }

    #[test]
    fn a_bubbles_number_is_written_in_white_on_a_dark_colour_and_in_black_on_a_light_one() {
        let (colouring, layout) = red_red_yellow();

        let sheet = draw(&colouring, &layout);

        assert_eq!(
            inks_of_the_numbers_in_the_bubbles(&sheet, &layout),
            [vec![Rgb::WHITE], vec![BLACK]]
        );
    }

    #[test]
    fn a_bright_magenta_bubble_takes_the_ink_that_contrasts_with_it_most_black_at_over_seven_to_one()
     {
        let (colouring, layout) = laid_out(&[[255, 78, 249, 255], [20, 30, 90, 255]]);

        let sheet = draw(&colouring, &layout);

        let inks = inks_of_the_numbers_in_the_bubbles(&sheet, &layout);
        let magenta = colouring
            .palette()
            .colours()
            .iter()
            .position(|&colour| colour == Rgb(255, 78, 249))
            .expect("the magenta in the palette");
        assert_eq!(inks[magenta], [BLACK]);
        assert_eq!(inks[1 - magenta], [Rgb::WHITE]);
    }

    #[test]
    fn a_near_black_bubble_takes_white_ink_however_dark_its_channels_are() {
        let (colouring, layout) = laid_out(&[[8, 8, 8, 255], [250, 250, 250, 255]]);

        let sheet = draw(&colouring, &layout);

        assert_eq!(
            colouring.palette().colours(),
            [Rgb(8, 8, 8), Rgb(250, 250, 250)]
        );
        assert_eq!(
            inks_of_the_numbers_in_the_bubbles(&sheet, &layout),
            [vec![Rgb::WHITE], vec![BLACK]]
        );
    }

    #[test]
    fn light_red_and_light_blue_bubbles_count_every_channel_and_write_their_numbers_in_black() {
        let (colouring, layout) = laid_out(&[
            [220, 30, 40, 255],
            [255, 110, 110, 255],
            [100, 140, 255, 255],
        ]);

        let sheet = draw(&colouring, &layout);

        assert_eq!(
            colouring.palette().colours(),
            [Rgb(100, 140, 255), Rgb(220, 30, 40), Rgb(255, 110, 110)]
        );
        assert_eq!(
            inks_of_the_numbers_in_the_bubbles(&sheet, &layout),
            [vec![BLACK], vec![Rgb::WHITE], vec![BLACK]]
        );
    }
}
