//! The dominant colours of a grid, numbered from 1, the most used first.

use std::collections::BTreeMap;
use std::fmt;

use crate::grid::Grid;
use crate::picture::Rgb;

/// How many colours a palette may hold at most, from 2 to 12: one colour is no colouring, and twelve is a box of crayons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColourCount(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColourCountOutOfRange;

impl ColourCount {
    pub const FEWEST: usize = 2;
    pub const MOST: usize = 12;

    pub fn new(count: usize) -> Result<ColourCount, ColourCountOutOfRange> {
        (Self::FEWEST..=Self::MOST)
            .contains(&count)
            .then_some(ColourCount(count))
            .ok_or(ColourCountOutOfRange)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Palette(Vec<Rgb>);

/// A colour's place in its palette, from 1, as written in the cells to paint with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColourNumber(usize);

impl fmt::Display for ColourNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Palette {
    /// The colours that best stand for a grid's cells, at most `count` of them.
    pub fn dominant(grid: &Grid, count: ColourCount) -> Palette {
        let mut boxes: Vec<Tallied> = vec![tally(grid.cells()).into_iter().collect()];
        while boxes.len() < count.0 {
            let Some(widest) = index_of_the_widest(&boxes) else {
                break;
            };
            let (low, high) = cut_through_its_middle(boxes.swap_remove(widest));
            boxes.extend([low, high]);
        }
        most_used_first(boxes.iter().map(average).collect())
    }

    /// The number of the palette colour nearest to the given one.
    pub fn number_for(&self, colour: Rgb) -> ColourNumber {
        let distance_to = |other: Rgb| {
            channels(colour)
                .into_iter()
                .zip(channels(other))
                .map(|(one, two)| u32::from(one.abs_diff(two)).pow(2))
                .sum::<u32>()
        };
        let nearest = (0..self.0.len())
            .min_by_key(|&index| distance_to(self.0[index]))
            .unwrap_or(0);
        ColourNumber(nearest + 1)
    }

    /// The colours some of the given cells are nearest to, ranked by those cells, the most used first:
    /// a box's average can lie nearer to none of its own colours than other boxes' averages do.
    pub fn used_by(&self, cells: &[Rgb]) -> Palette {
        let uses = cells.iter().fold(vec![0; self.0.len()], |mut uses, &cell| {
            uses[self.number_for(cell).0 - 1] += 1;
            uses
        });
        most_used_first(
            self.0
                .iter()
                .copied()
                .zip(uses)
                .filter(|&(_, uses)| uses > 0)
                .collect(),
        )
    }

    /// The colour a number stands for in this palette.
    pub fn colour_of(&self, number: ColourNumber) -> Rgb {
        self.0[number.0 - 1]
    }

    /// Each colour with its number, colour 1 first.
    pub fn numbered(&self) -> impl Iterator<Item = (ColourNumber, Rgb)> + '_ {
        self.0
            .iter()
            .enumerate()
            .map(|(index, &colour)| (ColourNumber(index + 1), colour))
    }

    pub fn colours(&self) -> &[Rgb] {
        &self.0
    }
}

type Tallied = Vec<(Rgb, usize)>;

/// A palette of the given colours, the most used first, ties going to the smaller colour.
fn most_used_first(mut colours: Tallied) -> Palette {
    colours.sort_by(|(colour, uses), (other, other_uses)| {
        other_uses.cmp(uses).then(colour.cmp(other))
    });
    Palette(colours.into_iter().map(|(colour, _)| colour).collect())
}

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
        .max_by_key(|&(_, spread)| spread)
        .unwrap_or((0, 0))
}

fn index_of_the_widest(boxes: &[Tallied]) -> Option<usize> {
    boxes
        .iter()
        .enumerate()
        .map(|(index, tallied)| (index, widest_channel(tallied).1))
        .filter(|&(_, spread)| spread > 0)
        .max_by_key(|&(_, spread)| spread)
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
    use crate::grid::GridSize;
    use crate::picture::Picture;

    const RED: Rgb = Rgb(220, 30, 40);
    const BLUE: Rgb = Rgb(20, 60, 200);

    fn grid_of(cells: &[Rgb]) -> Grid {
        let rgba: Vec<u8> = cells
            .iter()
            .flat_map(|&Rgb(red, green, blue)| [red, green, blue, 255])
            .collect();
        let columns = cells.len() as u32;
        let picture = Picture::from_rgba(columns, 1, &rgba).expect("a picture");
        Grid::averaging(&picture, GridSize::new(columns, 1).expect("a grid"))
    }

    #[test]
    fn fewer_colours_than_asked_are_each_kept_once_the_most_used_first()
    -> Result<(), ColourCountOutOfRange> {
        let cells = grid_of(&[BLUE, RED, RED, BLUE, RED]);

        let palette = Palette::dominant(&cells, ColourCount::new(4)?);

        assert_eq!(palette.colours(), [RED, BLUE]);
        Ok(())
    }

    #[test]
    fn more_colours_than_asked_merge_the_closest_into_their_weighted_average()
    -> Result<(), ColourCountOutOfRange> {
        let cells = grid_of(&[
            Rgb(250, 0, 0),
            Rgb(0, 250, 0),
            Rgb(250, 0, 0),
            Rgb(240, 0, 0),
            Rgb(0, 240, 0),
            Rgb(0, 0, 250),
        ]);

        let palette = Palette::dominant(&cells, ColourCount::new(3)?);

        assert_eq!(
            palette.colours(),
            [Rgb(247, 0, 0), Rgb(0, 245, 0), Rgb(0, 0, 250)]
        );
        Ok(())
    }

    #[test]
    fn a_palette_holds_from_two_to_twenty_four_colours() {
        let counts = [1, 2, 24, 25].map(|count| ColourCount::new(count).is_ok());

        assert_eq!(counts, [false, true, true, false]);
    }

    #[test]
    fn a_colour_takes_the_number_of_the_nearest_palette_colour_counted_from_one()
    -> Result<(), ColourCountOutOfRange> {
        let palette = Palette::dominant(&grid_of(&[RED, RED, BLUE]), ColourCount::new(2)?);

        let numbers = [Rgb(200, 20, 30), Rgb(10, 10, 180)]
            .map(|colour| palette.number_for(colour).to_string());

        assert_eq!(numbers, ["1", "2"]);
        Ok(())
    }

    #[test]
    fn the_box_of_colours_that_spreads_the_most_is_cut_first() -> Result<(), ColourCountOutOfRange>
    {
        let cells = grid_of(&[Rgb(0, 0, 0), Rgb(20, 0, 0), Rgb(200, 0, 0), Rgb(255, 0, 0)]);

        let palette = Palette::dominant(&cells, ColourCount::new(3)?);

        assert_eq!(
            palette.colours(),
            [Rgb(10, 0, 0), Rgb(200, 0, 0), Rgb(255, 0, 0)]
        );
        Ok(())
    }
}
