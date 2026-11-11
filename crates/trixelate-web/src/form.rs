//! The form's settings turned into an order for a sheet, the sheet painted as PNG bytes, and what the page says when
//! one is refused, worded for a parent.

use trixelate::colouring::Colouring;
use trixelate::layout::{Margin, NoRoom, Orientation, PageFormat, most_cells_across};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::picture_file::{MOST_MEGABYTES, MOST_MEGAPIXELS, Unreadable};
use trixelate::sheet::Sheet;
use trixelate::warning::warnings_for;
use trixelate::{Order, Refusal, colour, preview_of, sheet_file_name, sheet_of};

/// What the sheet is asked to be, as the form sets it.
#[derive(Clone, Copy, PartialEq)]
pub struct Settings {
    pub width: u32,
    pub colours: usize,
    pub format: PageFormat,
    pub landscape: bool,
    pub margin: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            width: most_cells_across(PageFormat::A4),
            colours: ColourCount::IN_A_SMALL_BOX.get(),
            format: PageFormat::A4,
            landscape: false,
            margin: Margin::AROUND_A_SHEET.millimetres(),
        }
    }
}

impl Settings {
    /// The settings on another paper, the cells across following it when they were the most the old paper holds or
    /// are more than the new one holds, as the command line takes the most a paper holds unless told otherwise.
    pub fn on_paper(self, format: PageFormat) -> Settings {
        let (was_the_most, most) = (
            self.width == most_cells_across(self.format),
            most_cells_across(format),
        );
        let width = if was_the_most || self.width > most {
            most
        } else {
            self.width
        };
        Settings {
            width,
            format,
            ..self
        }
    }
}

/// The order for the sheet asked for, or why it would be refused, before any picture is coloured.
pub fn ready(settings: Settings) -> Result<Order, String> {
    let colours = ColourCount::new(settings.colours).map_err(|_| {
        "A sheet holds from 2 to 24 colours, like a big box of crayons.".to_string()
    })?;
    let margin = Margin::new(settings.margin).map_err(|_| {
        "A margin from 5 mm, or the title and the credit fall off the page, to 20 mm, or the cells shrink.".to_string()
    })?;
    let orientation = if settings.landscape {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    };
    Order::new(
        settings.width,
        colours,
        settings.format,
        orientation,
        margin,
    )
    .map_err(|refusal| refused(settings.format, refusal))
}

/// A picture coloured for an order, the slow part of a sheet, or why its page could never number its cells.
pub fn coloured(picture: &Picture, order: Order) -> Result<Colouring, String> {
    colour(picture, order).map_err(explain)
}

/// A sheet and its preview as PNG bytes, both titled with the name the sheet is saved under so that only their cells
/// differ.
pub struct Painted {
    pub sheet: Vec<u8>,
    pub preview: Vec<u8>,
    pub name: String,
}

/// A colouring's sheet and preview painted on the order's page, named after the picture, or why they cannot be.
pub fn paint(colouring: &Colouring, picture_name: &str, order: Order) -> Result<Painted, String> {
    let name = sheet_file_name(picture_name, &order);
    let png = |sheet: Sheet| {
        sheet
            .png()
            .map_err(|_| "The sheet could not be turned into a PNG. That one is on us.".to_string())
    };
    Ok(Painted {
        sheet: png(sheet_of(colouring, order, &name).map_err(explain)?)?,
        preview: png(preview_of(colouring, order, &name).map_err(explain)?)?,
        name,
    })
}

fn refused(format: PageFormat, refusal: Refusal) -> String {
    match refusal {
        Refusal::NoCellAcross => "A grid needs at least one cell across.".to_string(),
        Refusal::MoreCellsAcrossThanThePaperHolds { most } => format!(
            "An {} sheet holds at most {most} cells across, or little fingers give up. Fewer cells, or bigger paper?",
            format.name()
        ),
    }
}

fn explain(no_room: NoRoom) -> String {
    match no_room {
        NoRoom::ForTheNumbersInTheCells => {
            "The cells got too small for their numbers. Fewer cells across, or bigger paper?"
        }
    }
    .to_string()
}

/// Whether a file of this many bytes is light enough to read as a picture.
pub fn light_enough(bytes: f64) -> bool {
    bytes <= (MOST_MEGABYTES * 1_000_000) as f64
}

/// Why a dropped file gives no picture, worded for a parent: too heavy to read, or not a picture the goblins can read.
pub fn not_a_picture(why: Option<Unreadable>) -> String {
    match why {
        None => format!(
            "That file weighs over {MOST_MEGABYTES} MB, more than any picture a sheet needs. Shrink it a little first."
        ),
        Some(Unreadable::NeitherPngNorJpeg) => {
            "That does not look like a PNG or a JPEG. Goblins only come in those two flavours."
                .to_string()
        }
        Some(Unreadable::TooBig { megapixels }) => format!(
            "That picture is {megapixels} megapixels. The goblins counting its dots gave up at {MOST_MEGAPIXELS}: shrink it a little first."
        ),
    }
}

/// What the settings are warned of, each as a sentence on the page, the cells' warning first.
pub fn warned(settings: Settings) -> Vec<String> {
    ColourCount::new(settings.colours)
        .map(|colours| warnings_for(settings.width, colours))
        .unwrap_or_default()
        .into_iter()
        .map(|warning| sentence(warning.says()))
        .collect()
}

