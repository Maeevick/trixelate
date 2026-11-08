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
        let pixel_count = width as usize * height as usize;
        if pixel_count == 0 {
            return Err(NotAPicture::NoPixel);
        }
        if rgba.len() != pixel_count * 4 {
            return Err(NotAPicture::BytesNotFillingItsSize);
        }
        let pixels = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[red, green, blue, alpha]| laid_on_white_paper(red, green, blue, alpha))
            .collect();
        Ok(Picture {
            width,
            height,
            pixels,
        })
    }

    /// A picture painted dot by dot, row after row, its pixels filling its size.
    pub(crate) fn painted(width: u32, height: u32, pixels: Vec<Rgb>) -> Picture {
        Picture {
            width,
            height,
            pixels,
        }
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

    /// Every pixel, row after row, as the picture holds them.
    pub(crate) fn pixels(&self) -> &[Rgb] {
        &self.pixels
    }
}

fn laid_on_white_paper(red: u8, green: u8, blue: u8, alpha: u8) -> Rgb {
    let over_white = |channel: u8| {
        let blended = u32::from(channel) * u32::from(alpha) + 255 * (255 - u32::from(alpha));
        ((blended + 127) / 255) as u8
    };
    Rgb(over_white(red), over_white(green), over_white(blue))
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

    #[test]
    fn bytes_that_do_not_fill_the_pictures_size_or_a_picture_without_pixel_are_no_picture() {
        let short_by_one_byte = [255, 0, 0, 255, 0, 0, 255];

        let refusals = [
            Picture::from_rgba(2, 1, &short_by_one_byte),
            Picture::from_rgba(0, 0, &[]),
        ];

        assert_eq!(
            refusals,
            [
                Err(NotAPicture::BytesNotFillingItsSize),
                Err(NotAPicture::NoPixel)
            ]
        );
    }
}
