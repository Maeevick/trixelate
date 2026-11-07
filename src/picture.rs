//! The picture a sheet is made from, opaque, its transparent parts read as white paper.

/// A colour as red, green and blue, each from 0 to 255.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const WHITE: Rgb = Rgb(255, 255, 255);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    width: u32,
    height: u32,
    pixels: Vec<Rgb>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotAPicture {
    NoPixel,
    BytesNotFillingItsSize,
}

impl Picture {
    /// Reads red, green, blue and alpha bytes, row after row, laid on white paper.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Picture, NotAPicture> {
        let pixels = rgba
            .chunks_exact(4)
            .map(|pixel| Rgb(pixel[0], pixel[1], pixel[2]))
            .collect();
        Ok(Picture {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel(&self, x: u32, y: u32) -> Rgb {
        self.pixels[(y * self.width + x) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transparent_pixel_reads_as_white_paper_and_a_half_transparent_one_as_its_colour_washed_out_on_it()
    -> Result<(), NotAPicture> {
        let rgba = [
            200, 0, 100, 255, //
            200, 0, 100, 0, //
            200, 0, 100, 128,
        ];

        let picture = Picture::from_rgba(3, 1, &rgba)?;

        assert_eq!(picture.pixel(0, 0), Rgb(200, 0, 100));
        assert_eq!(picture.pixel(1, 0), Rgb::WHITE);
        assert_eq!(picture.pixel(2, 0), Rgb(227, 127, 177));
        Ok(())
    }
}
