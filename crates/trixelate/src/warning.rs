//! What is worth a warning before a sheet is made: a sheet that will not feel like pixel art.

use crate::palette::ColourCount;

/// A reason a sheet may not turn out as colour-by-number pixel art, though a young child may still enjoy it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    FewCellsAcross,
    FewColours,
}

/// The warnings a sheet's cells across and colours call for, the cells' first.
pub fn warnings_for(cells_across: u32, colours: ColourCount) -> Vec<Warning> {
    let _ = (cells_across, colours);
    Vec::new()
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
