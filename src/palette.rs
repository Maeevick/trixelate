//! The dominant colours of a grid, numbered from 1, the most used first.

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
        let _ = (colours, count);
        Palette(vec![])
    }

    pub fn colours(&self) -> &[Rgb] {
        &self.0
    }
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
}
