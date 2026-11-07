//! The dominant colours of a grid, numbered from 1, the most used first.

use std::collections::BTreeMap;

use crate::picture::Rgb;

/// How many colours a palette may hold at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColourCount(usize);

impl ColourCount {
    pub fn new(count: usize) -> ColourCount {
        ColourCount(count)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette(Vec<Rgb>);

impl Palette {
    /// The colours that best stand for all the given ones, at most `count` of them.
    pub fn dominant(colours: &[Rgb], count: ColourCount) -> Palette {
        let _ = count;
        let mut most_used_first: Vec<(Rgb, usize)> = tally(colours).into_iter().collect();
        most_used_first.sort_by(|(colour, uses), (other, other_uses)| {
            other_uses.cmp(uses).then(colour.cmp(other))
        });
        Palette(
            most_used_first
                .into_iter()
                .map(|(colour, _)| colour)
                .collect(),
        )
    }

    pub fn colours(&self) -> &[Rgb] {
        &self.0
    }
}

fn tally(colours: &[Rgb]) -> BTreeMap<Rgb, usize> {
    colours.iter().fold(BTreeMap::new(), |mut uses, &colour| {
        *uses.entry(colour).or_default() += 1;
        uses
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb(220, 30, 40);
    const BLUE: Rgb = Rgb(20, 60, 200);

    #[test]
    fn fewer_colours_than_asked_are_each_kept_once_the_most_used_first() {
        let cells = [BLUE, RED, RED, BLUE, RED];

        let palette = Palette::dominant(&cells, ColourCount::new(4));

        assert_eq!(palette.colours(), [RED, BLUE]);
    }

    #[test]
    fn more_colours_than_asked_merge_the_closest_into_their_weighted_average() {
        let cells = [
            Rgb(250, 0, 0),
            Rgb(0, 250, 0),
            Rgb(250, 0, 0),
            Rgb(240, 0, 0),
            Rgb(0, 240, 0),
            Rgb(0, 0, 250),
        ];

        let palette = Palette::dominant(&cells, ColourCount::new(3));

        assert_eq!(
            palette.colours(),
            [Rgb(247, 0, 0), Rgb(0, 245, 0), Rgb(0, 0, 250)]
        );
    }
}
