//! Turns a picture into a colour-by-number pixel sheet, ready to print.

pub mod colouring;
pub mod grid;
pub mod jpeg_file;
pub mod layout;
pub mod lettering;
pub mod palette;
pub mod picture;
pub mod picture_file;
pub mod png_file;
pub mod sheet;
pub mod warning;

use std::num::NonZeroU32;

use colouring::Colouring;
use grid::GridSize;
use layout::{Layout, Margin, NoRoom, Orientation, PageFormat};
use palette::ColourCount;
use picture::Picture;
use sheet::Sheet;

/// What a sheet is asked to be: its cells across, its number of colours, its paper, which way up it lies and its
/// margin, only ever made by [`Order::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    cells_across: NonZeroU32,
    colours: ColourCount,
    format: PageFormat,
    orientation: Orientation,
    margin: Margin,
}

/// Why an order is refused before any picture is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoCellAcross,
    MoreCellsAcrossThanThePaperHolds { most: u32 },
}

impl Order {
    /// An order for a sheet, refused when it has no cell across or more than its paper holds for a child to colour.
    pub fn new(
        cells_across: u32,
        colours: ColourCount,
        format: PageFormat,
        orientation: Orientation,
        margin: Margin,
    ) -> Result<Order, Refusal> {
        let most = layout::most_cells_across(format);
        if cells_across > most {
            return Err(Refusal::MoreCellsAcrossThanThePaperHolds { most });
        }
        Ok(Order {
            cells_across: NonZeroU32::new(cells_across).ok_or(Refusal::NoCellAcross)?,
            colours,
            format,
            orientation,
            margin,
        })
    }

    pub fn cells_across(&self) -> u32 {
        self.cells_across.get()
    }

    pub fn colours(&self) -> ColourCount {
        self.colours
    }

    pub fn format(&self) -> PageFormat {
        self.format
    }

    /// The grid a picture is simplified into for this order: the cells across asked, and the rows that keep the
    /// picture's proportions, refused when the page could never number that many rows, before any cell is coloured.
    pub fn grid_for(&self, picture: &Picture) -> Result<GridSize, NoRoom> {
        let grid = GridSize::keeping_the_proportions_of(picture, self.cells_across);
        layout::could_number(grid, self.format, self.orientation, self.margin)?;
        Ok(grid)
    }
}

/// Turns a picture into the colour-by-number sheet ordered, titled with its file name, or says what lacks room on the
/// page.
pub fn make_sheet(picture: &Picture, order: Order, file_name: &str) -> Result<Sheet, NoRoom> {
    sheet_of(&colour(picture, order)?, order, file_name)
}

/// Turns a picture into the preview of the sheet ordered: the sheet as it looks once coloured in, titled with its file
/// name, or says what lacks room on the page, as making the sheet would.
pub fn make_preview(picture: &Picture, order: Order, file_name: &str) -> Result<Sheet, NoRoom> {
    preview_of(&colour(picture, order)?, order, file_name)
}

/// A picture simplified into the order's grid and numbered from its dominant colours, the slow part of making a sheet,
/// kept to lay out again on any page of the same cells across and colours.
pub fn colour(picture: &Picture, order: Order) -> Result<Colouring, NoRoom> {
    Ok(Colouring::of(
        picture,
        order.grid_for(picture)?,
        order.colours,
    ))
}

/// The sheet of a colouring, laid out on the order's paper, which way up and inside the margin it asks, titled with its
/// file name.
pub fn sheet_of(colouring: &Colouring, order: Order, file_name: &str) -> Result<Sheet, NoRoom> {
    Ok(sheet::draw(
        colouring,
        &lay_out(colouring, order)?.titled(file_name),
    ))
}

/// The preview of a colouring's sheet: the sheet as it looks once coloured in, laid out and titled as the sheet is.
pub fn preview_of(colouring: &Colouring, order: Order, file_name: &str) -> Result<Sheet, NoRoom> {
    Ok(sheet::draw_coloured_in(
        colouring,
        &lay_out(colouring, order)?.titled(file_name),
    ))
}

