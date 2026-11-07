//! Reads a picture from JPEG bytes, as cameras and the web save photos.

use crate::picture::Picture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAJpeg;

/// The picture a JPEG holds, whatever its colour space, in red, green and blue.
pub fn read(bytes: &[u8]) -> Result<Picture, NotAJpeg> {
    let _ = bytes;
    Err(NotAJpeg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picture::Rgb;

    const RED_AND_BLUE: &[u8] = include_bytes!("../tests/fixtures/red-and-blue.jpg");

    fn about(Rgb(red, green, blue): Rgb, Rgb(r, g, b): Rgb) -> bool {
        [(red, r), (green, g), (blue, b)]
            .iter()
            .all(|&(one, other)| one.abs_diff(other) <= 12)
    }

    #[test]
    fn a_jpeg_red_on_its_left_and_blue_on_its_right_reads_so_within_its_compression() {
        let picture = read(RED_AND_BLUE);

        let seen = picture.map(|picture| {
            (
                picture.width(),
                picture.height(),
                about(picture.pixel(2, 4), Rgb(220, 30, 40)),
                about(picture.pixel(13, 4), Rgb(20, 60, 200)),
            )
        });

        assert_eq!(seen, Ok((16, 8, true, true)));
    }
}