/// A text started with a capital, as a sentence on the page.
pub fn sentence(text: &str) -> String {
    let mut characters = text.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trixelate::png_file;

    fn red_and_yellow() -> Picture {
        Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture")
    }

    #[test]
    fn the_form_starts_on_an_a4_sheet_as_many_cells_across_as_it_holds_in_eight_colours_with_a_ten_millimetre_margin()
     {
        let order = ready(Settings::default());

        assert_eq!(
            order.map(|order| (order.cells_across(), order.colours().get(), order.format())),
            Ok((48, 8, PageFormat::A4))
        );
        assert_eq!(Settings::default().margin, 10);
        assert!(!Settings::default().landscape);
    }

    #[test]
    fn a_refused_sheet_says_why_and_what_to_try_worded_for_a_parent() {
        let ribbon = Picture::from_rgba(1, 40, &[220, 30, 40, 255].repeat(40)).expect("a picture");
        let asked = |change: fn(&mut Settings)| {
            let mut settings = Settings::default();
            change(&mut settings);
            settings
        };
        let refusals = [
            asked(|settings| settings.width = 49),
            asked(|settings| settings.width = 0),
            asked(|settings| settings.colours = 25),
            asked(|settings| settings.margin = 4),
        ];

        let said = refusals.map(|settings| ready(settings).err());
        let too_tall = ready(Settings::default()).and_then(|order| coloured(&ribbon, order));

        assert_eq!(
            said.map(Option::unwrap_or_default),
            [
                "An A4 sheet holds at most 48 cells across, or little fingers give up. Fewer cells, or bigger paper?",
                "A grid needs at least one cell across.",
                "A sheet holds from 2 to 24 colours, like a big box of crayons.",
                "A margin from 5 mm, or the title and the credit fall off the page, to 20 mm, or the cells shrink.",
            ]
        );
        assert_eq!(
            too_tall.err().as_deref(),
            Some("The cells got too small for their numbers. Fewer cells across, or bigger paper?")
        );
    }

    #[test]
    fn a_sheet_and_its_preview_are_painted_as_png_bytes_of_their_whole_page_upright_or_on_its_side_named_as_the_sheet()
     {
        let picture = red_and_yellow();
        let on_its_side = Settings {
            landscape: true,
            ..Settings::default()
        };
        let page = |bytes: &[u8]| {
            png_file::read(bytes)
                .ok()
                .map(|sheet| (sheet.width(), sheet.height()))
        };

        let painted = [Settings::default(), on_its_side].map(|settings| {
            ready(settings)
                .and_then(|order| paint(&coloured(&picture, order)?, "red-yellow.png", order))
                .map(|painted| (page(&painted.sheet), page(&painted.preview), painted.name))
        });

        assert_eq!(
            painted,
            [
                Ok((
                    Some((2480, 3508)),
                    Some((2480, 3508)),
                    "red-yellow-48-8-a4.sheet.png".to_string()
                )),
                Ok((
                    Some((3508, 2480)),
                    Some((3508, 2480)),
                    "red-yellow-48-8-a4-landscape.sheet.png".to_string()
                )),
            ]
        );
    }

    #[test]
    fn a_file_over_a_hundred_megabytes_or_a_picture_over_fifty_megapixels_is_refused_worded_for_a_parent()
     {
        let weighed = [100_000_000.0, 100_000_001.0].map(light_enough);
        let said = [
            None,
            Some(Unreadable::NeitherPngNorJpeg),
            Some(Unreadable::TooBig { megapixels: 64 }),
        ]
        .map(not_a_picture);

        assert_eq!(weighed, [true, false]);
        assert_eq!(
            said,
            [
                "That file weighs over 100 MB, more than any picture a sheet needs. Shrink it a little first.",
                "That does not look like a PNG or a JPEG. Goblins only come in those two flavours.",
                "That picture is 64 megapixels. The goblins counting its dots gave up at 50: shrink it a little first.",
            ]
        );
    }

    #[test]
    fn the_cells_across_follow_the_paper_when_they_were_the_most_it_holds_or_are_more_than_the_new_one_holds()
     {
        let across = |width, from, to| {
            let settings = Settings {
                width,
                format: from,
                ..Settings::default()
            };
            let moved = settings.on_paper(to);
            (moved.format.name(), moved.width)
        };

        let widths = [
            across(48, PageFormat::A4, PageFormat::A5),
            across(48, PageFormat::A4, PageFormat::A0),
            across(20, PageFormat::A4, PageFormat::A5),
            across(40, PageFormat::A4, PageFormat::A5),
            across(20, PageFormat::A4, PageFormat::A0),
        ];

        assert_eq!(
            widths,
            [("A5", 32), ("A0", 128), ("A5", 20), ("A5", 32), ("A0", 20)]
        );
    }

    #[test]
    fn every_warning_is_said_as_a_sentence_the_cells_first() {
        let few = Settings {
            width: 8,
            colours: 3,
            ..Settings::default()
        };

        let said = warned(few);

        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].starts_with("With so few cells across"));
        assert!(said[1].starts_with("With so few colours"));
        assert_eq!(warned(Settings::default()), Vec::<String>::new());
    }

    #[test]
    fn a_warning_reads_as_a_sentence_on_the_page() {
        let [warning, nothing] = ["with so few colours, it will be", ""].map(sentence);

        assert_eq!(
            [warning.as_str(), nothing.as_str()],
            ["With so few colours, it will be", ""]
        );
    }
}