fn lay_out(colouring: &Colouring, order: Order) -> Result<Layout, NoRoom> {
    Layout::of(colouring, order.format, order.orientation, order.margin)
}

/// What a sheet made from a picture file is called: the picture's name, without its extension, then the cells across,
/// the colours and the paper, then landscape when asked, so variants of one picture sit side by side
/// without overwriting each other, then what the sheet is.
pub fn sheet_file_name(picture_file_name: &str, order: &Order) -> String {
    named_after(picture_file_name, order, "sheet")
}

/// What a sheet's preview made from a picture file is called: named like its sheet, as a preview, so the two sit side
/// by side.
pub fn preview_file_name(picture_file_name: &str, order: &Order) -> String {
    named_after(picture_file_name, order, "preview")
}

/// A sheet's or a preview's file name stamped with the moment it was made, in UTC to the second, before what it is and
/// its extension, so that a sheet made again never takes an older one's name: `goblin-48-8-a4-20261111T142915Z.sheet.png`.
/// A name with neither gets its stamp at the end.
pub fn stamped(file_name: &str, seconds_since_1970: u64) -> String {
    let stamp = utc(seconds_since_1970);
    match file_name.rsplitn(3, '.').collect::<Vec<_>>()[..] {
        [extension, what, name] => format!("{name}-{stamp}.{what}.{extension}"),
        _ => format!("{file_name}-{stamp}"),
    }
}

const SECONDS_A_DAY: u64 = 86_400;

/// A moment as its UTC date and time to the second, in ISO 8601's basic form: `20261111T142915Z`.
fn utc(seconds_since_1970: u64) -> String {
    let (days, second_of_day) = (
        seconds_since_1970 / SECONDS_A_DAY,
        seconds_since_1970 % SECONDS_A_DAY,
    );
    let (year, month, day) = calendar_date(days);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        second_of_day / 3600,
        second_of_day % 3600 / 60,
        second_of_day % 60
    )
}

