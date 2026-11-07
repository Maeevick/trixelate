//! Where everything lies on the sheet's paper, in dots at 300 dots per inch.

use crate::colouring::Colouring;

/// The paper a sheet is drawn for, upright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFormat {
    A3,
    A4,
    A5,
}

impl PageFormat {
    fn millimetres(self) -> (u32, u32) {
        match self {
            PageFormat::A3 => (297, 420),
            PageFormat::A4 => (210, 297),
            PageFormat::A5 => (148, 210),
        }
    }
}

const DOTS_PER_INCH: u32 = 300;
const TENTHS_OF_MILLIMETRE_PER_INCH: u32 = 254;

fn dots(millimetres: u32) -> u32 {
    (millimetres * DOTS_PER_INCH * 10 + TENTHS_OF_MILLIMETRE_PER_INCH / 2)
        / TENTHS_OF_MILLIMETRE_PER_INCH
}

/// The blank paper around the sheet's content, in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Margin(pub u32);

impl Margin {
    pub const AROUND_A_SHEET: Margin = Margin(20);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRoom {
    InsideTheMargin,
    ForTheNumbersInTheCells,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    page_width: u32,
    page_height: u32,
}

impl Layout {
    /// Lays out a colouring's grid and legend on the page, inside its margin.
    pub fn of(colouring: &Colouring, format: PageFormat, margin: Margin) -> Result<Layout, NoRoom> {
        let _ = (colouring, margin);
        let (width, height) = format.millimetres();
        Ok(Layout {
            page_width: dots(width),
            page_height: dots(height),
        })
    }

    pub fn page_width(&self) -> u32 {
        self.page_width
    }

    pub fn page_height(&self) -> u32 {
        self.page_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridSize;
    use crate::palette::ColourCount;
    use crate::picture::Picture;

    fn colouring(columns: u32, rows: u32, colours: usize) -> Colouring {
        let rainbow: Vec<u8> = (0..columns * rows)
            .flat_map(|cell| {
                let shade = (cell * 37 % 256) as u8;
                [shade, 255 - shade, shade / 2, 255]
            })
            .collect();
        let picture = Picture::from_rgba(columns, rows, &rainbow).expect("a picture");
        Colouring::of(
            &picture,
            GridSize::new(columns, rows).expect("a grid"),
            ColourCount::new(colours).expect("a count"),
        )
    }

    #[test]
    fn a_page_counts_its_millimetres_in_dots_at_300_dots_per_inch() {
        let small = colouring(4, 4, 2);

        let pages = [PageFormat::A3, PageFormat::A4, PageFormat::A5]
            .map(|format| Layout::of(&small, format, Margin::AROUND_A_SHEET))
            .map(|layout| layout.map(|layout| (layout.page_width(), layout.page_height())));

        assert_eq!(
            pages,
            [Ok((3508, 4961)), Ok((2480, 3508)), Ok((1748, 2480))]
        );
    }
}
