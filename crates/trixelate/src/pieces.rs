//! A sheet cut into A4 pieces, for a printer that takes nothing bigger: each piece prints its part of the sheet at actual
//! size, and the pieces glued together make the sheet again.

use crate::sheet::Sheet;

/// A sheet cut into A4 pieces, in columns and rows.
pub struct Pieces<'a> {
    sheet: &'a Sheet,
}

/// Cuts a sheet into the fewest A4 pieces that cover it, lying whichever way needs fewer, the sheet's own way when
/// both need as many.
pub fn cut(sheet: &Sheet) -> Pieces<'_> {
    Pieces { sheet }
}

impl Pieces<'_> {
    pub fn across(&self) -> u32 {
        0
    }

    pub fn down(&self) -> u32 {
        0
    }

    /// The piece in a column and a row, from the top left one at 0 and 0, none past the last.
    pub fn piece(&self, _column: u32, _row: u32) -> Option<Sheet> {
        let _ = self.sheet;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Margin, Orientation, PageFormat};
    use crate::palette::ColourCount;
    use crate::picture::Picture;
    use crate::{Order, make_sheet};

    const RED: [u8; 4] = [220, 30, 40, 255];
    const YELLOW: [u8; 4] = [250, 220, 60, 255];
    const BLUE: [u8; 4] = [20, 60, 200, 255];

    fn sheet_on(format: PageFormat, orientation: Orientation) -> Sheet {
        let picture =
            Picture::from_rgba(3, 2, [RED, YELLOW, BLUE, BLUE, RED, YELLOW].as_flattened())
                .expect("a picture");
        let colours = ColourCount::new(3).expect("a count");
        let order =
            Order::new(12, colours, format, orientation, Margin::AROUND_A_SHEET).expect("an order");
        make_sheet(&picture, order, "goblin.png").expect("room")
    }

    #[test]
    fn a_sheet_is_cut_into_the_fewest_a4_pieces_that_cover_it_lying_whichever_way_needs_fewer() {
        let upright_poster = sheet_on(PageFormat::A2, Orientation::Portrait);
        let poster_on_its_side = sheet_on(PageFormat::A2, Orientation::Landscape);
        let small_sheet = sheet_on(PageFormat::A5, Orientation::Portrait);

        let cuts = [&upright_poster, &poster_on_its_side, &small_sheet].map(|sheet| {
            let pieces = cut(sheet);
            let first = pieces.piece(0, 0);
            (
                pieces.across(),
                pieces.down(),
                first.map(|piece| (piece.width(), piece.height(), piece.dots_per_inch())),
                pieces.piece(pieces.across(), 0).is_some()
                    || pieces.piece(0, pieces.down()).is_some(),
            )
        });

        assert_eq!(
            cuts,
            [
                (2, 4, Some((1754, 1240, 150)), false),
                (4, 2, Some((1240, 1754, 150)), false),
                (1, 1, Some((2480, 3508, 300)), false),
            ]
        );
    }
}
