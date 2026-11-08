//! Where everything lies on the sheet's paper, in dots at 300 dots per inch.

use crate::colouring::Colouring;
use crate::lettering;

/// The paper a sheet is drawn for, its sides given upright.
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

/// The most cells across a grid may have on a paper, for a child to colour them, half as many for each of a twin's grids.
pub fn most_cells_across(format: PageFormat, legend: Legend) -> u32 {
    let on_the_page = match format {
        PageFormat::A3 => 64,
        PageFormat::A4 => 48,
        PageFormat::A5 => 32,
    };
    match legend {
        Legend::Coloured(Model::Twin) => on_the_page / 2,
        Legend::Coloured(Model::None | Model::Full) | Legend::Freedom => on_the_page,
    }
}

/// Which way up the paper lies: upright, or on its side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
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
    pub const AROUND_A_SHEET: Margin = Margin(10);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRoom {
    InsideTheMargin,
    ForTheNumbersInTheCells,
    ForTheNumbersInTheLegend,
}

/// Whether the sheet has a legend of the picture's colours, with a model in those colours or none,
/// or none at all, leaving the child free to pick their own, which no model could show.
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

/// Where one grid lies on the page: its top left corner and its cells' side, in dots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPlace {
    left: u32,
    top: u32,
    cell_side: u32,
}

impl GridPlace {
    /// A cell's square, its outline's top and left lines included.
    pub fn cell(&self, column: u32, row: u32) -> Square {
        Square {
            left: self.left + column * self.cell_side,
            top: self.top + row * self.cell_side,
            side: self.cell_side,
        }
    }
}

/// The line crediting trixelate and its licence, written small at the bottom right of every sheet.
pub const CREDIT: &str = "Made with trixelate (github.com/Maeevick/trixelate) - MIT License";

/// Where the credit is written: the top left corner of its first character, and its scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credit {
    pub left: u32,
    pub top: u32,
    pub scale: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    page_width: u32,
    page_height: u32,
    grids: Vec<GridPlace>,
    line: u32,
    cell_number_scale: u32,
    bubbles: Vec<Disc>,
    bubble_number_scale: u32,
    credit: Option<Credit>,
}

/// The credit stays small, under two millimetres tall, and at most half as tall as the margin it sits in.
const CREDIT_SCALE_AT_MOST: u32 = 3;

const GAP_ABOVE_THE_LEGEND_MM: u32 = 10;
const GAP_BETWEEN_TWINS_MM: u32 = 10;
const WIDEST_BUBBLE_MM: u32 = 15;
const LINES_PER_CELL_SIDE: u32 = 16;