/// The Gregorian year, month and day so many days after 1 January 1970, after Howard Hinnant's `civil_from_days`.
fn calendar_date(days_since_1970: u64) -> (u64, u64, u64) {
    let days = days_since_1970 + 719_468;
    let (era, day_of_era) = (days / 146_097, days % 146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = era * 400 + year_of_era + u64::from(month <= 2);
    (year, month, day)
}

fn named_after(picture_file_name: &str, order: &Order, what: &str) -> String {
    let stem = std::path::Path::new(picture_file_name)
        .file_stem()
        .map_or(picture_file_name.into(), |stem| stem.to_string_lossy());
    let paper = order.format.name().to_lowercase();
    let landscape = match order.orientation {
        Orientation::Portrait => "",
        Orientation::Landscape => "-landscape",
    };
    format!(
        "{stem}-{}-{}-{paper}{landscape}.{what}.png",
        order.cells_across,
        order.colours.get()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(columns: u32, format: PageFormat) -> Order {
        Order::new(
            columns,
            ColourCount::new(2).expect("a count"),
            format,
            Orientation::Portrait,
            Margin::new(20).expect("a margin"),
        )
        .expect("an order")
    }

    fn red_and_yellow() -> Picture {
        Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture")
    }

    /// A picture forty times taller than wide, whose rows no A5 page has room to number at 32 cells across.
    fn ribbon() -> Picture {
        Picture::from_rgba(1, 40, &[220, 30, 40, 255].repeat(40)).expect("a picture")
    }

    #[test]
    fn an_order_of_no_cell_across_or_more_than_its_paper_holds_is_refused_saying_the_most_it_holds()
    {
        let ordered = |cells_across, format| {
            Order::new(
                cells_across,
                ColourCount::new(8).expect("a count"),
                format,
                Orientation::Portrait,
                Margin::new(10).expect("a margin"),
            )
            .map(|order| order.cells_across())
        };

        let orders = [
            ordered(48, PageFormat::A4),
            ordered(49, PageFormat::A4),
            ordered(32, PageFormat::A5),
            ordered(33, PageFormat::A5),
            ordered(128, PageFormat::A0),
            ordered(0, PageFormat::A4),
        ];

        assert_eq!(
            orders,
            [
                Ok(48),
                Err(Refusal::MoreCellsAcrossThanThePaperHolds { most: 48 }),
                Ok(32),
                Err(Refusal::MoreCellsAcrossThanThePaperHolds { most: 32 }),
                Ok(128),
                Err(Refusal::NoCellAcross)
            ]
        );
    }

    #[test]
    fn a_grid_with_more_rows_than_its_page_could_ever_number_is_refused_before_any_cell_is_coloured()
     {
        let a_hundred_thousand_dots_tall =
            Picture::from_rgba(1, 100_000, &[220, 30, 40, 255].repeat(100_000)).expect("a picture");
        let ordered = [
            (red_and_yellow(), order(4, PageFormat::A4)),
            (ribbon(), order(32, PageFormat::A5)),
            (a_hundred_thousand_dots_tall, order(48, PageFormat::A4)),
        ];

        let grids = ordered.map(|(picture, order)| {
            order
                .grid_for(&picture)
                .map(|grid| (grid.columns(), grid.rows()))
        });

        assert_eq!(
            grids,
            [
                Ok((4, 2)),
                Err(NoRoom::ForTheNumbersInTheCells),
                Err(NoRoom::ForTheNumbersInTheCells)
            ]
        );
    }

    #[test]
    fn a_grid_is_refused_before_colouring_only_once_its_rows_need_cells_under_fifteen_dots_the_smallest_holding_a_digit()
     {
        let one_across = Order::new(
            1,
            ColourCount::new(2).expect("a count"),
            PageFormat::A4,
            Orientation::Portrait,
            Margin::new(10).expect("a margin"),
        )
        .expect("an order");
        let tall = |rows: u32| {
            Picture::from_rgba(1, rows, &[220, 30, 40, 255].repeat(rows as usize))
                .expect("a picture")
        };

        let grids = [218, 219].map(|rows| one_across.grid_for(&tall(rows)).map(|grid| grid.rows()));

        assert_eq!(grids, [Ok(218), Err(NoRoom::ForTheNumbersInTheCells)]);
    }

    #[test]
    fn a_picture_becomes_the_sheet_ordered_or_is_refused_when_the_page_lacks_room() {
        let ordered = [
            (red_and_yellow(), order(4, PageFormat::A4)),
            (ribbon(), order(32, PageFormat::A5)),
        ];

        let sheets = ordered.map(|(picture, order)| {
            make_sheet(&picture, order, "red-yellow.png")
                .map(|sheet| (sheet.width(), sheet.height()))
        });

        assert_eq!(
            sheets,
            [Ok((2480, 3508)), Err(NoRoom::ForTheNumbersInTheCells)]
        );
    }

    #[test]
    fn a_preview_is_its_sheet_coloured_in_on_the_same_page_and_refused_when_its_sheet_is() {
        let fits = order(4, PageFormat::A4);
        let sheet = make_sheet(&red_and_yellow(), fits, "red-yellow.png").expect("a sheet");
        let ordered = [
            (red_and_yellow(), fits),
            (ribbon(), order(32, PageFormat::A5)),
        ];

        let previews =
            ordered.map(|(picture, order)| make_preview(&picture, order, "red-yellow.png"));

        let [preview, refused] = previews;
        let preview = preview.expect("a preview");
        assert_eq!(
            (preview.width(), preview.height()),
            (sheet.width(), sheet.height())
        );
        assert!(preview != sheet, "a preview coloured in, unlike its sheet");
        assert_eq!(refused, Err(NoRoom::ForTheNumbersInTheCells));
    }

    #[test]
    fn a_colouring_laid_on_another_page_gives_the_sheet_and_preview_its_picture_would_on_that_page()
    {
        let picture = red_and_yellow();
        let colouring = colour(&picture, order(4, PageFormat::A4)).expect("a colouring");
        let on_a5 = order(4, PageFormat::A5);

        let laid = [
            sheet_of(&colouring, on_a5, "red-yellow.png"),
            preview_of(&colouring, on_a5, "red-yellow.png"),
        ];

        assert_eq!(
            laid,
            [
                make_sheet(&picture, on_a5, "red-yellow.png"),
                make_preview(&picture, on_a5, "red-yellow.png")
            ]
        );
        assert!(laid[0].as_ref().is_ok_and(|sheet| sheet.width() == 1748));
    }

    #[test]
    fn a_sheet_is_titled_with_the_file_name_it_is_given() {
        let picture = red_and_yellow();

        let [red, yellow] = ["red.png", "yellow.png"]
            .map(|name| make_sheet(&picture, order(4, PageFormat::A4), name));

        assert!(red.is_ok() && red != yellow);
    }

    #[test]
    fn a_name_is_stamped_with_the_utc_second_it_was_made_before_what_it_is() {
        let names = [
            stamped("goblin-48-8-a4.sheet.png", 0),
            stamped("my.photo-64-12-a3-landscape.preview.png", 1_794_407_355),
            stamped("goblin-32-8-a5.sheet.png", 1_835_397_296),
            stamped("goblin-32-8-a5.sheet.png", 1_835_481_599),
            stamped("goblin", 0),
            stamped("dragon.sheet.png", 951_782_400),
            stamped("dragon.sheet.png", 4_102_444_800),
        ];

        assert_eq!(
            names,
            [
                "goblin-48-8-a4-19700101T000000Z.sheet.png",
                "my.photo-64-12-a3-landscape-20261111T142915Z.preview.png",
                "goblin-32-8-a5-20280229T003456Z.sheet.png",
                "goblin-32-8-a5-20280229T235959Z.sheet.png",
                "goblin-19700101T000000Z",
                "dragon-20000229T000000Z.sheet.png",
                "dragon-21000101T000000Z.sheet.png",
            ]
        );
    }

    #[test]
    fn a_preview_is_named_like_its_sheet_as_a_preview() {
        let order = order(48, PageFormat::A4);

        let names = [
            preview_file_name("goblin.png", &order),
            preview_file_name("my.photo.jpeg", &order),
        ];

        assert_eq!(
            names,
            ["goblin-48-2-a4.preview.png", "my.photo-48-2-a4.preview.png"]
        );
    }

    #[test]
    fn a_sheet_is_named_after_its_picture_then_its_cells_across_colours_paper_and_any_landscape_then_what_it_is()
     {
        let named = |picture, (columns, colours), format, orientation| {
            let order = Order::new(
                columns,
                ColourCount::new(colours).expect("a count"),
                format,
                orientation,
                Margin::new(20).expect("a margin"),
            )
            .expect("an order");
            sheet_file_name(picture, &order)
        };
        let (portrait, landscape) = (Orientation::Portrait, Orientation::Landscape);

        let names = [
            named("goblin.png", (48, 8), PageFormat::A4, portrait),
            named("my.photo.jpeg", (64, 12), PageFormat::A3, landscape),
            named("goblin", (48, 2), PageFormat::A4, landscape),
            named("goblin.png", (32, 8), PageFormat::A5, portrait),
        ];

        assert_eq!(
            names,
            [
                "goblin-48-8-a4.sheet.png",
                "my.photo-64-12-a3-landscape.sheet.png",
                "goblin-48-2-a4-landscape.sheet.png",
                "goblin-32-8-a5.sheet.png"
            ]
        );
    }
}
