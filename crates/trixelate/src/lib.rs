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

use colouring::Colouring;
use grid::GridSize;
use layout::{Layout, Legend, Margin, Model, NoRoom, Orientation, PageFormat};
use palette::ColourCount;
use picture::Picture;

/// What a sheet is asked to be: its grid, its number of colours, its paper, which way up it lies, its margin
/// and its legend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    pub grid: GridSize,
    pub colours: ColourCount,
    pub format: PageFormat,
    pub orientation: Orientation,
    pub margin: Margin,
    pub legend: Legend,
}

/// Turns a picture into the colour-by-number sheet ordered, or says what lacks room on the page.
pub fn make_sheet(picture: &Picture, order: Order) -> Result<Picture, NoRoom> {
    let (colouring, layout) = lay_out(picture, order)?;
    Ok(sheet::draw(&colouring, &layout, order.legend))
}

/// Whether the sheet ordered would fit its page, told without drawing it, so a page can say so before it is asked.
pub fn fits_on_its_page(picture: &Picture, order: Order) -> Result<(), NoRoom> {
    lay_out(picture, order).map(|_| ())
}

fn lay_out(picture: &Picture, order: Order) -> Result<(Colouring, Layout), NoRoom> {
    let colouring = Colouring::of(picture, order.grid, order.colours);
    let layout = Layout::of(
        &colouring,
        order.format,
        order.orientation,
        order.margin,
        order.legend,
    )?;
    Ok((colouring, layout))
}

/// What a sheet made from a picture file is called: the picture's name, without its extension, then what the sheet is.
pub fn sheet_file_name(picture_file_name: &str, legend: Legend) -> String {
    let stem = std::path::Path::new(picture_file_name)
        .file_stem()
        .map_or(picture_file_name.into(), |stem| stem.to_string_lossy());
    let kind = match legend {
        Legend::Coloured(Model::None) | Legend::Freedom => "sheet",
        Legend::Coloured(Model::Full) => "model",
        Legend::Coloured(Model::Twin) => "twin",
    };
    format!("{stem}.{kind}.png")
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
            margin: Margin::AROUND_A_SHEET,
            legend: Legend::Coloured(Model::None),
        }
    }

    #[test]
    fn a_picture_becomes_the_sheet_ordered_or_is_refused_when_the_page_lacks_room() {
        let picture =
            Picture::from_rgba(2, 1, &[220, 30, 40, 255, 250, 220, 60, 255]).expect("a picture");

        let sheets = [order(4, PageFormat::A4), order(400, PageFormat::A5)]
            .map(|order| make_sheet(&picture, order).map(|sheet| (sheet.width(), sheet.height())));

        assert_eq!(
            sheets,
            [Ok((2480, 3508)), Err(NoRoom::ForTheNumbersInTheCells)]
        );
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
    fn a_sheet_is_named_after_its_picture_without_its_extension_then_after_what_it_is() {
        let names = [
            ("goblin.png", Legend::Coloured(Model::None)),
            ("goblin.jpg", Legend::Coloured(Model::Full)),
            ("my.photo.jpeg", Legend::Coloured(Model::Twin)),
            ("goblin", Legend::Freedom),
        ]
        .map(|(picture, legend)| sheet_file_name(picture, legend));

        assert_eq!(
            names,
            [
                "goblin.sheet.png",
                "goblin.model.png",
                "my.photo.twin.png",
                "goblin.sheet.png"
            ]
        );
    }
}
