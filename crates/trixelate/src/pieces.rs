//! A sheet cut into A4 pieces, for a printer that takes nothing bigger: each piece prints its part of the sheet at actual
//! size, and the pieces glued together make the sheet again.

use crate::layout::{dots, dots_of_tenths};
use crate::lettering;
use crate::pdf_file::{self, CannotWritePdf};
use crate::sheet::{BLACK, NUMBER_GREY, Paper, Sheet};

const A4_MILLIMETRES: (u32, u32) = (210, 297);
/// The blank paper around a piece's part of the sheet, wider than any home printer leaves unprinted.
const MARGIN_MM: u32 = 10;
/// The strip of the sheet a piece shares with the next, for the next one to be glued over.
const OVERLAP_MM: u32 = 10;
/// A cut mark starts a millimetre from the edge it marks, runs five and is three tenths of a millimetre thick.
const CUT_MARK_GAP_MM: u32 = 1;
const CUT_MARK_LENGTH_MM: u32 = 5;
const CUT_MARK_THICKNESS_TENTHS_OF_MM: u32 = 3;
/// A piece says where it goes in letters about three millimetres tall.
const PLACE_TENTHS_OF_MM_TALL: u32 = 30;

/// A sheet cut into A4 pieces, in columns and rows.
pub struct Pieces<'a> {
    sheet: &'a Sheet,
    cutting: Cutting,
}

/// How a sheet is cut, in dots at its dots per inch: the piece's page and its margin, the part of the sheet each piece
/// shows inside its margin, how far apart parts start, a part less the strip it shares with the next, and how many
/// pieces across and down.
#[derive(Clone, Copy)]
struct Cutting {
    page: (u32, u32),
    margin: u32,
    part: (u32, u32),
    apart: (u32, u32),
    across: u32,
    down: u32,
}

/// Cuts a sheet into the fewest A4 pieces that cover it, lying whichever way needs fewer.
pub fn cut(sheet: &Sheet) -> Pieces<'_> {
    let dots_per_inch = sheet.dots_per_inch();
    let (short, long) = (
        dots(A4_MILLIMETRES.0, dots_per_inch),
        dots(A4_MILLIMETRES.1, dots_per_inch),
    );
    let [upright, on_its_side] =
        [(short, long), (long, short)].map(|page| Cutting::of(sheet, page));
    let cutting = std::cmp::min_by_key(upright, on_its_side, |cutting| cutting.count());
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
        let apart = (part.0 - overlap, part.1 - overlap);
        let pieces_along =
            |length: u32, part: u32, apart: u32| 1 + length.saturating_sub(part).div_ceil(apart);
        Cutting {
            page,
            margin,
            part,
            apart,
            across: pieces_along(sheet.width(), part.0, apart.0),
            down: pieces_along(sheet.height(), part.1, apart.1),
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
        (column < self.across() && row < self.down()).then(|| self.drawn(column, row))
    }

    /// Every piece, row after row, as a PDF of a page each that prints at actual size.
    pub fn pdf(&self) -> Result<Vec<u8>, CannotWritePdf> {
        let row_after_row =
            (0..self.down()).flat_map(|row| (0..self.across()).map(move |column| (column, row)));
        pdf_file::write(row_after_row.map(|(column, row)| self.drawn(column, row)))
    }

    /// A piece: its part of the sheet laid inside its margin, on white paper where the sheet ends.
    fn drawn(&self, column: u32, row: u32) -> Sheet {
        let Cutting {
            page,
            margin,
            part,
            apart,
            ..
        } = self.cutting;
        let (left, top) = (column * apart.0, row * apart.1);
        let size = (
            part.0.min(self.sheet.width() - left),
            part.1.min(self.sheet.height() - top),
        );
        let dots_per_inch = self.sheet.dots_per_inch();
        let mut paper = Paper::blank(page.0, page.1);
        paper.lay(self.sheet, (left, top), size, (margin, margin));
        mark_the_cuts(&mut paper, margin, size, dots_per_inch);
        let place = format!(
            "row {} of {} - column {} of {}",
            row + 1,
            self.down(),
            column + 1,
            self.across()
        );
        write_the_place(&mut paper, &place, page, margin, dots_per_inch);
        paper.into_sheet(dots_per_inch)
    }
}

