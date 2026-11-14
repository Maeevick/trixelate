//! A sheet cut into A4 pieces, for a printer that takes nothing bigger: each piece prints its part of the sheet at actual
//! size, and the pieces glued together make the sheet again.

use crate::layout::dots;
use crate::sheet::{Paper, Sheet};

const A4_MILLIMETRES: (u32, u32) = (210, 297);
/// The blank paper around a piece's part of the sheet, wider than any home printer leaves unprinted.
const MARGIN_MM: u32 = 10;
/// The strip of the sheet a piece shares with the next, for the next one to be glued over.
const OVERLAP_MM: u32 = 10;

/// A sheet cut into A4 pieces, in columns and rows.
pub struct Pieces<'a> {
    sheet: &'a Sheet,
    cutting: Cutting,
}

/// How a sheet is cut: the piece's page, in dots at the sheet's dots per inch, and how many pieces across and down.
#[derive(Clone, Copy)]
struct Cutting {
    page: (u32, u32),
    across: u32,
    down: u32,
}

/// Cuts a sheet into the fewest A4 pieces that cover it, lying whichever way needs fewer, the sheet's own way when
/// both need as many.
pub fn cut(sheet: &Sheet) -> Pieces<'_> {
    let dots_per_inch = sheet.dots_per_inch();
    let (short, long) = (
        dots(A4_MILLIMETRES.0, dots_per_inch),
        dots(A4_MILLIMETRES.1, dots_per_inch),
    );
    let (own_way, other_way) = if sheet.width() > sheet.height() {
        ((long, short), (short, long))
    } else {
        ((short, long), (long, short))
    };
    let [own, other] = [own_way, other_way].map(|page| Cutting::of(sheet, page));
    let cutting = if other.count() < own.count() {
        other
    } else {
        own
    };
    Pieces { sheet, cutting }
}

impl Cutting {
    fn of(sheet: &Sheet, page: (u32, u32)) -> Cutting {
        let dots_per_inch = sheet.dots_per_inch();
        let (margin, overlap) = (
            dots(MARGIN_MM, dots_per_inch),
            dots(OVERLAP_MM, dots_per_inch),
        );
        let part = (page.0 - 2 * margin, page.1 - 2 * margin);
        let step = (part.0 - overlap, part.1 - overlap);
        let pieces_along =
            |length: u32, part: u32, step: u32| 1 + length.saturating_sub(part).div_ceil(step);
        Cutting {
            page,
            across: pieces_along(sheet.width(), part.0, step.0),
            down: pieces_along(sheet.height(), part.1, step.1),
        }
    }

    fn count(self) -> u32 {
        self.across * self.down
    }
}

impl Pieces<'_> {
    pub fn across(&self) -> u32 {
        self.cutting.across
    }

    pub fn down(&self) -> u32 {
        self.cutting.down
    }

    /// The piece in a column and a row, from the top left one at 0 and 0, none past the last.
    pub fn piece(&self, column: u32, row: u32) -> Option<Sheet> {
        let Cutting { page, .. } = self.cutting;
        (column < self.across() && row < self.down())
            .then(|| Paper::blank(page.0, page.1).into_sheet(self.sheet.dots_per_inch()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Margin, Orientation, PageFormat};
    use crate::palette::ColourCount;
    use crate::picture::{Picture, Rgb};
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

    #[test]
    fn each_piece_shows_its_part_of_the_sheet_10_mm_in_from_its_edges_sharing_10_mm_with_the_pieces_after_it()
     {
        let poster = sheet_on(PageFormat::A2, Orientation::Portrait);
        let (margin, apart) = ((59, 59), (1636 - 59, 1122 - 59));

        let pieces = cut(&poster);

        let differences = |column: u32, row: u32, (width, height): (u32, u32)| {
            let piece = pieces.piece(column, row).expect("a piece");
            let (left, top) = (column * apart.0, row * apart.1);
            let part = (0..1122).flat_map(|y| (0..1636).map(move |x| (x, y)));
            part.filter(|&(x, y)| {
                let expected = if x < width && y < height {
                    poster.pixel(left + x, top + y)
                } else {
                    Rgb::WHITE
                };
                piece.pixel(margin.0 + x, margin.1 + y) != expected
            })
            .count()
        };
        assert_eq!(
            [
                differences(0, 1, (1636, 1122)),
                differences(1, 3, (2480 - 1577, 3508 - 3 * 1063)),
            ],
            [0, 0]
        );
    }
}
