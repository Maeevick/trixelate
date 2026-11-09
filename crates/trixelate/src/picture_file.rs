//! A picture read from a file's bytes, whether a PNG or a JPEG.

use crate::picture::Picture;
use crate::{jpeg_file, png_file};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NeitherPngNorJpeg;

/// The picture a PNG or a JPEG holds.
pub fn read(bytes: &[u8]) -> Result<Picture, NeitherPngNorJpeg> {
    png_file::read(bytes)
        .ok()
        .or_else(|| jpeg_file::read(bytes).ok())
        .ok_or(NeitherPngNorJpeg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_and_a_jpeg_are_read_and_anything_else_is_refused() {
        let png = png_file::write(
            &Picture::painted(3, 2, vec![crate::picture::Rgb::WHITE; 6]),
            300,
        )
        .expect("a PNG");
        let jpeg = include_bytes!("../tests/fixtures/red-and-blue.jpg").as_slice();
        let words = b"a goblin described in words".as_slice();

        let sizes = [png.as_slice(), jpeg, words]
            .map(|bytes| read(bytes).map(|picture| (picture.width(), picture.height())));

        assert_eq!(sizes, [Ok((3, 2)), Ok((16, 8)), Err(NeitherPngNorJpeg)]);
    }
}
