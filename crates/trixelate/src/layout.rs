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

/// Whether the sheet has a legend of the picture's colours, with a model in those colours or none,
/// or none at all, leaving the child free to pick their own, which no model could show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legend {
    Coloured(Model),
    Freedom,
}

/// The coloured-in picture to copy from: none, or a miniature beside the grid, its legend beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    None,
    Miniature(MiniatureSize),
}

/// How big the miniature is next to its grid, three tries side by side until one is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniatureSize {
    /// The grid takes the most room a miniature a quarter as big leaves, the miniature then fills what is left, up to half.
    RoomLeft,
    Half,
    Third,
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
    miniature: Option<Miniature>,
}

/// The credit stays small, under two millimetres tall, and at most half as tall as the margin it sits in.
const CREDIT_SCALE_AT_MOST: u32 = 3;

const GAP_ABOVE_THE_LEGEND_MM: u32 = 10;
const GAP_AROUND_THE_MODEL_MM: u32 = 5;
/// A bubble is at most eight millimetres wide, big enough for its number, leaving the room to the cells.
const WIDEST_BUBBLE_TENTHS_OF_MM: u32 = 80;
const LINES_PER_CELL_SIDE: u32 = 16;

/// The room inside the margin, in dots, from the margin's inner edge.
#[derive(Clone, Copy)]
struct Room {
    margin: u32,
    width: u32,
    height: u32,
}

/// Where the grid, its outlines, the legend's bubbles and any miniature lie.
struct Placed {
    grid: GridPlace,
    line: u32,
    bubbles: Vec<Disc>,
    miniature: Option<Miniature>,
}

impl Layout {
    /// Lays out a colouring's grid, its legend and any model on the page, inside its margin.
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
        let room = Room {
            margin,
            width: room_inside(page_width).ok_or(NoRoom::InsideTheMargin)?,
            height: room_inside(page_height).ok_or(NoRoom::InsideTheMargin)?,
        };
        let size = colouring.size();
        let colours = colouring.palette().colours().len() as u32;
        let widest_bubble = dots_of_tenths(WIDEST_BUBBLE_TENTHS_OF_MM);
        let odd = |diameter: u32| diameter.saturating_sub(1 - diameter % 2);
        let (placed, bubble) = match legend {
            Legend::Coloured(Model::Miniature(miniature_size)) => {
                let bubble = odd(widest_bubble);
                let placed =
                    beside_its_model(size, colours, bubble, orientation, miniature_size, room)
                        .ok_or(NoRoom::InsideTheMargin)?;
                (placed, Some(bubble))
            }
            Legend::Coloured(Model::None) => {
                let bubble = odd(widest_bubble.min(4 * room.width / (5 * colours - 1)));
                (
                    above_its_legend(size, colours, Some(bubble), room)?,
                    Some(bubble),
                )
            }
            Legend::Freedom => (above_its_legend(size, colours, None, room)?, None),
        };

        let widest_number = colours.to_string();
        let inside_a_cell = placed.grid.cell_side.saturating_sub(placed.line);
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

