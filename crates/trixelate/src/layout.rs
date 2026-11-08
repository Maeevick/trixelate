//! Where everything lies on the sheet's paper, in dots at 300 dots per inch.

use crate::colouring::Colouring;
use crate::grid::GridSize;
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

/// The most cells across a grid may have on a paper, for a child to colour them.
pub fn most_cells_across(format: PageFormat) -> u32 {
    match format {
        PageFormat::A3 => 64,
        PageFormat::A4 => 48,
        PageFormat::A5 => 32,
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
    dots_of_tenths(millimetres * 10)
}

fn dots_of_tenths(tenths_of_millimetre: u32) -> u32 {
    (tenths_of_millimetre * DOTS_PER_INCH + TENTHS_OF_MILLIMETRE_PER_INCH / 2)
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

/// Where the model's miniature lies: its top left corner and its cells' side, in dots, its frame around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miniature {
    pub left: u32,
    pub top: u32,
    pub cell_side: u32,
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

/// Where a sheet's title is written: its text, the top left corner of its first character, and its scale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Title {
    pub text: String,
    pub left: u32,
    pub top: u32,
    pub scale: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    page_width: u32,
    page_height: u32,
    grid: GridPlace,
    line: u32,
    cell_number_scale: u32,
    bubbles: Vec<Disc>,
    bubble_number_scale: u32,
    margin: u32,
    credit: Option<Credit>,
    title: Option<Title>,
    miniature: Miniature,
}

/// The credit stays small, under two millimetres tall, and at most half as tall as the margin it sits in.
const CREDIT_SCALE_AT_MOST: u32 = 3;

const GAP_ABOVE_THE_LEGEND_MM: u32 = 10;
/// A bubble is at most eight millimetres wide, big enough for its number, leaving the room to the cells.
const WIDEST_BUBBLE_TENTHS_OF_MM: u32 = 80;
const LINES_PER_CELL_SIDE: u32 = 16;
/// How many sixteenths of a grid's cell a miniature's cell is, upright: the miniature is seven sixteenths of the grid wide.
const MINIATURE_SIXTEENTHS_OF_A_CELL: u32 = 7;
/// On its side, the miniature is at least a quarter as wide as the grid, and at most half.
const GRID_CELLS_PER_MODEL_CELL_AT_MOST: u32 = 4;
const GRID_CELLS_PER_MODEL_CELL_AT_LEAST: u32 = 2;
const GAP_AROUND_THE_MODEL_MM: u32 = 5;

/// The room inside the margin, in dots, from the margin's inner edge.
#[derive(Clone, Copy)]
struct Room {
    margin: u32,
    width: u32,
    height: u32,
}

/// Where the grid, its outlines, the legend's bubbles and the model's miniature lie.
struct Placed {
    grid: GridPlace,
    line: u32,
    bubbles: Vec<Disc>,
    miniature: Miniature,
}

impl Layout {
    /// Lays out a colouring's grid, its legend and its model on the page, inside its margin.
    pub fn of(
        colouring: &Colouring,
        format: PageFormat,
        orientation: Orientation,
        margin: Margin,
    ) -> Result<Layout, NoRoom> {
        let (width, height) = match (orientation, format.millimetres()) {
            (Orientation::Portrait, (short, long)) => (short, long),
            (Orientation::Landscape, (short, long)) => (long, short),
        };
        let (page_width, page_height) = (dots(width), dots(height));
        let margin = dots(margin.0);
        let room_inside = |side: u32| side.checked_sub(2 * margin).filter(|&room| room > 0);
        let room = Room {
            margin,
            width: room_inside(page_width).ok_or(NoRoom::InsideTheMargin)?,
            height: room_inside(page_height).ok_or(NoRoom::InsideTheMargin)?,
        };
        let size = colouring.size();
        let colours = colouring.palette().colours().len() as u32;
        let widest_bubble = dots_of_tenths(WIDEST_BUBBLE_TENTHS_OF_MM);
        let odd = |diameter: u32| diameter.saturating_sub(1 - diameter % 2);
        let (placed, bubble) = match orientation {
            Orientation::Landscape => {
                let bubble = odd(widest_bubble);
                let placed =
                    beside_its_model(size, colours, bubble, room).ok_or(NoRoom::InsideTheMargin)?;
                (placed, bubble)
            }
            Orientation::Portrait => {
                let bubble = odd(widest_bubble.min(4 * room.width / (5 * colours - 1)));
                (
                    on_its_legend_under_its_model(size, colours, bubble, room)?,
                    bubble,
                )
            }
        };

        let widest_number = colours.to_string();
        let inside_a_cell = placed.grid.cell_side.saturating_sub(placed.line);
        let cell_number_scale = lettering::largest_scale_fitting(
            &widest_number,
            inside_a_cell * 2 / 3,
            inside_a_cell / 2,
        )
        .ok_or(NoRoom::ForTheNumbersInTheCells)?;
        let bubble_number_scale =
            lettering::largest_scale_fitting(&widest_number, bubble * 11 / 20, bubble * 9 / 20)
                .ok_or(NoRoom::ForTheNumbersInTheLegend)?;

        let credit =
            lettering::largest_scale_fitting(CREDIT, room.width, margin / 2).map(|scale| {
                let scale = scale.min(CREDIT_SCALE_AT_MOST);
                let (width, height) = lettering::size(CREDIT, scale);
                Credit {
                    left: margin + room.width - width,
                    top: page_height - margin + (margin - height) / 2,
                    scale,
                }
            });

        Ok(Layout {
            page_width,
            page_height,
            bubbles: placed.bubbles,
            grid: placed.grid,
            line: placed.line,
            cell_number_scale,
            bubble_number_scale,
            margin,
            credit,
            title: None,
            miniature: placed.miniature,
        })
    }

    pub fn page_width(&self) -> u32 {
        self.page_width
    }

    pub fn page_height(&self) -> u32 {
        self.page_height
    }

    /// Where the grid lies.
    pub fn grid(&self) -> GridPlace {
        self.grid
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

    /// The layout with the sheet's file name as its title, centred in the top margin as big as the bubbles' numbers, if
    /// the margin has room for it.
    pub fn titled(self, file_name: &str) -> Layout {
        let room_width = self.page_width - 2 * self.margin;
        let title = lettering::largest_scale_fitting(file_name, room_width, self.margin / 2)
            .map(|scale| scale.min(self.bubble_number_scale))
            .map(|scale| {
                let (width, height) = lettering::size(file_name, scale);
                Title {
                    text: file_name.to_string(),
                    left: self.margin + (room_width - width) / 2,
                    top: (self.margin - height) / 2,
                    scale,
                }
            });
        Layout { title, ..self }
    }

    /// Where the title is written, if the margin has room for it.
    pub fn title(&self) -> Option<Title> {
        self.title.clone()
    }

    /// Where the model's miniature lies.
    pub fn miniature(&self) -> Miniature {
        self.miniature
    }

    /// How thick the lines outlining the cells are, in dots.
    pub fn line(&self) -> u32 {
        self.line
    }
}

/// Upright, the grid as big as the room allows above a row of bubbles, one per colour, centred on the room's bottom
/// edge, the grid centred across and set down on its legend, leaving the top left corner to its model, seven sixteenths
/// of the grid wide, over the grid where they meet.
fn on_its_legend_under_its_model(
    size: GridSize,
    colours: u32,
    bubble: u32,
    room: Room,
) -> Result<Placed, NoRoom> {
    let legend_top = room.margin + room.height - bubble;
    let grid_room = room
        .height
        .checked_sub(dots(GAP_ABOVE_THE_LEGEND_MM) + bubble)
        .ok_or(NoRoom::InsideTheMargin)?;
    let roomiest_side = (room.width / size.columns()).min(grid_room / size.rows());
    let line = (roomiest_side / LINES_PER_CELL_SIDE).max(1);
    let cell_side = (room.width.saturating_sub(line) / size.columns())
        .min(grid_room.saturating_sub(line) / size.rows());
    let grid_width = cell_side * size.columns() + line;
    let grid_height = cell_side * size.rows() + line;
    let spacing = bubble / 4;
    let row_width = colours * bubble + (colours - 1) * spacing;
    let row_left = room.margin + (room.width - row_width) / 2;
    Ok(Placed {
        grid: GridPlace {
            left: room.margin + (room.width - grid_width) / 2,
            top: legend_top - dots(GAP_ABOVE_THE_LEGEND_MM) - grid_height,
            cell_side,
        },
        line,
        bubbles: (0..colours)
            .map(|index| Disc {
                centre_x: row_left + index * (bubble + spacing) + bubble / 2,
                centre_y: room.margin + room.height - 1 - bubble / 2,
                diameter: bubble,
            })
            .collect(),
        miniature: Miniature {
            left: room.margin + line,
            top: room.margin + line,
            cell_side: (cell_side * MINIATURE_SIXTEENTHS_OF_A_CELL / 16).max(1),
        },
    })
}

/// On its side, the grid as big as the room allows on the right, beside a model in the top left corner at least a
/// quarter of the grid wide, the model then filling the room the grid leaves up to half its width, and the legend in rows
/// centred below the model.
fn beside_its_model(size: GridSize, colours: u32, bubble: u32, room: Room) -> Option<Placed> {
    let arranged = |cell_side: u32, model_cell_side: u32| {
        let sides = Sides {
            size,
            line: (cell_side / LINES_PER_CELL_SIDE).max(1),
            cell: cell_side,
            model_cell: model_cell_side,
        };
        on_its_side_beside_its_model(sides, colours, bubble, room)
    };
    let smallest_model_cell =
        |cell_side: u32| (cell_side / GRID_CELLS_PER_MODEL_CELL_AT_MOST).max(1);
    let cell_side = (1..=room.width.min(room.height))
        .rev()
        .find(|&cell_side| arranged(cell_side, smallest_model_cell(cell_side)).is_some())?;
    (1..=(cell_side / GRID_CELLS_PER_MODEL_CELL_AT_LEAST).max(1))
        .rev()
        .find_map(|model_cell_side| arranged(cell_side, model_cell_side))
}

/// The sides of a grid's cells, of its outlines and of its model's cells, in dots.
#[derive(Clone, Copy)]
struct Sides {
    size: GridSize,
    line: u32,
    cell: u32,
    model_cell: u32,
}

impl Sides {
    fn grid(self) -> (u32, u32) {
        (
            self.cell * self.size.columns() + self.line,
            self.cell * self.size.rows() + self.line,
        )
    }

    /// The model's width and height, its frame included.
    fn model(self) -> (u32, u32) {
        (
            self.model_cell * self.size.columns() + 2 * self.line,
            self.model_cell * self.size.rows() + 2 * self.line,
        )
    }
}

fn on_its_side_beside_its_model(
    sides: Sides,
    colours: u32,
    bubble: u32,
    room: Room,
) -> Option<Placed> {
    let gap = dots(GAP_AROUND_THE_MODEL_MM);
    let ((grid_width, grid_height), (model_width, model_height)) = (sides.grid(), sides.model());
    let column = room.width.checked_sub(grid_width + gap)?;
    let below_the_model = room.height.checked_sub(model_height + gap)?;
    let legend = BubbleRows::fitting(colours, bubble, column)?;
    if grid_height > room.height || model_width > column || legend.height() > below_the_model {
        return None;
    }
    let (legend_left, legend_top) = (
        room.margin + (column - legend.width()) / 2,
        room.margin + model_height + gap + (below_the_model - legend.height()) / 2,
    );
    Some(Placed {
        grid: GridPlace {
            left: room.margin + column + gap,
            top: room.margin + (room.height - grid_height) / 2,
            cell_side: sides.cell,
        },
        line: sides.line,
        bubbles: legend
            .centres(colours)
            .map(|(x, y)| Disc {
                centre_x: legend_left + x,
                centre_y: legend_top + y,
                diameter: bubble,
            })
            .collect(),
        miniature: Miniature {
            left: room.margin + sides.line,
            top: room.margin + sides.line,
            cell_side: sides.model_cell,
        },
    })
}

/// A legend's bubbles in as few rows as a width holds, all as long as the first but the last, read row after row.
struct BubbleRows {
    bubble: u32,
    spacing: u32,
    rows: u32,
    per_row: u32,
}

impl BubbleRows {
    fn fitting(colours: u32, bubble: u32, width: u32) -> Option<BubbleRows> {
        let spacing = bubble / 4;
        let most_per_row = (width + spacing) / (bubble + spacing);
        (most_per_row > 0).then(|| {
            let rows = colours.div_ceil(most_per_row);
            BubbleRows {
                bubble,
                spacing,
                rows,
                per_row: colours.div_ceil(rows),
            }
        })
    }

    fn width(&self) -> u32 {
        self.per_row * (self.bubble + self.spacing) - self.spacing
    }

    fn height(&self) -> u32 {
        self.rows * (self.bubble + self.spacing) - self.spacing
    }

    /// Each bubble's centre, from the legend's top left corner.
    fn centres(&self, colours: u32) -> impl Iterator<Item = (u32, u32)> {
        let (step, per_row, radius) = (self.bubble + self.spacing, self.per_row, self.bubble / 2);
        (0..colours).map(move |index| {
            (
                index % per_row * step + radius,
                index / per_row * step + radius,
            )
        })
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
        Layout::of(colouring, format, Orientation::Portrait, margin)
    }

    fn on_its_side(colouring: &Colouring, margin: Margin) -> Result<Layout, NoRoom> {
        Layout::of(colouring, PageFormat::A4, Orientation::Landscape, margin)
    }

    #[test]
    fn either_way_up_a_sheet_centres_its_grid_above_a_row_of_bubbles_centred_on_the_bottom_its_model_seven_thirty_seconds_of_the_grid_wide_in_the_top_left_corner()
    -> Result<(), NoRoom> {
        let square = colouring(48, 48, 8);
        let margin = 118;

        let layouts = [
            (upright(&square, PageFormat::A4, Margin(10))?, (2480, 3508)),
            (on_its_side(&square, Margin(10))?, (3508, 2480)),
        ];

        for (layout, (page_width, page_height)) in layouts {
            let (right, bottom) = (page_width - margin, page_height - margin);
            let (grid, line) = (layout.grid(), layout.line());
            let (top_left, past_the_end) = (grid.cell(0, 0), grid.cell(48, 48));
            let (grid_right, grid_bottom) = (past_the_end.left + line, past_the_end.top + line);
            let bubbles = layout.bubbles();
            let radius = bubbles[0].diameter / 2;
            let legend_left = bubbles[0].centre_x - radius;
            let legend_right = bubbles[bubbles.len() - 1].centre_x + radius + 1;
            let legend_top = bubbles[0].centre_y - radius;
            let above_the_legend = legend_top - dots(10);
            assert_eq!(
                layout.miniature(),
                Miniature {
                    left: margin + line,
                    top: margin + line,
                    cell_side: top_left.side * 7 / 32,
                }
            );
            assert!((top_left.left - margin).abs_diff(right - grid_right) <= 1);
            assert!((top_left.top - margin).abs_diff(above_the_legend - grid_bottom) <= 1);
            assert!(
                (right - margin - (grid_right - top_left.left))
                    .min(above_the_legend - margin - (grid_bottom - top_left.top))
                    < 48
            );
            assert!(
                bubbles
                    .iter()
                    .all(|bubble| bubble.centre_y + radius + 1 == bottom)
            );
            assert!((legend_left - margin).abs_diff(right - legend_right) <= 1);
        }
        Ok(())
    }

    #[test]
    fn upright_a_tall_grid_grows_as_tall_as_the_room_above_its_legend_and_no_taller()
    -> Result<(), NoRoom> {
        let tall = colouring(10, 40, 8);
        let margin = 118;

        let layout = upright(&tall, PageFormat::A4, Margin(10))?;

        let top = layout.grid().cell(0, 0).top;
        assert!(top >= margin && top - margin < 40, "{top}");
        Ok(())
    }

    #[test]
    fn with_no_margin_the_miniature_frame_sits_in_the_page_corner() -> Result<(), NoRoom> {
        let square = colouring(48, 48, 8);

        let layout = on_its_side(&square, Margin(0))?;

        let miniature = layout.miniature();
        assert_eq!(
            (miniature.left, miniature.top),
            (layout.line(), layout.line())
        );
        Ok(())
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

        let on_its_side = Layout::of(&small, PageFormat::A4, Orientation::Landscape, Margin(20));

        assert_eq!(
            on_its_side.map(|layout| (layout.page_width(), layout.page_height())),
            Ok((3508, 2480))
        );
    }

    #[test]
    fn a_sheet_is_titled_with_its_file_name_centred_in_the_top_margin_as_big_as_the_bubbles_numbers_shrinking_to_fit()
    -> Result<(), NoRoom> {
        let square = colouring(48, 48, 8);
        let (name, long_name) = ("goblin-48-8-a4.sheet.png", "goblin-".repeat(40));

        let titled = upright(&square, PageFormat::A4, Margin(10))?.titled(name);
        let long = upright(&square, PageFormat::A4, Margin(10))?.titled(&long_name);
        let thin_margin = on_its_side(&square, Margin(5))?.titled(name);

        let (width, height) = lettering::size(name, 5);
        assert_eq!(titled.bubble_number_scale(), 5);
        assert_eq!(
            titled.title(),
            Some(Title {
                text: name.to_string(),
                left: 118 + (2244 - width) / 2,
                top: (118 - height) / 2,
                scale: 5,
            })
        );
        assert_eq!(long.title().map(|title| title.scale), Some(1));
        assert_eq!(thin_margin.title().map(|title| title.scale), Some(4));
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

        let first = layout.grid().cell(0, 0);
        let next_across = layout.grid().cell(1, 0);
        let next_down = layout.grid().cell(0, 1);
        let last = layout.grid().cell(9, 4);
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
        let last_cell = layout.grid().cell(4, 9);
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
    fn a_grid_has_at_most_64_cells_across_on_a3_48_on_a4_and_32_on_a5() {
        let formats = [PageFormat::A3, PageFormat::A4, PageFormat::A5];

        let most = formats.map(most_cells_across);

        assert_eq!(most, [64, 48, 32]);
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
    fn bubbles_are_eight_millimetres_wide_when_the_row_has_room_and_shrink_to_share_a_long_row()
    -> Result<(), NoRoom> {
        let (few, many) = (colouring(4, 4, 2), colouring(6, 4, 24));
        let content_width = 2480 - 2 * 236;

        let roomy = upright(&few, PageFormat::A4, Margin(20))?;
        let crowded = upright(&many, PageFormat::A4, Margin(20))?;

        let bubbles = crowded.bubbles();
        let (first, last) = (bubbles[0], bubbles[bubbles.len() - 1]);
        let row_width =
            last.centre_x + last.diameter / 2 + 1 - (first.centre_x - first.diameter / 2);
        assert_eq!(roomy.bubbles()[0].diameter, 93);
        assert!(first.diameter < 93);
        assert!(row_width <= content_width && content_width - row_width < first.diameter);
        Ok(())
    }

    #[test]
    fn cell_outlines_are_a_sixteenth_of_a_cells_side_thick() -> Result<(), NoRoom> {
        let tall = colouring(5, 10, 2);

        let layout = upright(&tall, PageFormat::A4, Margin(20))?;

        assert_eq!(layout.line(), layout.grid().cell(0, 0).side / 16);
        Ok(())
    }

    #[test]
    fn a_cells_single_digit_number_stands_as_tall_as_half_the_cells_inside_allows()
    -> Result<(), NoRoom> {
        let few_colours = colouring(10, 5, 2);

        let layout = upright(&few_colours, PageFormat::A4, Margin(20))?;

        let half_inside = (layout.grid().cell(0, 0).side - layout.line()) / 2;
        let (_, height) = lettering::size("2", layout.cell_number_scale());
        assert!(height <= half_inside && height + 7 > half_inside);
        Ok(())
    }
}
