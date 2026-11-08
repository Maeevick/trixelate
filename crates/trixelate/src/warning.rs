//! What is worth a warning before a sheet is made: a sheet that will not feel like pixel art.

use crate::palette::ColourCount;

/// A reason a sheet may not turn out as colour-by-number pixel art, though a young child may still enjoy it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    FewCellsAcross,
    FewColours,
}

impl Warning {
    /// What the warning says, starting lower case, for a page or a terminal to set as it likes.
    pub fn says(self) -> &'static str {
        match self {
            Warning::FewCellsAcross => {
                "with so few cells across, it will be more playing with big cubes than colour-by-number pixel art. Still interesting for the youngest."
            }
            Warning::FewColours => {
                "with so few colours, it will be more painting solid patches than colour-by-number pixel art. Still interesting for the youngest."
            }
        }
    }
}

/// Below this many cells across, the cells are big cubes more than pixels.
const FEWEST_CELLS_ACROSS_FOR_PIXEL_ART: u32 = 16;
/// Below this many colours, the sheet is solid patches more than pixel art.
const FEWEST_COLOURS_FOR_PIXEL_ART: usize = 4;

/// The warnings a sheet's cells across and colours call for, the cells' first.
pub fn warnings_for(cells_across: u32, colours: ColourCount) -> Vec<Warning> {
    [
        (cells_across < FEWEST_CELLS_ACROSS_FOR_PIXEL_ART).then_some(Warning::FewCellsAcross),
        (colours.get() < FEWEST_COLOURS_FOR_PIXEL_ART).then_some(Warning::FewColours),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fewer_than_sixteen_cells_across_or_four_colours_are_warned_of_the_cells_first() {
        let count = |colours| ColourCount::new(colours).expect("a count");

        let warned = [(15, 3), (16, 4), (4, 8), (48, 2)]
            .map(|(cells_across, colours)| warnings_for(cells_across, count(colours)));

        assert_eq!(
            warned,
            [
                vec![Warning::FewCellsAcross, Warning::FewColours],
                vec![],
                vec![Warning::FewCellsAcross],
                vec![Warning::FewColours]
            ]
        );
    }
}
