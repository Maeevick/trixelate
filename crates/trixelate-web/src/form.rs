//! The form's settings turned into an order for a sheet, the sheet painted as PNG bytes or its pieces as a PDF, and what
//! the page says when one is refused, worded for a parent.

use std::sync::Arc;

use trixelate::colouring::Colouring;
use trixelate::layout::{
    Margin, NoRoom, Orientation, PageFormat, most_cells_across, usual_cells_across, usual_colours,
};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::picture_file::{MOST_MEGABYTES, MOST_MEGAPIXELS, Unreadable};
use trixelate::pieces::Pieces;
use trixelate::png_file::CannotWritePng;
use trixelate::sheet::Sheet;
use trixelate::warning::warnings_for;
use trixelate::{
    Order, Refusal, colour, pieces_file_name, pieces_pdf, preview_of, sheet_file_name, sheet_of,
    sheet_pdf,
};

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
            width: usual_cells_across(PageFormat::A4),
            colours: usual_colours(PageFormat::A4).get(),
            format: PageFormat::A4,
            landscape: false,
            margin: Margin::AROUND_A_SHEET.millimetres(),
        }
    }
}

impl Settings {
    /// The settings for a picture dropped in: the paper on its side when the picture suits it, upright otherwise.
    pub fn for_picture(self, picture: &Picture) -> Settings {
        Settings {
            landscape: Orientation::suiting(picture) == Orientation::Landscape,
            ..self
        }
    }