impl Layout {
    /// Lays out a colouring's grids and legend on the page, inside its margin.
    pub fn of(
        colouring: &Colouring,
        format: PageFormat,
        orientation: Orientation,
        margin: Margin,
        legend: Legend,
    ) -> Result<Layout, NoRoom> {
        let (width, height) = match (orientation, format.millimetres()) {
            (Orientation::Portrait, (short, long)) => (short, long),
            (Orientation::Landscape, (short, long)) => (long, short),
        };
        let (page_width, page_height) = (dots(width), dots(height));
        let margin = dots(margin.0);
        let room_inside = |side: u32| side.checked_sub(2 * margin).filter(|&room| room > 0);
        let content_width = room_inside(page_width).ok_or(NoRoom::InsideTheMargin)?;
        let content_height = room_inside(page_height).ok_or(NoRoom::InsideTheMargin)?;
        let colours = colouring.palette().colours().len() as u32;
        let bubble = match legend {
            Legend::Coloured(_) => {
                let widest = dots(WIDEST_BUBBLE_MM).min(4 * content_width / (5 * colours - 1));
                Some(widest.saturating_sub(1 - widest % 2))
            }
            Legend::Freedom => None,
        };
        let legend_height = bubble.map_or(0, |bubble| dots(GAP_ABOVE_THE_LEGEND_MM) + bubble);
        let grid_room = content_height
            .checked_sub(legend_height)
            .ok_or(NoRoom::InsideTheMargin)?;

        let (grids_across, grids_down) = match (legend, orientation) {
            (Legend::Coloured(Model::Twin), Orientation::Portrait) => (1, 2),
            (Legend::Coloured(Model::Twin), Orientation::Landscape) => (2, 1),
            (Legend::Coloured(Model::None | Model::Full) | Legend::Freedom, _) => (1, 1),
        };
        let gap_between_twins = dots(GAP_BETWEEN_TWINS_MM);
        let room_shared = |room: u32, grids: u32| {
            room.checked_sub((grids - 1) * gap_between_twins)
                .map(|room| room / grids)
                .ok_or(NoRoom::InsideTheMargin)
        };
        let room_across = room_shared(content_width, grids_across)?;
        let room_down = room_shared(grid_room, grids_down)?;

        let size = colouring.size();
        let roomiest_side = (room_across / size.columns()).min(room_down / size.rows());
        let line = (roomiest_side / LINES_PER_CELL_SIDE).max(1);
        let cell_side = (room_across.saturating_sub(line) / size.columns())
            .min(room_down.saturating_sub(line) / size.rows());
        let widest_number = colours.to_string();
        let inside_a_cell = cell_side.saturating_sub(line);
        let cell_number_scale = lettering::largest_scale_fitting(
            &widest_number,
            inside_a_cell * 2 / 3,
            inside_a_cell / 2,
        )
        .ok_or(NoRoom::ForTheNumbersInTheCells)?;
        let bubble_number_scale = bubble
            .map(|bubble| {
                lettering::largest_scale_fitting(&widest_number, bubble * 11 / 20, bubble * 9 / 20)
                    .ok_or(NoRoom::ForTheNumbersInTheLegend)
            })
            .transpose()?
            .unwrap_or_default();
        let grid_width = cell_side * size.columns() + line;
        let grid_height = cell_side * size.rows() + line;
        let twins_width = grids_across * grid_width + (grids_across - 1) * gap_between_twins;
        let twins_height = grids_down * grid_height + (grids_down - 1) * gap_between_twins;
        let twins_left = margin + (content_width - twins_width) / 2;
        let twins_top = margin + (grid_room - twins_height) / 2;

        let bubbles = bubble.map_or_else(Vec::new, |bubble| {
            let spacing = bubble / 4;
            let row_width = colours * bubble + (colours - 1) * spacing;
            let row_left = margin + (content_width - row_width) / 2;
            (0..colours)
                .map(|index| Disc {
                    centre_x: row_left + index * (bubble + spacing) + bubble / 2,
                    centre_y: page_height - margin - 1 - bubble / 2,
                    diameter: bubble,
                })
                .collect()
        });

        let credit =
            lettering::largest_scale_fitting(CREDIT, content_width, margin / 2).map(|scale| {
                let scale = scale.min(CREDIT_SCALE_AT_MOST);
                let (width, height) = lettering::size(CREDIT, scale);
                Credit {
                    left: margin + content_width - width,
                    top: page_height - margin + (margin - height) / 2,
                    scale,
                }
            });

        Ok(Layout {
            page_width,
            page_height,
            bubbles,
            grids: (0..grids_across * grids_down)
                .map(|index| GridPlace {
                    left: twins_left + index % grids_across * (grid_width + gap_between_twins),
                    top: twins_top + index / grids_across * (grid_height + gap_between_twins),
                    cell_side,
                })
                .collect(),
            line,
            cell_number_scale,
            bubble_number_scale,
            credit,
        })
    }

    pub fn page_width(&self) -> u32 {
        self.page_width
    }

    pub fn page_height(&self) -> u32 {
        self.page_height
    }

    /// Where each grid lies, in the order they are drawn.
    pub fn grids(&self) -> &[GridPlace] {
        &self.grids
    }

    /// The legend's bubbles, in the order of their colour numbers.
    pub fn bubbles(&self) -> &[Disc] {
        &self.bubbles
    }

    /// The scale every cell's number is written at.
    pub fn cell_number_scale(&self) -> u32 {
        self.cell_number_scale
    }

    /// The scale every bubble's number is written at.
    pub fn bubble_number_scale(&self) -> u32 {
        self.bubble_number_scale
    }

