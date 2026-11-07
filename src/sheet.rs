//! The page printed for the child: the grid of numbered cells and its legend, drawn where the layout says.

use crate::colouring::Colouring;
use crate::layout::Layout;
use crate::picture::{Picture, Rgb};

/// Draws a colouring's sheet on white paper, as laid out.
pub fn draw(colouring: &Colouring, layout: &Layout) -> Picture {
    let _ = (colouring, layout);
    Picture::painted(1, 1, vec![Rgb::WHITE])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::GridSize;
    use crate::layout::{Margin, PageFormat};
    use crate::palette::ColourCount;

    const RED: [u8; 4] = [220, 30, 40, 255];
    const YELLOW: [u8; 4] = [250, 220, 60, 255];
    const LINE_GREY: Rgb = Rgb(170, 170, 170);

    fn red_red_yellow() -> (Colouring, Layout) {
        let picture =
            Picture::from_rgba(3, 1, [RED, RED, YELLOW].as_flattened()).expect("a picture");
        let colouring = Colouring::of(
            &picture,
            GridSize::new(3, 1).expect("a grid"),
            ColourCount::new(2).expect("a count"),
        );
        let layout = Layout::of(&colouring, PageFormat::A5, Margin::AROUND_A_SHEET).expect("room");
        (colouring, layout)
    }

    #[test]
    fn a_sheet_fills_its_page_with_its_margin_blank_and_every_cell_outlined() {
        let (colouring, layout) = red_red_yellow();
        let margin = 236;

        let sheet = draw(&colouring, &layout);

        let (width, height) = (layout.page_width(), layout.page_height());
        assert_eq!((sheet.width(), sheet.height()), (width, height));
        let in_the_margin = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                x < margin || y < margin || x >= width - margin || y >= height - margin
            });
        assert!(
            in_the_margin
                .into_iter()
                .all(|(x, y)| sheet.pixel(x, y) == Rgb::WHITE)
        );
        let corners = (0..3).map(|column| layout.cell(column, 0));
        assert!(
            corners
                .into_iter()
                .all(|cell| sheet.pixel(cell.left, cell.top) == LINE_GREY
                    && sheet.pixel(cell.left + cell.side, cell.top + cell.side) == LINE_GREY)
        );
    }
}
