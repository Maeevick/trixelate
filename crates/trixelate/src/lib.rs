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

use colouring::Colouring;
use grid::GridSize;
use layout::{Layout, Margin, NoRoom, Orientation, PageFormat};
use palette::ColourCount;
use picture::Picture;

/// What a sheet is asked to be: its grid, its number of colours, its paper, which way up it lies and its margin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub grid: GridSize,
    pub colours: ColourCount,
    pub format: PageFormat,
    pub orientation: Orientation,
    pub margin: Margin,
}

/// Turns a picture into the colour-by-number sheet ordered, titled with its file name, or says what lacks room on the
/// page.
pub fn make_sheet(picture: &Picture, order: Order, file_name: &str) -> Result<Picture, NoRoom> {
    let (colouring, layout) = lay_out(picture, order)?;
    Ok(sheet::draw(&colouring, &layout.titled(file_name)))
}

/// Turns a picture into the preview of the sheet ordered: the sheet as it looks once coloured in, titled with its file
/// name, or says what lacks room on the page, as making the sheet would.
pub fn make_preview(picture: &Picture, order: Order, file_name: &str) -> Result<Picture, NoRoom> {
    let (colouring, layout) = lay_out(picture, order)?;
    Ok(sheet::draw_coloured_in(
        &colouring,
        &layout.titled(file_name),
    ))
}

/// Whether the sheet ordered would fit its page, told without drawing it, so a page can say so before it is asked.
pub fn fits_on_its_page(picture: &Picture, order: Order) -> Result<(), NoRoom> {
    lay_out(picture, order).map(|_| ())
}

fn lay_out(picture: &Picture, order: Order) -> Result<(Colouring, Layout), NoRoom> {
    let colouring = Colouring::of(picture, order.grid, order.colours);
    let layout = Layout::of(&colouring, order.format, order.orientation, order.margin)?;
    Ok((colouring, layout))
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

fn named_after(picture_file_name: &str, order: &Order, what: &str) -> String {
    let stem = std::path::Path::new(picture_file_name)
        .file_stem()
        .map_or(picture_file_name.into(), |stem| stem.to_string_lossy());
    let paper = match order.format {
        PageFormat::A3 => "a3",
        PageFormat::A4 => "a4",
        PageFormat::A5 => "a5",
    };
    let landscape = match order.orientation {
        Orientation::Portrait => "",
        Orientation::Landscape => "-landscape",
    };
    format!(
        "{stem}-{}-{}-{paper}{landscape}.{what}.png",
        order.grid.columns(),
        order.colours.get()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(columns: u32, format: PageFormat) -> Order {
        Order {
            grid: GridSize::new(columns, 2).expect("a grid"),
            colours: ColourCount::new(2).expect("a count"),
            format,
            orientation: Orientation::Portrait,
            margin: Margin::new(20).expect("a margin"),
        }
    }

    #[test]
    fn a_picture_becomes_the_sheet_ordered_or_is_refused_when_the_page_lacks_room() {
        let picture =
            Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture");

        let sheets = [order(4, PageFormat::A4), order(400, PageFormat::A5)].map(|order| {
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
        let picture =
            Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture");
        let fits = order(4, PageFormat::A4);
        let sheet = make_sheet(&picture, fits, "red-yellow.png").expect("a sheet");

        let previews = [fits, order(400, PageFormat::A5)]
            .map(|order| make_preview(&picture, order, "red-yellow.png"));

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
    fn a_sheet_is_titled_with_the_file_name_it_is_given() {
        let picture =
            Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture");

        let [red, yellow] = ["red.png", "yellow.png"]
            .map(|name| make_sheet(&picture, order(4, PageFormat::A4), name));

        assert!(red.is_ok() && red != yellow);
    }

    #[test]
    fn whether_a_sheet_fits_its_page_is_told_without_drawing_it_as_making_it_would_tell() {
        let picture =
            Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture");

        let fits = [order(4, PageFormat::A4), order(400, PageFormat::A5)]
            .map(|order| fits_on_its_page(&picture, order));

        assert_eq!(fits, [Ok(()), Err(NoRoom::ForTheNumbersInTheCells)]);
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
            let order = Order {
                grid: GridSize::new(columns, 2).expect("a grid"),
                colours: ColourCount::new(colours).expect("a count"),
                format,
                orientation,
                margin: Margin::new(20).expect("a margin"),
            };
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
