//! A poster cut into A4 pieces for a home printer: as many as the A4 pages its paper is made of, each printing its part
//! of the poster a little smaller inside a margin the printer leaves blank, and the pieces glued together make the
//! poster again.

use std::collections::BTreeSet;

use crate::drawing::Drawing;
use crate::layout::{Orientation, PageFormat, dots, dots_of_tenths};
use crate::lettering;
use crate::pdf_file;
use crate::sheet::{BLACK, NUMBER_GREY};

const A4_MILLIMETRES: (u32, u32) = (210, 297);
/// The blank paper around a piece's part of the sheet, wider than home printers usually leave unprinted.
const MARGIN_MM: u32 = 10;
/// The strip of the sheet a piece shares with the next, for the next one to be glued over.
const OVERLAP_MM: u32 = 10;
/// A cut mark starts a millimetre from the edge it marks, runs five and is three tenths of a millimetre thick.
const CUT_MARK_GAP_MM: u32 = 1;
const CUT_MARK_LENGTH_MM: u32 = 5;
const CUT_MARK_THICKNESS_TENTHS_OF_MM: u32 = 3;
/// A piece says where it goes in letters about three millimetres tall.
const PLACE_TENTHS_OF_MM_TALL: u32 = 30;

/// How a poster is cut into A4 pieces for a home printer: how many across and down, the page each is printed on, upright
/// or on its side, and how much the poster shrinks for its pieces to be as many as the A4 pages its paper is made of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pieces {
    across: u32,
    down: u32,
    page: (u32, u32),
    scale: f64,
}

impl Pieces {
    /// The pieces a sheet on a paper lying one way is cut into, none for a paper a home printer takes whole.
    pub fn of(format: PageFormat, orientation: Orientation) -> Option<Pieces> {
        let dots_per_inch = format.dots_per_inch();
        let lying = |(short, long): (u32, u32)| match orientation {
            Orientation::Portrait => (short, long),
            Orientation::Landscape => (long, short),
        };
        let sheet_mm = lying(format.millimetres());
        let sheet = (
            dots(sheet_mm.0, dots_per_inch),
            dots(sheet_mm.1, dots_per_inch),
        );
        let (margin, overlap) = (
            dots(MARGIN_MM, dots_per_inch),
            dots(OVERLAP_MM, dots_per_inch),
        );
        let [upright, on_its_side] =
            [A4_MILLIMETRES, (A4_MILLIMETRES.1, A4_MILLIMETRES.0)].map(|page_mm| {
                let (across, down) = (
                    sheet_mm.0.div_ceil(page_mm.0),
                    sheet_mm.1.div_ceil(page_mm.1),
                );
                let page = (
                    dots(page_mm.0, dots_per_inch),
                    dots(page_mm.1, dots_per_inch),
                );
                let covered = |pieces: u32, page: u32| {
                    f64::from(pieces * (page - 2 * margin) - (pieces - 1) * overlap)
                };
                let scale = (covered(across, page.0) / f64::from(sheet.0))
                    .min(covered(down, page.1) / f64::from(sheet.1))
                    .min(1.0);
                Pieces {
                    across,
                    down,
                    page,
                    scale,
                }
            });
        let fewer =
            std::cmp::min_by_key(upright, on_its_side, |pieces| pieces.across * pieces.down);
        (fewer.across * fewer.down > 1).then_some(fewer)
    }

    pub fn across(&self) -> u32 {
        self.across
    }

    pub fn down(&self) -> u32 {
        self.down
    }

    /// How big the glued pieces are against the paper, in whole percent rounded down.
    pub fn percent(&self) -> u32 {
        (self.scale * 100.0) as u32
    }

    /// Whether each piece is printed on an A4 page on its side.
    pub fn on_their_side(&self) -> bool {
        self.page.0 > self.page.1
    }