    /// The settings on another paper, the cells across and the colours following it when they were the old paper's
    /// usual, as the command line takes a paper's usual unless told otherwise, and the cells never more than it holds.
    pub fn on_paper(self, format: PageFormat) -> Settings {
        let width = if self.width == usual_cells_across(self.format) {
            usual_cells_across(format)
        } else {
            self.width.min(most_cells_across(format))
        };
        let colours = if self.colours == usual_colours(self.format).get() {
            usual_colours(format).get()
        } else {
            self.colours
        };
        Settings {
            width,
            colours,
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

/// A sheet as PNG bytes to show or a PDF to print, or its pieces as a PDF, and the name they are saved under, which
/// the sheet's preview is titled with too, so that only their cells differ.
pub struct Painted {
    pub bytes: Vec<u8>,
    pub name: String,
}

const CANNOT_SHOW_THE_SHEET: &str = "The sheet could not be turned into a PNG. That one is on us.";
const PRINTS_WHOLE: &str = "This sheet prints whole at home: pieces are for an A3 or A2 sheet.";

/// A colouring's sheet painted on the order's page, named after the picture, encoded by `encode` to be shown only,
/// or why it cannot be.
pub fn paint_sheet(
    colouring: &Colouring,
    picture_name: &str,
    order: Order,
    encode: impl Fn(&Sheet) -> Result<Vec<u8>, CannotWritePng>,
) -> Result<Painted, String> {
    let name = sheet_file_name(picture_name, &order);
    let sheet = sheet_of(colouring, order, &name).map_err(explain)?;
    Ok(Painted {
        bytes: encode(&sheet).map_err(|_| CANNOT_SHOW_THE_SHEET.to_string())?,
        name,
    })
}

/// A colouring's sheet as a PDF to print, named after the picture and saying when it was made, in seconds since 1970,
/// or why it cannot be.
pub fn sheet_to_print(
    colouring: &Colouring,
    picture_name: &str,
    order: Order,
    made: u64,
) -> Result<Painted, String> {
    let name = sheet_file_name(picture_name, &order);
    Ok(Painted {
        bytes: sheet_pdf(colouring, order, &name, made).map_err(explain)?,
        name,
    })
}

/// A colouring's preview painted on the order's page, titled as its sheet, encoded by `encode` to be shown only, or why
/// it cannot be.
pub fn paint_preview(
    colouring: &Colouring,
    title: &str,
    order: Order,
    encode: impl Fn(&Sheet) -> Result<Vec<u8>, CannotWritePng>,
) -> Result<Vec<u8>, String> {
    encode(&preview_of(colouring, order, title).map_err(explain)?)
        .map_err(|_| CANNOT_SHOW_THE_SHEET.to_string())
}

/// Whether a sheet on this paper is offered in A4 pieces: only one bigger than a home printer takes.
pub fn offered_in_pieces(format: PageFormat) -> bool {
    Pieces::of(format, Orientation::Portrait).is_some()
}

/// A colouring's sheet cut into A4 pieces, as a PDF named after the picture and saying when it was made, in seconds
/// since 1970, or why it cannot be.
pub fn paint_pieces(
    colouring: &Colouring,
    picture_name: &str,
    order: Order,
    made: u64,
) -> Result<Painted, String> {
    let name = pieces_file_name(picture_name, &order);
    Ok(Painted {
        bytes: pieces_pdf(colouring, order, &name, made)
            .map_err(explain)?
            .ok_or_else(|| PRINTS_WHOLE.to_string())?,
        name,
    })
}

/// The last colouring made, kept with its picture, cells across and colours, so that a new paper, margin or
/// orientation lays it out again without colouring the picture anew.
#[derive(Default)]
pub struct LastColouring(Option<Kept>);

struct Kept {
    picture: Arc<Picture>,
    cells_across: u32,
    colours: usize,
    colouring: Arc<Colouring>,
}

impl LastColouring {
    /// The colouring an order asks of a picture: the one kept when it was made from that very picture with the same
    /// cells across and colours, or a new one, kept in its place.
    pub fn for_order(
        &mut self,
        picture: &Arc<Picture>,
        order: Order,
    ) -> Result<Arc<Colouring>, String> {
        let kept = self.0.as_ref().filter(|kept| {
            Arc::ptr_eq(&kept.picture, picture)
                && kept.cells_across == order.cells_across()
                && kept.colours == order.colours().get()
        });
        if let Some(kept) = kept {
            return Ok(kept.colouring.clone());
        }
        let colouring = Arc::new(coloured(picture, order)?);
        self.0 = Some(Kept {
            picture: picture.clone(),
            cells_across: order.cells_across(),
            colours: order.colours().get(),
            colouring: colouring.clone(),
        });
        Ok(colouring)
    }
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

/// The latest piece of work of a kind the page started, so that one a newer one overtook drops what it made instead of
/// showing it over the newer one's.
#[derive(Default)]
pub struct Latest(u64);

/// A piece of work, told apart from those started before and after it.
#[derive(Clone, Copy)]
pub struct Work(u64);

impl Latest {
    /// A new piece of work, the latest from now on.
    pub fn start(&mut self) -> Work {
        self.0 += 1;
        Work(self.0)
    }

    /// Whether no newer piece of work was started since this one.
    pub fn still(&self, work: Work) -> bool {
        self.0 == work.0
    }
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
    fn the_form_starts_on_an_a4_sheet_with_its_usual_32_cells_across_and_4_colours_and_a_ten_millimetre_margin()
     {
        let order = ready(Settings::default());

        assert_eq!(
            order.map(|order| (order.cells_across(), order.colours().get(), order.format())),
            Ok((32, 4, PageFormat::A4))
        );
        assert_eq!(Settings::default().margin, 10);
        assert!(!Settings::default().landscape);
    }

    #[test]
    fn a_picture_dropped_in_lays_the_paper_the_way_it_suits_on_its_side_once_a_tenth_wider_than_tall()
     {
        let picture = |width, height| {
            Picture::from_rgba(
                width,
                height,
                &[220, 30, 40, 255].repeat((width * height) as usize),
            )
            .expect("a picture")
        };
        let on_its_side = Settings {
            landscape: true,
            ..Settings::default()
        };

        let landscape = [
            Settings::default().for_picture(&picture(4, 3)),
            on_its_side.for_picture(&picture(3, 3)),
            on_its_side.for_picture(&picture(3, 4)),
        ]
        .map(|settings| settings.landscape);

        assert_eq!(landscape, [true, false, false]);
    }

    #[test]
    fn a_sheet_bigger_than_a4_is_offered_in_pieces_and_an_a4_or_a5_one_prints_whole_at_home() {
        let offered = PageFormat::ALL.map(|format| (format.name(), offered_in_pieces(format)));

        assert_eq!(
            offered,
            [("A2", true), ("A3", true), ("A4", false), ("A5", false)]
        );
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
    fn a_sheet_and_its_preview_are_painted_to_show_as_png_bytes_of_their_whole_page_upright_or_on_its_side_named_as_the_sheet()
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
            ready(settings).and_then(|order| {
                let colouring = coloured(&picture, order)?;
                let sheet = paint_sheet(&colouring, "red-yellow.png", order, Sheet::png_to_show)?;
                let preview = paint_preview(&colouring, &sheet.name, order, Sheet::png_to_show)?;
                Ok((page(&sheet.bytes), page(&preview), sheet.name))
            })
        });

        assert_eq!(
            painted,
            [
                Ok((
                    Some((2480, 3508)),
                    Some((2480, 3508)),
                    "red-yellow-32-4-a4.sheet.pdf".to_string()
                )),
                Ok((
                    Some((3508, 2480)),
                    Some((3508, 2480)),
                    "red-yellow-32-4-a4-landscape.sheet.pdf".to_string()
                )),
            ]
        );
    }

    #[test]
    fn a_sheet_to_print_is_a_pdf_of_its_whole_page_named_as_the_sheet_and_dated() {
        let picture = red_and_yellow();
        let order = ready(Settings::default()).expect("an order");
        let colouring = coloured(&picture, order).expect("a colouring");

        let sheet =
            sheet_to_print(&colouring, "red-yellow.png", order, 1_794_407_355).expect("a sheet");

        let text = String::from_utf8_lossy(&sheet.bytes);
        assert!(sheet.bytes.starts_with(b"%PDF-"));
        assert!(text.contains("/MediaBox [0 0 595.2 841.92]"));
        assert!(text.contains("/CreationDate (D:20261111142915Z)"));
        assert_eq!(sheet.name, "red-yellow-32-4-a4.sheet.pdf");
    }

    #[test]
    fn the_pieces_of_an_a2_sheet_are_painted_as_one_pdf_of_a4_pages_named_as_pieces() {
        let picture = red_and_yellow();
        let on_a2 = Settings {
            format: PageFormat::A2,
            width: 2,
            ..Settings::default()
        };

        let painted = ready(on_a2).and_then(|order| {
            let colouring = coloured(&picture, order)?;
            paint_pieces(&colouring, "red-yellow.png", order, 0)
        });

        let (name, pages) = painted
            .map(|pieces| {
                let pdf = String::from_utf8_lossy(&pieces.bytes).into_owned();
                let pages = pdf
                    .starts_with("%PDF-")
                    .then(|| pdf.matches("/Type /Page ").count());
                (pieces.name, pages)
            })
            .unwrap_or_default();
        assert_eq!(name, "red-yellow-2-4-a2.pieces.pdf");
        assert_eq!(pages, Some(4));
    }

    #[test]
    fn an_a4_or_a5_sheet_asked_for_in_pieces_is_said_to_print_whole_at_home() {
        let picture = red_and_yellow();
        let on = |format| Settings {
            format,
            width: 2,
            ..Settings::default()
        };

        let painted = [PageFormat::A4, PageFormat::A5].map(|format| {
            ready(on(format)).and_then(|order| {
                let colouring = coloured(&picture, order)?;
                paint_pieces(&colouring, "red-yellow.png", order, 0).map(|pieces| pieces.name)
            })
        });

        let prints_whole =
            "This sheet prints whole at home: pieces are for an A3 or A2 sheet.".to_string();
        assert_eq!(painted, [Err(prints_whole.clone()), Err(prints_whole)]);
    }

    #[test]
    fn a_sheet_or_its_preview_the_page_cannot_encode_to_show_says_so_owning_the_failure() {
        let picture = red_and_yellow();
        let failing = |_: &Sheet| Err(CannotWritePng);

        let shown = ready(Settings::default()).and_then(|order| {
            let colouring = coloured(&picture, order)?;
            Ok((
                paint_sheet(&colouring, "red-yellow.png", order, failing).err(),
                paint_preview(&colouring, "red-yellow-32-4-a4.sheet.pdf", order, failing).err(),
            ))
        });

        let on_us = "The sheet could not be shown. That one is on us.".to_string();
        assert_eq!(shown, Ok((Some(on_us.clone()), Some(on_us))));
    }

    #[test]
    fn a_colouring_is_reused_for_another_paper_margin_or_orientation_of_the_same_picture_and_made_anew_otherwise()
     {
        let picture = Arc::new(red_and_yellow());
        let first = Settings::default();
        let laid_out_again = [
            Settings {
                format: PageFormat::A3,
                ..first
            },
            Settings {
                margin: 20,
                ..first
            },
            Settings {
                landscape: true,
                ..first
            },
        ];
        let coloured_again = [
            (picture.clone(), Settings { width: 20, ..first }),
            (
                picture.clone(),
                Settings {
                    colours: 3,
                    ..first
                },
            ),
            (Arc::new(red_and_yellow()), first),
        ];
        let reused = |asked_of: &Arc<Picture>, settings: Settings| {
            let mut last = LastColouring::default();
            let made = last.for_order(&picture, ready(first)?)?;
            let asked = last.for_order(asked_of, ready(settings)?)?;
            Ok::<_, String>(Arc::ptr_eq(&made, &asked))
        };

        let laid_out = laid_out_again.map(|settings| reused(&picture, settings));
        let coloured = coloured_again.map(|(other, settings)| reused(&other, settings));

        assert_eq!(laid_out, [Ok(true), Ok(true), Ok(true)]);
        assert_eq!(coloured, [Ok(false), Ok(false), Ok(false)]);
    }

    #[test]
    fn a_piece_of_work_overtaken_by_a_newer_one_is_no_longer_the_latest_and_the_newer_one_is() {
        let mut latest = Latest::default();

        let first = latest.start();
        let first_alone = latest.still(first);
        let second = latest.start();

        assert_eq!(
            [first_alone, latest.still(first), latest.still(second)],
            [true, false, true]
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
    fn the_cells_and_colours_follow_the_paper_when_they_were_its_usual_the_cells_too_when_more_than_the_new_one_holds()
     {
        let moved = |width, colours, from, to| {
            let settings = Settings {
                width,
                colours,
                format: from,
                ..Settings::default()
            };
            let moved = settings.on_paper(to);
            (moved.format.name(), moved.width, moved.colours)
        };

        let settings = [
            moved(32, 4, PageFormat::A4, PageFormat::A5),
            moved(32, 4, PageFormat::A4, PageFormat::A2),
            moved(20, 4, PageFormat::A4, PageFormat::A2),
            moved(40, 6, PageFormat::A4, PageFormat::A5),
            moved(20, 6, PageFormat::A4, PageFormat::A2),
        ];

        assert_eq!(
            settings,
            [
                ("A5", 24, 4),
                ("A2", 64, 8),
                ("A2", 20, 8),
                ("A5", 32, 6),
                ("A2", 20, 6)
            ]
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
