//! A page as the marks it is drawn with, in dots at its dots per inch, whatever it is then drawn on: dots for a screen,
//! or shapes for a PDF.

use crate::layout::Disc;
use crate::lettering;
use crate::picture::Rgb;
use crate::sheet::{Paper, Sheet};

/// One mark on a page, in dots from its top left corner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// A rectangle filled with a colour.
    Fill {
        left: u32,
        top: u32,
        width: u32,
        height: u32,
        colour: Rgb,
    },
    /// A disc filled with a colour and outlined in black.
    Disc {
        disc: Disc,
        outline: u32,
        colour: Rgb,
    },
    /// A line of lettering, from the top left corner of its first character, each of its dots a square of its scale.
    Letters {
        text: String,
        scale: u32,
        left: u32,
        top: u32,
        colour: Rgb,
    },
}

/// A page's size in dots, how finely it is printed, and its marks in the order they are drawn, each over the last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Drawing {
    width: u32,
    height: u32,
    dots_per_inch: u32,
    marks: Vec<Mark>,
}

impl Drawing {
    pub(crate) fn blank(width: u32, height: u32, dots_per_inch: u32) -> Drawing {
        Drawing {
            width,
            height,
            dots_per_inch,
            marks: Vec::new(),
        }
    }

    pub(crate) fn fill(&mut self, left: u32, top: u32, width: u32, height: u32, colour: Rgb) {
        self.marks.push(Mark::Fill {
            left,
            top,
            width,
            height,
            colour,
        });
    }

    pub(crate) fn disc(&mut self, disc: Disc, outline: u32, colour: Rgb) {
        self.marks.push(Mark::Disc {
            disc,
            outline,
            colour,
        });
    }

    /// Writes a line of lettering from the top left corner of its first character.
    pub(crate) fn letters(&mut self, text: &str, scale: u32, (left, top): (u32, u32), colour: Rgb) {
        self.marks.push(Mark::Letters {
            text: text.to_string(),
            scale,
            left,
            top,
            colour,
        });
    }

    /// Writes a line of lettering centred in a box given by its top left corner and its size.
    pub(crate) fn write(
        &mut self,
        text: &str,
        scale: u32,
        (left, top): (u32, u32),
        (width, height): (u32, u32),
        colour: Rgb,
    ) {
        let (text_width, text_height) = lettering::size(text, scale);
        let corner = (
            left + width.saturating_sub(text_width) / 2,
            top + height.saturating_sub(text_height) / 2,
        );
        self.letters(text, scale, corner, colour);
    }

    /// The page drawn dot by dot, as a sheet printed at its dots per inch.
    pub(crate) fn painted(&self) -> Sheet {
        let mut paper = Paper::blank(self.width, self.height);
        for mark in &self.marks {
            match mark {
                Mark::Fill {
                    left,
                    top,
                    width,
                    height,
                    colour,
                } => paper.fill(*left, *top, *width, *height, *colour),
                Mark::Disc {
                    disc,
                    outline,
                    colour,
                } => paper.disc(disc, *outline, *colour),
                Mark::Letters {
                    text,
                    scale,
                    left,
                    top,
                    colour,
                } => paper.letters(text, *scale, (*left, *top), *colour),
            }
        }
        paper.into_sheet(self.dots_per_inch)
    }
}
