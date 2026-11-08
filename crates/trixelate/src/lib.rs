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

/// What a sheet made from a picture file is called: the picture's name, without its extension, then the cells across,
/// the colours and the paper, then landscape or freedom when asked, so variants of one picture sit side by side
/// without overwriting each other, then what the sheet is.
pub fn sheet_file_name(picture_file_name: &str, order: &Order) -> String {
    let stem = std::path::Path::new(picture_file_name)
        .file_stem()
        .map_or(picture_file_name.into(), |stem| stem.to_string_lossy());
    let kind = match order.legend {
        Legend::Coloured(Model::None) | Legend::Freedom => "sheet",
        Legend::Coloured(Model::Full) => "model",
        Legend::Coloured(Model::Twin) => "twin",
    };
    let paper = match order.format {
        PageFormat::A3 => "a3",
        PageFormat::A4 => "a4",
        PageFormat::A5 => "a5",
    };
    let landscape = match order.orientation {
        Orientation::Portrait => "",
        Orientation::Landscape => "-landscape",
    };
    let freedom = match order.legend {
        Legend::Coloured(_) => "",
        Legend::Freedom => "-freedom",
    };
    format!(
        "{stem}-{}-{}-{paper}{landscape}{freedom}.{kind}.png",
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
            margin: Margin(20),
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
    fn a_sheet_is_named_after_its_picture_then_its_cells_across_colours_paper_and_any_landscape_or_freedom_then_what_it_is()
     {
        let ordered = |columns, colours, format, orientation, legend| Order {
            grid: GridSize::new(columns, 2).expect("a grid"),
            colours: ColourCount::new(colours).expect("a count"),
            format,
            orientation,
            margin: Margin(20),
            legend,
        };
        let (portrait, landscape) = (Orientation::Portrait, Orientation::Landscape);

        let names = [
            (
                "goblin.png",
                ordered(
                    48,
                    8,
                    PageFormat::A4,
                    portrait,
                    Legend::Coloured(Model::None),
                ),
            ),
            (
                "goblin.jpg",
                ordered(
                    64,
                    24,
                    PageFormat::A5,
                    portrait,
                    Legend::Coloured(Model::Full),
                ),
            ),
            (
                "my.photo.jpeg",
                ordered(
                    32,
                    12,
                    PageFormat::A3,
                    landscape,
                    Legend::Coloured(Model::Twin),
                ),
            ),
            (
                "goblin",
                ordered(48, 2, PageFormat::A4, landscape, Legend::Freedom),
            ),
        ]
        .map(|(picture, order)| sheet_file_name(picture, &order));

        assert_eq!(
            names,
            [
                "goblin-48-8-a4.sheet.png",
                "goblin-64-24-a5.model.png",
                "my.photo-32-12-a3-landscape.twin.png",
                "goblin-48-2-a4-landscape-freedom.sheet.png"
            ]
        );
    }
}
