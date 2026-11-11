//! Reads a picture from JPEG bytes, as cameras and the web save photos.

use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::picture::Picture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAJpeg;

/// A JPEG's width and height, read from its headers alone.
pub(crate) fn size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut decoder = JpegDecoder::new(ZCursor::new(bytes));
    decoder.decode_headers().ok()?;
    let info = decoder.info()?;
    Some((u32::from(info.width), u32::from(info.height)))
}

/// The picture a JPEG holds, whatever its colour space, in red, green and blue.
pub fn read(bytes: &[u8]) -> Result<Picture, NotAJpeg> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    let rgba = decoder.decode().map_err(|_| NotAJpeg)?;
    let size = decoder.info().ok_or(NotAJpeg)?;
    Picture::from_rgba(u32::from(size.width), u32::from(size.height), &rgba).map_err(|_| NotAJpeg)
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
