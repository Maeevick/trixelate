//! The form's settings turned into an order for a sheet, the sheet painted as PNG bytes, and what the page says when
//! one is refused, worded for a parent.

use trixelate::layout::{Margin, NoRoom, Orientation, PageFormat, most_cells_across};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::{Order, Refusal, fits_on_its_page, png_file, sheet_file_name};

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

/// The order for the sheet asked for, once sure its page has room for it, or why it would be refused.
pub fn ready(picture: &Picture, settings: Settings) -> Result<Order, String> {
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
    let order = Order::new(
        settings.width,
        colours,
        settings.format,
        orientation,
        margin,
    )
    .map_err(|refusal| refused(settings.format, refusal))?;
    fits_on_its_page(picture, order).map_err(explain)?;
    Ok(order)
}

/// How a sheet or its preview is drawn from a picture, its order and the name it is titled with.
pub type Making = fn(&Picture, Order, &str) -> Result<Picture, NoRoom>;

/// The sheet ordered, or its preview, as PNG bytes, both titled with the name the sheet is saved under so that only
/// their cells differ, or why it cannot be made.
pub fn paint(picture: &Picture, name: &str, order: Order, make: Making) -> Result<Vec<u8>, String> {
    let sheet = make(picture, order, &sheet_file_name(name, &order)).map_err(explain)?;
    png_file::write(&sheet, order.format().dots_per_inch())
        .map_err(|_| "The sheet could not be turned into a PNG. That one is on us.".to_string())
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
    use trixelate::make_sheet;

    fn red_and_yellow() -> Picture {
        Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture")
    }

    #[test]
    fn the_form_starts_on_an_a4_sheet_as_many_cells_across_as_it_holds_in_eight_colours_with_a_ten_millimetre_margin()
     {
        let order = ready(&red_and_yellow(), Settings::default());

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
            (red_and_yellow(), asked(|settings| settings.width = 49)),
            (red_and_yellow(), asked(|settings| settings.width = 0)),
            (red_and_yellow(), asked(|settings| settings.colours = 25)),
            (red_and_yellow(), asked(|settings| settings.margin = 4)),
            (ribbon, asked(|_| ())),
        ];

        let said = refusals.map(|(picture, settings)| ready(&picture, settings).err());

        assert_eq!(
            said.map(Option::unwrap_or_default),
            [
                "An A4 sheet holds at most 48 cells across, or little fingers give up. Fewer cells, or bigger paper?",
                "A grid needs at least one cell across.",
                "A sheet holds from 2 to 24 colours, like a big box of crayons.",
                "A margin from 5 mm, or the title and the credit fall off the page, to 20 mm, or the cells shrink.",
                "The cells got too small for their numbers. Fewer cells across, or bigger paper?",
            ]
        );
    }

    #[test]
    fn a_sheet_is_painted_as_png_bytes_of_its_whole_page_upright_or_on_its_side() {
        let picture = red_and_yellow();
        let on_its_side = Settings {
            landscape: true,
            ..Settings::default()
        };

        let pages = [Settings::default(), on_its_side].map(|settings| {
            ready(&picture, settings)
                .and_then(|order| paint(&picture, "red-yellow.png", order, make_sheet))
                .ok()
                .and_then(|bytes| png_file::read(&bytes).ok())
                .map(|sheet| (sheet.width(), sheet.height()))
        });

        assert_eq!(pages, [Some((2480, 3508)), Some((3508, 2480))]);
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