    /// The layout with the sheet's file name as its title, centred in the top margin as small as the credit, when the
    /// sheet shows its model.
    pub fn titled(self, file_name: &str) -> Layout {
        let room_width = self.page_width - 2 * self.margin;
        let title = self
            .miniature
            .and(self.credit)
            .and_then(|credit| {
                lettering::largest_scale_fitting(file_name, room_width, self.margin)
                    .map(|scale| scale.min(credit.scale))
            })
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

    /// Where the title is written, if the sheet has one.
    pub fn title(&self) -> Option<Title> {
        self.title.clone()
    }

    /// Where the model's miniature lies, if the sheet has one.
    pub fn miniature(&self) -> Option<Miniature> {
        self.miniature
    }

    /// How thick the lines outlining the cells are, in dots.
    pub fn line(&self) -> u32 {
        self.line
    }
}

/// The grid as big as the room allows above a row of bubbles, one per colour, on the room's bottom edge, centred across
/// in the room left above them; or alone, centred, without a legend.
fn above_its_legend(
    size: GridSize,
    colours: u32,
    bubble: Option<u32>,
    room: Room,
) -> Result<Placed, NoRoom> {
    let legend_height = bubble.map_or(0, |bubble| dots(GAP_ABOVE_THE_LEGEND_MM) + bubble);
    let grid_room = room
        .height
        .checked_sub(legend_height)
        .ok_or(NoRoom::InsideTheMargin)?;
    let roomiest_side = (room.width / size.columns()).min(grid_room / size.rows());
    let line = (roomiest_side / LINES_PER_CELL_SIDE).max(1);
    let cell_side = (room.width.saturating_sub(line) / size.columns())
        .min(grid_room.saturating_sub(line) / size.rows());
    let grid_width = cell_side * size.columns() + line;
    let grid_height = cell_side * size.rows() + line;
    let bubbles = bubble.map_or_else(Vec::new, |bubble| {
        let spacing = bubble / 4;
        let row_width = colours * bubble + (colours - 1) * spacing;
        let row_left = room.margin + (room.width - row_width) / 2;
        (0..colours)
            .map(|index| Disc {
                centre_x: row_left + index * (bubble + spacing) + bubble / 2,
                centre_y: room.margin + room.height - 1 - bubble / 2,
                diameter: bubble,
            })
            .collect()
    });
    Ok(Placed {
        grid: GridPlace {
            left: room.margin + (room.width - grid_width) / 2,
            top: room.margin + (grid_room - grid_height) / 2,
            cell_side,
        },
        line,
        bubbles,
        miniature: None,
    })
}

/// The grid as big as the room allows beside its model and its legend: upright, the grid at the top, the model in the
/// bottom right corner and the legend in rows on its left; on its side, the model in the top left corner, the legend in
/// columns below it and the grid on the right.
fn beside_its_model(
    size: GridSize,
    colours: u32,
    bubble: u32,
    orientation: Orientation,
    miniature_size: MiniatureSize,
    room: Room,
) -> Option<Placed> {
    let arranged = |cell_side: u32, model_cell_side: u32| {
        let sides = Sides {
            size,
            line: (cell_side / LINES_PER_CELL_SIDE).max(1),
            cell: cell_side,
            model_cell: model_cell_side,
        };
        match orientation {
            Orientation::Portrait => upright_beside_its_model(sides, colours, bubble, room),
            Orientation::Landscape => on_its_side_beside_its_model(sides, colours, bubble, room),
        }
    };
    let (smallest, largest) = match miniature_size {
        MiniatureSize::RoomLeft => (4, 2),
        MiniatureSize::Half => (2, 2),
        MiniatureSize::Third => (3, 3),
    };
    let cell_side = (1..=room.width.min(room.height))
        .rev()
        .find(|&cell_side| arranged(cell_side, (cell_side / smallest).max(1)).is_some())?;
    (1..=(cell_side / largest).max(1))
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

fn upright_beside_its_model(sides: Sides, colours: u32, bubble: u32, room: Room) -> Option<Placed> {
    let gap = dots(GAP_AROUND_THE_MODEL_MM);
    let ((grid_width, grid_height), (model_width, model_height)) = (sides.grid(), sides.model());
    let legend_room = room.width.checked_sub(model_width + gap)?;
    let legend = BubbleLines::fitting(colours, bubble, legend_room)?;
    let band = model_height.max(legend.across());
    if grid_width > room.width || grid_height + gap + band > room.height {
        return None;
    }
    let (right, bottom) = (room.margin + room.width, room.margin + room.height);
    let (legend_left, legend_top) = (
        room.margin + (legend_room - legend.along()) / 2,
        bottom - legend.across(),
    );
    Some(Placed {
        grid: GridPlace {
            left: room.margin + (room.width - grid_width) / 2,
            top: room.margin,
            cell_side: sides.cell,
        },
        line: sides.line,
        bubbles: legend
            .centres(colours)
            .map(|(along, across)| Disc {
                centre_x: legend_left + along,
                centre_y: legend_top + across,
                diameter: bubble,
            })
            .collect(),
        miniature: Some(Miniature {
            left: right - model_width + sides.line,
            top: bottom - model_height + sides.line,
            cell_side: sides.model_cell,
        }),
    })
}

fn on_its_side_beside_its_model(
    sides: Sides,
    colours: u32,
    bubble: u32,
    room: Room,
) -> Option<Placed> {
    let gap = dots(GAP_AROUND_THE_MODEL_MM);
    let ((grid_width, grid_height), (model_width, model_height)) = (sides.grid(), sides.model());
    let legend_room = room.height.checked_sub(model_height + gap)?;
    let legend = BubbleLines::fitting(colours, bubble, legend_room)?;
    let column = model_width.max(legend.across());
    if grid_height > room.height || column + gap + grid_width > room.width {
        return None;
    }
    let (legend_left, legend_top) = (
        room.margin + (column - legend.across()) / 2,
        room.margin + model_height + gap + (legend_room - legend.along()) / 2,
    );
    Some(Placed {
        grid: GridPlace {
            left: room.margin + room.width - grid_width,
            top: room.margin + (room.height - grid_height) / 2,
            cell_side: sides.cell,
        },
        line: sides.line,
        bubbles: legend
            .centres(colours)
            .map(|(along, across)| Disc {
                centre_x: legend_left + across,
                centre_y: legend_top + along,
                diameter: bubble,
            })
            .collect(),
        miniature: Some(Miniature {
            left: room.margin + sides.line,
            top: room.margin + sides.line,
            cell_side: sides.model_cell,
        }),
    })
}

/// A legend's bubbles in as few lines as a length holds, all as long as the first but the last, read line after line.
struct BubbleLines {
    bubble: u32,
    spacing: u32,
    lines: u32,
    per_line: u32,
}

impl BubbleLines {
    fn fitting(colours: u32, bubble: u32, length: u32) -> Option<BubbleLines> {
        let spacing = bubble / 4;
        let most_per_line = (length + spacing) / (bubble + spacing);
        (most_per_line > 0).then(|| {
            let lines = colours.div_ceil(most_per_line);
            BubbleLines {
                bubble,
                spacing,
                lines,
                per_line: colours.div_ceil(lines),
            }
        })
    }

    fn along(&self) -> u32 {
        self.per_line * (self.bubble + self.spacing) - self.spacing
    }

    fn across(&self) -> u32 {
        self.lines * (self.bubble + self.spacing) - self.spacing
    }

    /// Each bubble's centre, along its line and across the lines, from the legend's corner.
    fn centres(&self, colours: u32) -> impl Iterator<Item = (u32, u32)> {
        let (step, per_line, radius) = (self.bubble + self.spacing, self.per_line, self.bubble / 2);
        (0..colours).map(move |index| {
            (
                index % per_line * step + radius,
                index / per_line * step + radius,
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
        Layout::of(
            colouring,
            format,
            Orientation::Portrait,
            margin,
            Legend::Coloured(Model::None),
        )
    }

    fn model_upright(colouring: &Colouring, size: MiniatureSize) -> Result<Layout, NoRoom> {
        let legend = Legend::Coloured(Model::Miniature(size));
        Layout::of(
            colouring,
            PageFormat::A4,
            Orientation::Portrait,
            Margin(10),
            legend,
        )
    }

    fn model_on_its_side(
        colouring: &Colouring,
        margin: Margin,
        size: MiniatureSize,
    ) -> Result<Layout, NoRoom> {
        let legend = Legend::Coloured(Model::Miniature(size));
        Layout::of(
            colouring,
            PageFormat::A4,
            Orientation::Landscape,
            margin,
            legend,
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

    #[test]
    fn a_sheet_with_its_model_keeps_the_grid_and_legend_of_a_plain_sheet_its_model_seven_sixteenths_of_the_grid_wide_in_the_top_left_corner_over_the_grid()
    -> Result<(), NoRoom> {
        let square = colouring(48, 48, 8);
        let margin = 118;

        let upright_with_model = model_upright(&square, MiniatureSize::RoomLeft)?;
        let upright_plain = upright(&square, PageFormat::A4, Margin(10))?;
        let lying_with_model = model_on_its_side(&square, Margin(10), MiniatureSize::RoomLeft)?;
        let lying_plain = Layout::of(
            &square,
            PageFormat::A4,
            Orientation::Landscape,
            Margin(10),
            Legend::Coloured(Model::None),
        )?;

        for (with_model, plain) in [
            (&upright_with_model, &upright_plain),
            (&lying_with_model, &lying_plain),
        ] {
            let line = with_model.line();
            assert_eq!(
                with_model.miniature(),
                Some(Miniature {
                    left: margin + line,
                    top: margin + line,
                    cell_side: with_model.grid().cell(0, 0).side * 7 / 16,
                })
            );
            assert_eq!(
                (with_model.grid(), with_model.bubbles()),
                (plain.grid(), plain.bubbles())
            );
            assert_eq!(plain.miniature(), None);
        }
        Ok(())
    }

    #[test]
    fn with_no_margin_the_miniature_frame_sits_in_the_page_corner() -> Result<(), NoRoom> {
        let square = colouring(48, 48, 8);

        let layout = model_on_its_side(&square, Margin(0), MiniatureSize::RoomLeft)?;

        assert_eq!(
            layout
                .miniature()
                .map(|miniature| (miniature.left, miniature.top)),
            Some((layout.line(), layout.line()))
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
    fn a_free_sheet_has_no_legend_its_grid_taking_the_room_and_no_bubble_too_small_to_refuse()
    -> Result<(), NoRoom> {
        let (tall, twelve_colours) = (colouring(5, 10, 12), colouring(10, 5, 12));

        let with_a_legend = upright(&tall, PageFormat::A4, Margin(20))?;
        let without = free(&tall, PageFormat::A4, Margin(20))?;
        let on_a_narrow_page = free(&twelve_colours, PageFormat::A5, Margin(63));

        assert!(without.bubbles().is_empty());
        assert!(without.grid().cell(0, 0).side > with_a_legend.grid().cell(0, 0).side);
        assert!(on_a_narrow_page.is_ok());
        Ok(())
    }

    #[test]
    fn with_its_model_the_sheet_is_titled_with_its_file_name_centred_in_the_top_margin_as_big_as_the_bubbles_numbers_shrinking_to_fit()
    -> Result<(), NoRoom> {
        let square = colouring(48, 48, 8);
        let (name, long_name) = ("goblin-48-8-a4-with-model.sheet.png", "goblin-".repeat(40));

        let titled = model_upright(&square, MiniatureSize::RoomLeft)?.titled(name);
        let long = model_upright(&square, MiniatureSize::RoomLeft)?.titled(&long_name);
        let plain = upright(&square, PageFormat::A4, Margin(10))?.titled(name);

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
        assert_eq!(plain.title(), None);
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
