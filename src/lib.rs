//! Turns a picture into a colour-by-number pixel sheet, ready to print.

pub mod colouring;
pub mod grid;
pub mod layout;
pub mod lettering;
pub mod palette;
pub mod picture;
pub mod png_file;
pub mod sheet;

use colouring::Colouring;
use grid::GridSize;
use layout::{Grids, Layout, Margin, NoRoom, Orientation, PageFormat};
use palette::ColourCount;
use picture::Picture;
use sheet::{Legend, Model};

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
    let colouring = Colouring::of(picture, order.grid, order.colours);
    let grids = match order.legend {
        Legend::Coloured(Model::Twin) => Grids::Twinned,
        Legend::Coloured(Model::None | Model::Full) | Legend::Freedom => Grids::Alone,
    };
    let layout = Layout::of(
        &colouring,
        order.format,
        order.orientation,
        order.margin,
        grids,
    )?;
    Ok(sheet::draw(&colouring, &layout, order.legend))
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
}
