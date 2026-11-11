//! A picture read from a file's bytes, whether a PNG or a JPEG.

use crate::picture::Picture;
use crate::{jpeg_file, png_file};

/// The most megapixels a picture may have: more than a 48 megapixel phone photo, far less than what would exhaust the
/// memory of a browser tab.
pub const MOST_MEGAPIXELS: u64 = 50;

/// The most megabytes a picture file may weigh, checked before it is read: ten times a 48 megapixel phone photo.
pub const MOST_MEGABYTES: u64 = 100;

/// Why a file's bytes give no picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    NeitherPngNorJpeg,
    /// More megapixels than [`MOST_MEGAPIXELS`], counted up to the next whole one, refused before its pixels are read.
    TooBig {
        megapixels: u64,
    },
}

/// The picture a PNG or a JPEG holds.
pub fn read(bytes: &[u8]) -> Result<Picture, Unreadable> {
    let (width, height) = png_file::size(bytes)
        .or_else(|| jpeg_file::size(bytes))
        .ok_or(Unreadable::NeitherPngNorJpeg)?;
    let megapixels = (u64::from(width) * u64::from(height)).div_ceil(1_000_000);
    if megapixels > MOST_MEGAPIXELS {
        return Err(Unreadable::TooBig { megapixels });
    }
    png_file::read(bytes)
        .ok()
        .or_else(|| jpeg_file::read(bytes).ok())
        .ok_or(Unreadable::NeitherPngNorJpeg)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED_AND_BLUE: &[u8] = include_bytes!("../tests/fixtures/red-and-blue.jpg");

    #[test]
    fn a_png_and_a_jpeg_are_read_and_anything_else_is_refused() {
        let png = png_file::write(
            &Picture::painted(3, 2, vec![crate::picture::Rgb::WHITE; 6]),
            300,
        )
        .expect("a PNG");
        let words = b"a goblin described in words".as_slice();

        let sizes = [png.as_slice(), RED_AND_BLUE, words]
            .map(|bytes| read(bytes).map(|picture| (picture.width(), picture.height())));

        assert_eq!(
            sizes,
            [Ok((3, 2)), Ok((16, 8)), Err(Unreadable::NeitherPngNorJpeg)]
        );
    }

    /// A PNG's header claiming a size, with no pixel behind it.
    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgb);
        drop(encoder.write_header().expect("a header"));
        bytes
    }

    /// The red and blue JPEG, its header claiming a size its data does not have.
    fn jpeg_claiming(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = RED_AND_BLUE.to_vec();
        let frame = bytes
            .windows(2)
            .position(|marker| marker == [0xFF, 0xC0])
            .expect("a baseline frame");
        bytes[frame + 5..frame + 7].copy_from_slice(&height.to_be_bytes());
        bytes[frame + 7..frame + 9].copy_from_slice(&width.to_be_bytes());
        bytes
    }

    #[test]
    fn a_picture_over_fifty_megapixels_is_refused_before_its_pixels_are_read_saying_how_many_it_has()
     {
        let files = [
            png_header(8_000, 7_000),
            png_header(10_000, 5_000),
            jpeg_claiming(10_000, 6_001),
        ];

        let read = files.map(|bytes| read(&bytes).map(|picture| picture.width()));

        assert_eq!(
            read,
            [
                Err(Unreadable::TooBig { megapixels: 56 }),
                Err(Unreadable::NeitherPngNorJpeg),
                Err(Unreadable::TooBig { megapixels: 61 }),
            ]
        );
    }
}