/// Marks the left and top edges of a piece's part, which are cut along before the piece is glued over the pieces to its
/// left and above: each edge marked at both ends, in the blank paper beyond them.
fn mark_the_cuts(paper: &mut Paper, margin: u32, (width, height): (u32, u32), dots_per_inch: u32) {
    let gap = dots(CUT_MARK_GAP_MM, dots_per_inch);
    let length = dots(CUT_MARK_LENGTH_MM, dots_per_inch);
    let thickness = dots_of_tenths(CUT_MARK_THICKNESS_TENTHS_OF_MM, dots_per_inch).max(1);
    let (before, after) = (margin - gap - length, margin + gap);
    paper.fill(margin, before, thickness, length, BLACK);
    paper.fill(margin, after + height, thickness, length, BLACK);
    paper.fill(before, margin, length, thickness, BLACK);
    paper.fill(after + width, margin, length, thickness, BLACK);
}

/// Writes where a piece goes in grey, under its part, flush with its right margin.
fn write_the_place(
    paper: &mut Paper,
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
    paper.write(place, scale, corner, size, NUMBER_GREY);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Margin, Orientation, PageFormat};
    use crate::lettering;
    use crate::palette::ColourCount;
    use crate::picture::{Picture, Rgb};
    use crate::{Order, make_sheet};
    use std::collections::{BTreeMap, BTreeSet};

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
            let part = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
            part.filter(|&(x, y)| {
                piece.pixel(margin.0 + x, margin.1 + y) != poster.pixel(left + x, top + y)
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

    #[test]
    fn a_piece_is_marked_where_to_cut_it_on_its_left_and_top_and_says_where_it_goes_its_margin_blank_besides()
     {
        let poster = sheet_on(PageFormat::A2, Orientation::Portrait);
        let (black, grey) = (Rgb(0, 0, 0), Rgb(90, 90, 90));
        let within = |(left, top, width, height): (u32, u32, u32, u32)| {
            (top..top + height).flat_map(move |y| (left..left + width).map(move |x| (x, y)))
        };
        let cut_marks: BTreeSet<_> = [
            (59, 23, 2, 30),
            (59, 59 + 319 + 6, 2, 30),
            (23, 59, 30, 2),
            (59 + 903 + 6, 59, 30, 2),
        ]
        .into_iter()
        .flat_map(within)
        .collect();
        let place = "row 4 of 4 - column 2 of 2";
        let (place_width, _) = lettering::size(place, 2);
        let written: BTreeSet<_> = lettering::ink(place, 2)
            .into_iter()
            .map(|(x, y)| (1754 - 59 - place_width + x, 1240 - 59 + 6 + y))
            .collect();

        let last = cut(&poster).piece(1, 3).expect("the last piece");

        let the_part =
            |(x, y): &(u32, u32)| (59..59 + 903).contains(x) && (59..59 + 319).contains(y);
        let misdrawn = within((0, 0, 1754, 1240))
            .filter(|dot| !the_part(dot))
            .filter(|&(x, y)| {
                let expected = if cut_marks.contains(&(x, y)) {
                    black
                } else if written.contains(&(x, y)) {
                    grey
                } else {
                    Rgb::WHITE
                };
                last.pixel(x, y) != expected
            })
            .count();
        assert!(!written.is_empty());
        assert_eq!(misdrawn, 0);
    }

    #[test]
    fn the_pieces_are_written_as_a_pdf_of_a_page_each_row_after_row_printing_at_actual_size() {
        let poster = sheet_on(PageFormat::A2, Orientation::Portrait);
        let pieces = cut(&poster);

        let pdf = pieces.pdf().unwrap_or_default();

        let objects = objects_of(&pdf);
        let pages: Vec<_> = numbers_referred_to(
            &dictionary_of(
                &objects,
                number_after(&dictionary_of(&objects, 1), "/Pages "),
            ),
            "/Kids [",
        )
        .into_iter()
        .map(|page| {
            let page = dictionary_of(&objects, page);
            let image = number_after(&page, "/Piece ");
            let media_box = page
                .split_once("/MediaBox [")
                .and_then(|(_, rest)| rest.split_once(']'))
                .map(|(media_box, _)| media_box.to_string());
            (media_box, dots_of(&objects, image))
        })
        .collect();
        let expected: Vec<_> = (0..4)
            .flat_map(|row| (0..2).map(move |column| (column, row)))
            .map(|(column, row)| {
                let piece = pieces.piece(column, row).expect("a piece");
                (Some("0 0 841.92 595.2".to_string()), dots_in(&piece))
            })
            .collect();
        assert!(pdf.starts_with(b"%PDF-1.4\n") && pdf.ends_with(b"%%EOF\n"));
        assert_eq!(pages.len(), 8);
        assert!(
            pages == expected,
            "the PDF's pages are not the pieces, row after row"
        );
    }

    /// Every object of a PDF by its number, from where its cross-reference table says it starts, which must be there.
    fn objects_of(pdf: &[u8]) -> BTreeMap<u32, &[u8]> {
        let text = String::from_utf8_lossy(pdf);
        let table_at: usize = text
            .rsplit_once("startxref\n")
            .and_then(|(_, rest)| rest.lines().next())
            .and_then(|line| line.parse().ok())
            .unwrap_or_default();
        let table = String::from_utf8_lossy(&pdf[table_at..]);
        let mut lines = table.lines();
        assert_eq!(lines.next(), Some("xref"));
        let count: u32 = lines
            .next()
            .and_then(|line| line.split(' ').nth(1))
            .and_then(|count| count.parse().ok())
            .unwrap_or_default();
        lines
            .skip(1)
            .take(count as usize - 1)
            .zip(1..)
            .map(|(entry, number)| {
                let at: usize = entry[..10].parse().unwrap_or_default();
                let object = &pdf[at..];
                assert!(object.starts_with(format!("{number} 0 obj\n").as_bytes()));
                (number, object)
            })
            .collect()
    }

    fn dictionary_of(objects: &BTreeMap<u32, &[u8]>, number: u32) -> String {
        let object = String::from_utf8_lossy(objects.get(&number).copied().unwrap_or_default());
        let end = [object.find("stream\n"), object.find("endobj")]
            .into_iter()
            .flatten()
            .min();
        object[..end.unwrap_or_default()].to_string()
    }

    fn number_after(text: &str, key: &str) -> u32 {
        text.split_once(key)
            .and_then(|(_, rest)| rest.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|number| number.parse().ok())
            .unwrap_or_default()
    }

    fn numbers_referred_to(text: &str, key: &str) -> Vec<u32> {
        text.split_once(key)
            .and_then(|(_, rest)| rest.split_once(']'))
            .map(|(references, _)| {
                references
                    .split(" 0 R")
                    .filter_map(|number| number.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// An image object's dots, red, green and blue, row after row, its stream read as a PNG's compressed rows are.
    fn dots_of(objects: &BTreeMap<u32, &[u8]>, image: u32) -> Vec<u8> {
        let dictionary = dictionary_of(objects, image);
        let (width, height, length) = (
            number_after(&dictionary, "/Width "),
            number_after(&dictionary, "/Height "),
            number_after(&dictionary, "/Length ") as usize,
        );
        assert!(
            dictionary.contains("/ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode")
        );
        assert!(dictionary.contains(&format!(
            "/DecodeParms << /Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns {width} >>"
        )));
        let object = objects.get(&image).copied().unwrap_or_default();
        let starts = dictionary.len() + "stream\n".len();
        let compressed = &object[starts..starts + length];
        let mut header = width.to_be_bytes().to_vec();
        header.extend(height.to_be_bytes());
        header.extend([8, 2, 0, 0, 0]);
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        for (kind, data) in [
            (b"IHDR", &header[..]),
            (b"IDAT", compressed),
            (b"IEND", &[]),
        ] {
            png.extend((data.len() as u32).to_be_bytes());
            png.extend(kind);
            png.extend(data);
            png.extend([0; 4]);
        }
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png));
        decoder.ignore_checksums(true);
        let mut reader = decoder.read_info().expect("a PNG's rows");
        let mut dots = vec![0; reader.output_buffer_size().unwrap_or_default()];
        reader.next_frame(&mut dots).expect("rows that inflate");
        dots
    }

    fn dots_in(piece: &Sheet) -> Vec<u8> {
        (0..piece.height())
            .flat_map(|y| (0..piece.width()).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let Rgb(red, green, blue) = piece.pixel(x, y);
                [red, green, blue]
            })
            .collect()
    }
}