    /// Where the credit is written, if the margin has room for it.
    pub fn credit(&self) -> Option<Credit> {
        self.credit
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

    fn upright(
        colouring: &Colouring,
        format: PageFormat,
        margin: Margin,
    ) -> Result<Layout, NoRoom> {
        Layout::of(
            colouring,
            format,
            Orientation::Portrait,
            margin,
            Legend::Coloured(Model::None),
        )
    }

    fn free(colouring: &Colouring, format: PageFormat, margin: Margin) -> Result<Layout, NoRoom> {
        Layout::of(
            colouring,
            format,
            Orientation::Portrait,
            margin,
            Legend::Freedom,
        )
    }

    fn twinned(colouring: &Colouring, orientation: Orientation) -> Result<Layout, NoRoom> {
        Layout::of(
            colouring,
            PageFormat::A4,
            orientation,
            Margin(20),
            Legend::Coloured(Model::Twin),
        )
    }

    #[test]
    fn a_page_counts_its_millimetres_in_dots_at_300_dots_per_inch() {
        let small = colouring(4, 4, 2);

        let pages = [PageFormat::A3, PageFormat::A4, PageFormat::A5]
            .map(|format| upright(&small, format, Margin(20)))
            .map(|layout| layout.map(|layout| (layout.page_width(), layout.page_height())));

        assert_eq!(
            pages,
            [Ok((3508, 4961)), Ok((2480, 3508)), Ok((1748, 2480))]
        );
    }

    #[test]
    fn a_landscape_page_lies_its_format_on_its_side() {
        let small = colouring(4, 4, 2);

        let on_its_side = Layout::of(
            &small,
            PageFormat::A4,
            Orientation::Landscape,
            Margin(20),
            Legend::Coloured(Model::None),
        );

        assert_eq!(
            on_its_side.map(|layout| (layout.page_width(), layout.page_height())),
            Ok((3508, 2480))
        );
    }

    #[test]
    fn on_an_upright_page_a_twinned_model_stands_above_its_sheet_a_gap_apart_and_alike()
    -> Result<(), NoRoom> {
        let wide = colouring(10, 5, 2);
        let (margin, gap) = (236, 118);

        let layout = twinned(&wide, Orientation::Portrait)?;

        let grids = layout.grids();
        assert_eq!(grids.len(), 2);
        let (model, sheet) = (grids[0], grids[1]);
        let model_bottom = model.cell(0, 5).top + layout.line();
        let sheet_bottom = sheet.cell(0, 5).top + layout.line();
        let legend_top = layout.bubbles()[0].centre_y - layout.bubbles()[0].diameter / 2;
        assert_eq!(model.cell(0, 0).side, sheet.cell(0, 0).side);
        assert_eq!(model.cell(0, 0).left, sheet.cell(0, 0).left);
        assert_eq!(sheet.cell(0, 0).top - model_bottom, gap);
        assert!(model.cell(0, 0).top >= margin && sheet_bottom < legend_top);
        Ok(())
    }

    #[test]
    fn on_an_upright_page_twins_stand_together_halfway_between_the_top_margin_and_the_legend()
    -> Result<(), NoRoom> {
        let wide = colouring(10, 5, 2);
        let (margin, gap) = (236, 118);

        let layout = twinned(&wide, Orientation::Portrait)?;

        let (model, sheet) = (layout.grids()[0], layout.grids()[1]);
        let above = model.cell(0, 0).top - margin;
        let legend_top = layout.bubbles()[0].centre_y - layout.bubbles()[0].diameter / 2;
        let below = legend_top - gap - (sheet.cell(0, 5).top + layout.line());
        assert!(
            above > gap && above.abs_diff(below) <= 1,
            "{above} above, {below} below"
        );
        Ok(())
    }

    #[test]
    fn on_a_landscape_page_a_twinned_model_stands_left_of_its_sheet_a_gap_apart_and_alike()
    -> Result<(), NoRoom> {
        let square = colouring(5, 5, 2);
        let (margin, gap) = (236, 118);

        let layout = twinned(&square, Orientation::Landscape)?;

        let grids = layout.grids();
        assert_eq!(grids.len(), 2);
        let (model, sheet) = (grids[0], grids[1]);
        let model_right = model.cell(5, 0).left + layout.line();
        let sheet_right = sheet.cell(5, 0).left + layout.line();
        assert_eq!(model.cell(0, 0).side, sheet.cell(0, 0).side);
        assert_eq!(model.cell(0, 0).top, sheet.cell(0, 0).top);
        assert_eq!(sheet.cell(0, 0).left - model_right, gap);
        assert!(model.cell(0, 0).left >= margin && sheet_right <= 3508 - margin);
        Ok(())
    }

    #[test]
    fn a_free_sheet_has_no_legend_its_grid_taking_the_room_and_no_bubble_too_small_to_refuse()
    -> Result<(), NoRoom> {
        let (tall, twelve_colours) = (colouring(5, 10, 12), colouring(10, 5, 12));

        let with_a_legend = upright(&tall, PageFormat::A4, Margin(20))?;
        let without = free(&tall, PageFormat::A4, Margin(20))?;
        let on_a_narrow_page = free(&twelve_colours, PageFormat::A5, Margin(63));

        assert!(without.bubbles().is_empty());
        assert!(without.grids()[0].cell(0, 0).side > with_a_legend.grids()[0].cell(0, 0).side);
        assert!(on_a_narrow_page.is_ok());
        Ok(())
    }

    #[test]
    fn the_credit_sits_small_in_the_bottom_margin_flush_right_smaller_on_a_narrow_page_or_left_out()
    -> Result<(), NoRoom> {
        let small = colouring(4, 4, 2);

        let a4 = upright(&small, PageFormat::A4, Margin(20))?;
        let narrow = upright(&small, PageFormat::A5, Margin(40))?;
        let thin_margin = upright(&small, PageFormat::A4, Margin(1))?;

        let edges = a4.credit().map(|credit| {
            let (width, height) = lettering::size(CREDIT, credit.scale);
            (
                credit.scale,
                credit.left + width,
                credit.top,
                credit.top + height,
            )
        });
        assert_eq!(edges, Some((3, 2480 - 236, 3379, 3400)));
        assert_eq!(narrow.credit().map(|credit| credit.scale), Some(2));
        assert_eq!(thin_margin.credit(), None);
        Ok(())
    }

    #[test]
    fn a_wide_grid_spans_the_page_inside_its_margin_with_square_cells_centred_across()
    -> Result<(), NoRoom> {
        let ten_by_five = colouring(10, 5, 2);
        let margin = 236;

        let layout = upright(&ten_by_five, PageFormat::A4, Margin(20))?;

        let first = layout.grids()[0].cell(0, 0);
        let next_across = layout.grids()[0].cell(1, 0);
        let next_down = layout.grids()[0].cell(0, 1);
        let last = layout.grids()[0].cell(9, 4);
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

        let layout = upright(&tall, PageFormat::A4, Margin(20))?;

        let bubbles = layout.bubbles();
        assert_eq!(bubbles.len(), 12);
        let last_cell = layout.grids()[0].cell(4, 9);
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
            [Margin(74), Margin(75)].map(|margin| upright(&small, PageFormat::A5, margin));

        assert_eq!(
            layouts,
            [Err(NoRoom::InsideTheMargin), Err(NoRoom::InsideTheMargin)]
        );
    }

    #[test]
    fn numbers_too_big_for_their_cells_or_their_bubbles_are_refused_even_on_a_sliver_of_paper() {
        let three_hundred_columns = colouring(300, 2, 2);
        let twelve_colours = colouring(10, 5, 12);
        let one_column_of_twelve_colours = colouring(1, 12, 12);

        let layouts = [
            upright(&three_hundred_columns, PageFormat::A5, Margin(20)),
            upright(&twelve_colours, PageFormat::A5, Margin(63)),
            upright(&one_column_of_twelve_colours, PageFormat::A5, Margin(73)),
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

    #[test]
    fn a_grid_has_at_most_64_cells_across_on_a3_48_on_a4_and_32_on_a5_and_half_as_many_in_a_twin() {
        let formats = [PageFormat::A3, PageFormat::A4, PageFormat::A5];

        let most = formats.map(|format| {
            [
                Legend::Coloured(Model::None),
                Legend::Coloured(Model::Full),
                Legend::Freedom,
                Legend::Coloured(Model::Twin),
            ]
            .map(|legend| most_cells_across(format, legend))
        });

        assert_eq!(most, [[64, 64, 64, 32], [48, 48, 48, 24], [32, 32, 32, 16]]);
    }

    #[test]
    fn two_digit_numbers_fit_the_cells_of_a_grid_sixty_four_cells_across_on_a5_within_its_usual_margin()
     {
        let sixty_four_by_sixty_four_in_twelve_colours = colouring(64, 64, 12);

        let layout = upright(
            &sixty_four_by_sixty_four_in_twelve_colours,
            PageFormat::A5,
            Margin(20),
        );

        assert_eq!(layout.map(|layout| layout.cell_number_scale()), Ok(1));
    }

    #[test]
    fn bubbles_are_fifteen_millimetres_wide_when_the_row_has_room_and_shrink_to_share_a_long_row()
    -> Result<(), NoRoom> {
        let (few, many) = (colouring(4, 4, 2), colouring(4, 4, 12));
        let content_width = 2480 - 2 * 236;

        let roomy = upright(&few, PageFormat::A4, Margin(20))?;
        let crowded = upright(&many, PageFormat::A4, Margin(20))?;

        let bubbles = crowded.bubbles();
        let (first, last) = (bubbles[0], bubbles[bubbles.len() - 1]);
        let row_width =
            last.centre_x + last.diameter / 2 + 1 - (first.centre_x - first.diameter / 2);
        assert_eq!(roomy.bubbles()[0].diameter, 177);
        assert!(content_width - row_width < first.diameter / 4);
        Ok(())
    }

    #[test]
    fn cell_outlines_are_a_sixteenth_of_a_cells_side_thick() -> Result<(), NoRoom> {
        let tall = colouring(5, 10, 2);

        let layout = upright(&tall, PageFormat::A4, Margin(20))?;

        assert_eq!(layout.line(), layout.grids()[0].cell(0, 0).side / 16);
        Ok(())
    }

    #[test]
    fn a_cells_single_digit_number_stands_as_tall_as_half_the_cells_inside_allows()
    -> Result<(), NoRoom> {
        let few_colours = colouring(10, 5, 2);

        let layout = upright(&few_colours, PageFormat::A4, Margin(20))?;

        let half_inside = (layout.grids()[0].cell(0, 0).side - layout.line()) / 2;
        let (_, height) = lettering::size("2", layout.cell_number_scale());
        assert!(height <= half_inside && height + 7 > half_inside);
        Ok(())
    }
}
