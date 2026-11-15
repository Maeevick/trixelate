//! Turns a picture into a colour-by-number pixel sheet, ready to print.

pub mod colouring;
mod drawing;
pub mod grid;
pub mod jpeg_file;
pub mod layout;
pub mod lettering;
pub mod palette;
pub mod pdf_file;
#[cfg(test)]
mod pdf_reading;
pub mod picture;
pub mod picture_file;
pub mod pieces;
pub mod png_file;
pub mod sheet;
pub mod warning;

use std::num::NonZeroU32;

use colouring::Colouring;
use grid::GridSize;
use layout::{Layout, Margin, NoRoom, Orientation, PageFormat};
use palette::ColourCount;
use picture::Picture;
use pieces::Pieces;
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

    /// How the sheet is cut into A4 pieces for a home printer, none for a sheet it takes whole.
    pub fn pieces(&self) -> Option<Pieces> {
        Pieces::of(self.format, self.orientation)
    }

    /// The same order on a page lying the other way, as a picture suits it.
    pub fn lying(self, orientation: Orientation) -> Order {
        Order {
            orientation,
            ..self
        }
    }

    /// The grid a picture is simplified into for this order: the cells across asked, and the rows that keep the
    /// picture's proportions, refused when the page could never number that many rows, before any cell is coloured.
    pub fn grid_for(&self, picture: &Picture) -> Result<GridSize, NoRoom> {
        let grid = GridSize::keeping_the_proportions_of(picture, self.cells_across);
        layout::room_to_number(grid, self.format, self.orientation, self.margin)?;
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

/// Where trixelate's code lives, under the MIT licence.
pub const SOURCE: &str = "https://github.com/Maeevick/trixelate";
/// Where trixelate's page lives, to make a sheet in a browser.
pub const WEBSITE: &str = "https://trixelate.maeevick.com";

/// The sheet of a colouring as a PDF of one page at its paper's size, drawn in shapes rather than dots, so it prints
/// sharp at any size, saying its title, when it was made, in seconds since 1970, and the trixelate that made it.
pub fn sheet_pdf(
    colouring: &Colouring,
    order: Order,
    file_name: &str,
    made: u64,
) -> Result<Vec<u8>, NoRoom> {
    let drawing = sheet::drawing(colouring, &lay_out(colouring, order)?.titled(file_name));
    Ok(pdf_file::write_pages(
        &[pdf_file::page_of(&drawing)],
        &pdf_file::characters_of(&drawing),
        &about(
            file_name,
            "A colour-by-number sheet, to colour in by hand",
            made,
        ),
    ))
}

/// The preview of a colouring's sheet as a PDF of one page, the sheet as it looks once coloured in, said to be one.
pub fn preview_pdf(
    colouring: &Colouring,
    order: Order,
    file_name: &str,
    made: u64,
) -> Result<Vec<u8>, NoRoom> {
    let drawing =
        sheet::drawing_coloured_in(colouring, &lay_out(colouring, order)?.titled(file_name));
    Ok(pdf_of(
        &drawing,
        about(
            file_name,
            "A colour-by-number sheet's preview, coloured in",
            made,
        ),
    ))
}

/// A colouring's sheet cut into A4 pieces for a home printer, as a PDF of a page each, row after row, saying what it is,
/// or none for a sheet a home printer takes whole.
pub fn pieces_pdf(
    colouring: &Colouring,
    order: Order,
    file_name: &str,
    made: u64,
) -> Result<Option<Vec<u8>>, NoRoom> {
    let drawing = sheet::drawing(colouring, &lay_out(colouring, order)?.titled(file_name));
    Ok(order.pieces().map(|pieces| {
        let (pages, characters) = pieces.pages(&drawing);
        let subject =
            "A colour-by-number poster in A4 pieces, to print at actual size and glue together";
        pdf_file::write_pages(&pages, &characters, &about(file_name, subject, made))
    }))
}

/// A drawing as a PDF of one page, saying what it is about.
fn pdf_of(drawing: &drawing::Drawing, about: pdf_file::About) -> Vec<u8> {
    pdf_file::write_pages(
        &[pdf_file::page_of(drawing)],
        &pdf_file::characters_of(drawing),
        &about,
    )
}

/// What a PDF made by trixelate says of itself.
fn about(title: &str, subject: &str, made: u64) -> pdf_file::About {
    let version = env!("CARGO_PKG_VERSION");
    pdf_file::About {
        title: title.to_string(),
        subject: subject.to_string(),
        keywords: "colour by number, pixel art, trixelate".to_string(),
        creator: format!("trixelate {version}, {WEBSITE}"),
        producer: format!("trixelate {version}, under the MIT licence, {SOURCE}"),
        made: format!("D:{}", utc(made).replace('T', "")),
    }
}

fn lay_out(colouring: &Colouring, order: Order) -> Result<Layout, NoRoom> {
    Layout::of(colouring, order.format, order.orientation, order.margin)
}

/// What a sheet made from a picture file is called: the picture's name, without its extension, then the cells across,
/// the colours and the paper, then landscape when asked, so variants of one picture sit side by side
/// without overwriting each other, then what the sheet is.
pub fn sheet_file_name(picture_file_name: &str, order: &Order) -> String {
    named_after(picture_file_name, order, "sheet.pdf")
}

/// What a sheet's pieces made from a picture file are called: named like their sheet, as pieces, in the PDF that holds
/// them.
pub fn pieces_file_name(picture_file_name: &str, order: &Order) -> String {
    named_after(picture_file_name, order, "pieces.pdf")
}

/// What a sheet's preview made from a picture file is called: named like its sheet, as a preview, so the two sit side
/// by side.
pub fn preview_file_name(picture_file_name: &str, order: &Order) -> String {
    named_after(picture_file_name, order, "preview.pdf")
}

/// A sheet's or a preview's file name stamped with the moment it was made, in UTC to the second, before what it is and
/// its extension, so that a sheet made again never takes an older one's name: `goblin-32-4-a4-20261111T142915Z.sheet.pdf`.
/// A name with neither gets its stamp at the end. Sheets already made under that name in that second number this one
/// after its stamp, from two: `goblin-32-4-a4-20261111T142915Z-2.sheet.pdf`.
pub fn stamped(file_name: &str, seconds_since_1970: u64, made_before: u32) -> String {
    let stamp = match made_before {
        0 => utc(seconds_since_1970),
        _ => format!("{}-{}", utc(seconds_since_1970), made_before + 1),
    };
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

fn named_after(picture_file_name: &str, order: &Order, what_and_its_extension: &str) -> String {
    let stem = std::path::Path::new(picture_file_name)
        .file_stem()
        .map_or(picture_file_name.into(), |stem| stem.to_string_lossy());
    let paper = order.format.name().to_lowercase();
    let landscape = match order.orientation {
        Orientation::Portrait => "",
        Orientation::Landscape => "-landscape",
    };
    format!(
        "{stem}-{}-{}-{paper}{landscape}.{what_and_its_extension}",
        order.cells_across,
        order.colours.get()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picture::Rgb;
    use std::collections::{BTreeMap, BTreeSet};

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
            ordered(80, PageFormat::A2),
            ordered(0, PageFormat::A4),
        ];

        assert_eq!(
            orders,
            [
                Ok(48),
                Err(Refusal::MoreCellsAcrossThanThePaperHolds { most: 48 }),
                Ok(32),
                Err(Refusal::MoreCellsAcrossThanThePaperHolds { most: 32 }),
                Ok(80),
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

    /// A three by two picture of four colours, one of them dark, coloured for a sheet of 12 cells across on A5.
    fn four_colours_on_a5() -> (Colouring, Order) {
        let picture = Picture::from_rgba(
            3,
            2,
            &[
                [220, 30, 40, 255],
                [250, 220, 60, 255],
                [20, 60, 200, 255],
                [20, 60, 200, 255],
                [250, 220, 60, 255],
                [30, 30, 30, 255],
            ]
            .concat(),
        )
        .expect("a picture");
        let order = Order::new(
            12,
            ColourCount::new(4).expect("a count"),
            PageFormat::A5,
            Orientation::Portrait,
            Margin::new(10).expect("a margin"),
        )
        .expect("an order");
        (colour(&picture, order).expect("room"), order)
    }

    /// Each page of a PDF drawn dot by dot, as its width, its height and how many of its dots differ from a sheet's.
    fn pages_against(pdf: &[u8], sheet: &Sheet) -> Vec<(u32, u32, usize)> {
        pdf_reading::pages_drawn(pdf, 300)
            .into_iter()
            .map(|(width, height, dots)| {
                let differing = dots
                    .iter()
                    .enumerate()
                    .filter(|&(at, dot)| {
                        let (x, y) = (at as u32 % width, at as u32 / width);
                        x >= sheet.width() || y >= sheet.height() || sheet.pixel(x, y) != *dot
                    })
                    .count();
                (width, height, differing)
            })
            .collect()
    }

    #[test]
    fn a_sheet_written_as_a_pdf_draws_every_dot_of_the_sheet_in_shapes_on_a_page_of_its_papers_size()
    -> Result<(), NoRoom> {
        let (colouring, order) = four_colours_on_a5();
        let sheet = sheet_of(&colouring, order, "goblin-12-4-a5.sheet.pdf")?;

        let pdf = sheet_pdf(&colouring, order, "goblin-12-4-a5.sheet.pdf", 0)?;

        assert_eq!(
            pages_against(&pdf, &sheet),
            [(sheet.width(), sheet.height(), 0)]
        );
        Ok(())
    }

    #[test]
    fn a_preview_written_as_a_pdf_draws_every_dot_of_the_preview_in_shapes_saying_it_is_one()
    -> Result<(), NoRoom> {
        let (colouring, order) = four_colours_on_a5();
        let preview = preview_of(&colouring, order, "goblin-12-4-a5.preview.pdf")?;

        let pdf = preview_pdf(&colouring, order, "goblin-12-4-a5.preview.pdf", 0)?;

        assert_eq!(
            pages_against(&pdf, &preview),
            [(preview.width(), preview.height(), 0)]
        );
        assert_eq!(
            pdf_reading::info_of(&pdf)
                .get("Subject")
                .map(String::as_str),
            Some("A colour-by-number sheet's preview, coloured in")
        );
        Ok(())
    }

    #[test]
    fn a_sheets_pdf_says_its_title_when_it_was_made_and_the_trixelate_that_made_it_with_where_it_lives()
     {
        let (colouring, order) = four_colours_on_a5();
        let version = env!("CARGO_PKG_VERSION");

        let pdf = sheet_pdf(
            &colouring,
            order,
            "goblin (2)-12-4-a5.sheet.pdf",
            1_794_407_355,
        )
        .expect("room");

        let info = pdf_reading::info_of(&pdf);
        let expected: BTreeMap<String, String> = [
            ("CreationDate", "D:20261111142915Z".to_string()),
            (
                "Creator",
                format!("trixelate {version}, https://trixelate.maeevick.com"),
            ),
            (
                "Keywords",
                "colour by number, pixel art, trixelate".to_string(),
            ),
            (
                "Producer",
                format!(
                    "trixelate {version}, under the MIT licence, https://github.com/Maeevick/trixelate"
                ),
            ),
            (
                "Subject",
                "A colour-by-number sheet, to colour in by hand".to_string(),
            ),
            ("Title", "goblin (2)-12-4-a5.sheet.pdf".to_string()),
        ]
        .map(|(key, value)| (key.to_string(), value))
        .into();
        assert_eq!(info, expected);
    }

    #[test]
    fn a_posters_pieces_are_a_pdf_of_an_a4_page_each_its_sheet_drawn_in_a_window_inside_the_margin_each_starting_where_the_last_shares_its_strip()
     {
        let (colouring, _) = four_colours_on_a5();
        let order = Order::new(
            12,
            ColourCount::new(4).expect("a count"),
            PageFormat::A2,
            Orientation::Portrait,
            Margin::new(10).expect("a margin"),
        )
        .expect("an order");
        let sheet = sheet_pdf(&colouring, order, "goblin-12-4-a2.pieces.pdf", 0).expect("room");
        let (_, drawing) = pdf_reading::pages_of(&sheet).remove(0);
        let (margin, part, apart) = (118.0, (2244.0, 3272.0), (2126.0, 3154.0));
        let points = |dots: f64| dots * 72.0 / 300.0;

        let pdf = pieces_pdf(&colouring, order, "goblin-12-4-a2.pieces.pdf", 0)
            .expect("room")
            .expect("pieces");

        let pages = pdf_reading::pages_of(&pdf);
        assert_eq!(pages.len(), 4);
        for (at, (media_box, content)) in pages.iter().enumerate() {
            let (column, row) = (at as f64 % 2.0, (at / 2) as f64);
            let numbers_before = |operator: &str| -> Vec<f64> {
                let (before, _) = content.split_once(operator).unwrap_or_default();
                let mut numbers: Vec<f64> = before
                    .split_whitespace()
                    .rev()
                    .map_while(|token| token.parse().ok())
                    .collect();
                numbers.reverse();
                numbers
            };
            let window = numbers_before(" re W n");
            let moved = numbers_before(" cm\n");
            let (scale, right, up) = (moved[0], moved[4], moved[5]);
            let corner = (
                points(column * apart.0 / scale),
                points(7016.0 - row * apart.1 / scale),
            );
            assert_eq!(media_box, "0 0 595.2 841.92");
            assert_eq!(
                window,
                [margin, margin, part.0, part.1].map(points),
                "piece {at}"
            );
            assert_eq!(moved[1..3], [0.0, 0.0]);
            assert_eq!(moved[3], scale);
            assert!(
                (scale * corner.0 + right - points(margin)).abs() < 0.01,
                "piece {at}"
            );
            assert!(
                (scale * corner.1 + up - points(margin + part.1)).abs() < 0.01,
                "piece {at}"
            );
            assert!(content.contains(&drawing), "piece {at} draws its sheet");
        }
    }

    #[test]
    fn a_piece_is_marked_where_to_cut_it_on_its_left_and_top_and_says_where_it_goes_its_margin_blank_besides()
     {
        let (colouring, _) = four_colours_on_a5();
        let order = Order::new(
            12,
            ColourCount::new(4).expect("a count"),
            PageFormat::A2,
            Orientation::Portrait,
            Margin::new(10).expect("a margin"),
        )
        .expect("an order");
        let (black, grey) = (Rgb(0, 0, 0), Rgb(90, 90, 90));
        let (margin, part) = (118, (2244, 3272));
        let within = |(left, top, width, height): (u32, u32, u32, u32)| {
            (top..top + height).flat_map(move |y| (left..left + width).map(move |x| (x, y)))
        };
        let cut_marks: BTreeSet<_> = [
            (margin, margin - 12 - 59, 4, 59),
            (margin, margin + part.1 + 12, 4, 59),
            (margin - 12 - 59, margin, 59, 4),
            (margin + part.0 + 12, margin, 59, 4),
        ]
        .into_iter()
        .flat_map(within)
        .collect();
        let place = "row 2 of 2 - column 2 of 2";
        let (place_width, _) = lettering::size(place, 5);
        let written: BTreeSet<_> = lettering::ink(place, 5)
            .into_iter()
            .map(|(x, y)| (2480 - margin - place_width + x, 3508 - margin + 12 + y))
            .collect();

        let pdf = pieces_pdf(&colouring, order, "goblin-12-4-a2.pieces.pdf", 0)
            .expect("room")
            .expect("pieces");

        let (width, height, dots) = pdf_reading::pages_drawn(&pdf, 300).remove(3);
        let in_the_window = |&(x, y): &(u32, u32)| {
            (margin..margin + part.0).contains(&x) && (margin..margin + part.1).contains(&y)
        };
        let misdrawn = within((0, 0, width, height))
            .filter(|dot| !in_the_window(dot))
            .filter(|&(x, y)| {
                let expected = if cut_marks.contains(&(x, y)) {
                    black
                } else if written.contains(&(x, y)) {
                    grey
                } else {
                    Rgb::WHITE
                };
                dots[(y * width + x) as usize] != expected
            })
            .count();
        let inked_in_the_window = within((margin, margin, part.0, part.1))
            .filter(|&(x, y)| dots[(y * width + x) as usize] != Rgb::WHITE)
            .count();
        assert_eq!((width, height), (2480, 3508));
        assert!(!written.is_empty() && inked_in_the_window > 0);
        assert_eq!(misdrawn, 0);
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
            stamped("goblin-48-8-a4.sheet.png", 0, 0),
            stamped("my.photo-64-12-a3-landscape.preview.png", 1_794_407_355, 0),
            stamped("goblin-32-8-a5.sheet.png", 1_835_397_296, 0),
            stamped("goblin-32-8-a5.sheet.png", 1_835_481_599, 0),
            stamped("goblin", 0, 0),
            stamped("dragon.sheet.png", 951_782_400, 0),
            stamped("dragon.sheet.png", 4_102_444_800, 0),
            stamped("dragon.sheet.png", 4_107_542_400, 0),
            stamped("dragon.sheet.png", 13_569_465_599, 0),
            stamped("dragon.sheet.png", 13_574_606_400, 0),
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
                "dragon-21000301T000000Z.sheet.png",
                "dragon-23991231T235959Z.sheet.png",
                "dragon-24000229T120000Z.sheet.png",
            ]
        );
    }

    #[test]
    fn a_sheet_made_again_in_a_second_already_taken_is_numbered_after_its_stamp_from_two() {
        let names = [
            stamped("goblin-48-8-a4.sheet.png", 0, 1),
            stamped("goblin-48-8-a4.sheet.png", 0, 2),
            stamped("goblin", 0, 1),
        ];

        assert_eq!(
            names,
            [
                "goblin-48-8-a4-19700101T000000Z-2.sheet.png",
                "goblin-48-8-a4-19700101T000000Z-3.sheet.png",
                "goblin-19700101T000000Z-2",
            ]
        );
    }

    #[test]
    fn a_preview_is_named_like_its_sheet_as_a_preview_in_a_pdf() {
        let order = order(48, PageFormat::A4);

        let names = [
            preview_file_name("goblin.png", &order),
            preview_file_name("my.photo.jpeg", &order),
        ];

        assert_eq!(
            names,
            ["goblin-48-2-a4.preview.pdf", "my.photo-48-2-a4.preview.pdf"]
        );
    }

    #[test]
    fn the_pieces_of_a_sheet_are_named_like_it_as_pieces_in_a_pdf() {
        let order = order(80, PageFormat::A2);

        let names = [
            pieces_file_name("goblin.png", &order),
            pieces_file_name("my.photo.jpeg", &order),
        ];

        assert_eq!(
            names,
            ["goblin-80-2-a2.pieces.pdf", "my.photo-80-2-a2.pieces.pdf"]
        );
    }

    #[test]
    fn a_sheet_is_named_after_its_picture_then_its_cells_across_colours_paper_and_any_landscape_as_a_sheet_in_a_pdf()
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
                "goblin-48-8-a4.sheet.pdf",
                "my.photo-64-12-a3-landscape.sheet.pdf",
                "goblin-48-2-a4-landscape.sheet.pdf",
                "goblin-32-8-a5.sheet.pdf"
            ]
        );
    }
}