    /// Each piece of a sheet's drawing as a page, row after row: the sheet shrunk and moved for its part to fill the
    /// window inside the margin, the window starting where the last piece's shared strip does, and in the margin the
    /// marks to cut along and where the piece goes. Also every character the pages write.
    pub(crate) fn pages(&self, sheet: &Drawing) -> (Vec<pdf_file::Page>, BTreeSet<char>) {
        let dots_per_inch = sheet.dots_per_inch();
        let points = |dots: f64| dots * 72.0 / f64::from(dots_per_inch);
        let (margin, overlap) = (
            dots(MARGIN_MM, dots_per_inch),
            dots(OVERLAP_MM, dots_per_inch),
        );
        let part = (self.page.0 - 2 * margin, self.page.1 - 2 * margin);
        let apart = (part.0 - overlap, part.1 - overlap);
        let drawn = String::from_utf8_lossy(&pdf_file::page_of(sheet).content).into_owned();
        let mut characters = pdf_file::characters_of(sheet);
        let mut pages = Vec::new();
        for (column, row) in
            (0..self.down).flat_map(|row| (0..self.across).map(move |column| (column, row)))
        {
            let mut around = Drawing::blank(self.page.0, self.page.1, dots_per_inch);
            mark_where_to_cut(&mut around, margin, part, dots_per_inch);
            let place = format!(
                "row {} of {} - column {} of {}",
                row + 1,
                self.down,
                column + 1,
                self.across
            );
            say_where_it_goes(&mut around, &place, self.page, margin, dots_per_inch);
            characters.extend(pdf_file::characters_of(&around));
            let page = pdf_file::page_of(&around);
            let moved = (
                points(f64::from(margin) - f64::from(column * apart.0)),
                points(
                    f64::from(self.page.1 - margin + row * apart.1)
                        - self.scale * f64::from(sheet.height()),
                ),
            );
            let mut content = format!(
                "q {} {} {} {} re W n\n{} 0 0 {} {} {} cm\n{drawn}Q\n",
                pdf_file::number(points(f64::from(margin))),
                pdf_file::number(points(f64::from(margin))),
                pdf_file::number(points(f64::from(part.0))),
                pdf_file::number(points(f64::from(part.1))),
                pdf_file::number_to(self.scale, 6),
                pdf_file::number_to(self.scale, 6),
                pdf_file::number(moved.0),
                pdf_file::number(moved.1),
            )
            .into_bytes();
            content.extend(page.content);
            pages.push(pdf_file::Page { content, ..page });
        }
        (pages, characters)
    }
}

/// Marks a piece where to cut it: along its window's left and top edges, a little past each end, to trim the margin
/// that the pieces before it are glued over.
fn mark_where_to_cut(
    around: &mut Drawing,
    margin: u32,
    (width, height): (u32, u32),
    dots_per_inch: u32,
) {
    let gap = dots(CUT_MARK_GAP_MM, dots_per_inch);
    let length = dots(CUT_MARK_LENGTH_MM, dots_per_inch);
    let thickness = dots_of_tenths(CUT_MARK_THICKNESS_TENTHS_OF_MM, dots_per_inch).max(1);
    let (before, after) = (margin - gap - length, margin + gap);
    around.fill(margin, before, thickness, length, BLACK);
    around.fill(margin, after + height, thickness, length, BLACK);
    around.fill(before, margin, length, thickness, BLACK);
    around.fill(after + width, margin, length, thickness, BLACK);
}

/// Writes where a piece goes in grey, under its window, flush with its right margin.
fn say_where_it_goes(
    around: &mut Drawing,
    place: &str,
    page: (u32, u32),
    margin: u32,
    dots_per_inch: u32,
) {
    let (_, height_at_one) = lettering::size(place, 1);
    let scale = dots_of_tenths(PLACE_TENTHS_OF_MM_TALL, dots_per_inch) / height_at_one;
    let size = lettering::size(place, scale);
    let corner = (
        page.0 - margin - size.0,
        page.1 - margin + dots(CUT_MARK_GAP_MM, dots_per_inch),
    );
    around.letters(place, scale, corner, NUMBER_GREY);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_on_a2_or_a3_takes_as_many_a4_pieces_as_its_paper_holds_a4_pages_shrunk_to_90_percent_and_on_a4_or_a5_none()
     {
        let (upright, on_its_side) = (Orientation::Portrait, Orientation::Landscape);

        let cuts = [
            (PageFormat::A2, upright),
            (PageFormat::A2, on_its_side),
            (PageFormat::A3, upright),
            (PageFormat::A3, on_its_side),
            (PageFormat::A4, upright),
            (PageFormat::A5, on_its_side),
        ]
        .map(|(format, orientation)| {
            Pieces::of(format, orientation).map(|pieces| {
                (
                    pieces.across(),
                    pieces.down(),
                    pieces.on_their_side(),
                    pieces.percent(),
                )
            })
        });

        assert_eq!(
            cuts,
            [
                Some((2, 2, false, 90)),
                Some((2, 2, true, 90)),
                Some((1, 2, true, 90)),
                Some((2, 1, false, 90)),
                None,
                None
            ]
        );
    }
}
