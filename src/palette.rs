//! The dominant colours of a grid, numbered from 1, the most used first.

use std::collections::BTreeMap;

use crate::picture::Rgb;

/// How many colours a palette may hold at most, from 2 to 30: one colour is no colouring, and thirty is more than a box of crayons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColourCount(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColourCountOutOfRange;

impl ColourCount {
    pub const FEWEST: usize = 2;
    pub const MOST: usize = 30;

    pub fn new(count: usize) -> Result<ColourCount, ColourCountOutOfRange> {
        (Self::FEWEST..=Self::MOST)
            .contains(&count)
            .then_some(ColourCount(count))
            .ok_or(ColourCountOutOfRange)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette(Vec<Rgb>);

impl Palette {
    /// The colours that best stand for all the given ones, at most `count` of them.
    pub fn dominant(colours: &[Rgb], count: ColourCount) -> Palette {
        let every_colour: Tallied = tally(colours).into_iter().collect();
        let mut boxes = if every_colour.is_empty() {
            vec![]
        } else {
            vec![every_colour]
        };
        while boxes.len() < count.0 {
            let Some(widest) = index_of_the_widest(&boxes) else {
                break;
            };
            let (low, high) = cut_through_its_middle(boxes.swap_remove(widest));
            boxes.extend([low, high]);
        }
        let mut most_used_first: Vec<(Rgb, usize)> = boxes.iter().map(average).collect();
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

type Tallied = Vec<(Rgb, usize)>;

fn channels(Rgb(red, green, blue): Rgb) -> [u8; 3] {
    [red, green, blue]
}

/// The channel whose values spread the most in a box, and how far they spread.
fn widest_channel(tallied: &Tallied) -> (usize, u8) {
    (0..3)
        .map(|channel| {
            let values = tallied.iter().map(|&(colour, _)| channels(colour)[channel]);
            let spread = values.clone().max().unwrap_or(0) - values.min().unwrap_or(0);
            (channel, spread)
        })
        .fold((0, 0), |widest, candidate| {
            if candidate.1 > widest.1 {
                candidate
            } else {
                widest
            }
        })
}

fn index_of_the_widest(boxes: &[Tallied]) -> Option<usize> {
    boxes
        .iter()
        .enumerate()
        .map(|(index, tallied)| (index, widest_channel(tallied).1))
        .filter(|&(_, spread)| spread > 0)
        .fold(
            None,
            |widest: Option<(usize, u8)>, candidate| match widest {
                Some(found) if found.1 >= candidate.1 => Some(found),
                _ => Some(candidate),
            },
        )
        .map(|(index, _)| index)
}

fn cut_through_its_middle(tallied: Tallied) -> (Tallied, Tallied) {
    let (channel, _) = widest_channel(&tallied);
    let values = tallied
        .iter()
        .map(|&(colour, _)| u16::from(channels(colour)[channel]));
    let middle = (values.clone().min().unwrap_or(0) + values.max().unwrap_or(0)) / 2;
    tallied
        .into_iter()
        .partition(|&(colour, _)| u16::from(channels(colour)[channel]) <= middle)
}

fn average(tallied: &Tallied) -> (Rgb, usize) {
    let uses: usize = tallied.iter().map(|&(_, uses)| uses).sum();
    let mean = |channel: usize| {
        let sum: usize = tallied
            .iter()
            .map(|&(colour, uses)| usize::from(channels(colour)[channel]) * uses)
            .sum();
        ((sum + uses / 2) / uses) as u8
    };
    (Rgb(mean(0), mean(1), mean(2)), uses)
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
    fn fewer_colours_than_asked_are_each_kept_once_the_most_used_first()
    -> Result<(), ColourCountOutOfRange> {
        let cells = [BLUE, RED, RED, BLUE, RED];

        let palette = Palette::dominant(&cells, ColourCount::new(4)?);

        assert_eq!(palette.colours(), [RED, BLUE]);
        Ok(())
    }

    #[test]
    fn more_colours_than_asked_merge_the_closest_into_their_weighted_average()
    -> Result<(), ColourCountOutOfRange> {
        let cells = [
            Rgb(250, 0, 0),
            Rgb(0, 250, 0),
            Rgb(250, 0, 0),
            Rgb(240, 0, 0),
            Rgb(0, 240, 0),
            Rgb(0, 0, 250),
        ];

        let palette = Palette::dominant(&cells, ColourCount::new(3)?);

        assert_eq!(
            palette.colours(),
            [Rgb(247, 0, 0), Rgb(0, 245, 0), Rgb(0, 0, 250)]
        );
        Ok(())
    }

    #[test]
    fn no_colour_gives_an_empty_palette() -> Result<(), ColourCountOutOfRange> {
        let palette = Palette::dominant(&[], ColourCount::new(8)?);

        assert_eq!(palette.colours(), []);
        Ok(())
    }

    #[test]
    fn a_palette_holds_from_two_to_thirty_colours() {
        let counts = [1, 2, 30, 31].map(|count| ColourCount::new(count).is_ok());

        assert_eq!(counts, [false, true, true, false]);
    }
}
