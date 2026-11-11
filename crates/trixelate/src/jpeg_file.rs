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

    const RED_CORNER: &[u8] = include_bytes!("../tests/fixtures/red-corner.jpg");

    /// The red-cornered JPEG, sixteen by eight, red in its top left quarter and blue elsewhere, told by its EXIF to be
    /// shown turned as a camera says, the tag written big-endian as most cameras do, or little-endian.
    fn told_to_turn(orientation: u16, big_endian: bool) -> Vec<u8> {
        let (order, number): (&[u8], fn(u16) -> [u8; 2]) = if big_endian {
            (b"MM\x00\x2a\x00\x00\x00\x08", u16::to_be_bytes)
        } else {
            (b"II\x2a\x00\x08\x00\x00\x00", u16::to_le_bytes)
        };
        let entry = [
            number(1).as_slice(),
            &number(0x0112),
            &number(3),
            &if big_endian {
                [0, 0, 0, 1]
            } else {
                [1, 0, 0, 0]
            },
            &number(orientation),
            &[0, 0],
            &[0, 0, 0, 0],
        ]
        .concat();
        let tiff = [order, &entry].concat();
        let length = (2 + 6 + tiff.len()) as u16;
        [
            &RED_CORNER[..2],
            &[0xFF, 0xE1],
            &length.to_be_bytes(),
            b"Exif\x00\x00",
            &tiff,
            &RED_CORNER[2..],
        ]
        .concat()
    }

    /// Where the red quarter shows: which corner of the picture as read, with its width and height.
    fn red_corner_of(picture: &Picture) -> (u32, u32, &'static str) {
        let (width, height) = (picture.width(), picture.height());
        let corners = [
            ((1, 1), "top left"),
            ((width - 2, 1), "top right"),
            ((1, height - 2), "bottom left"),
            ((width - 2, height - 2), "bottom right"),
        ];
        let red = corners
            .iter()
            .filter(|&&((x, y), _)| about(picture.pixel(x, y), Rgb(220, 30, 40)))
            .map(|&(_, corner)| corner)
            .collect::<Vec<_>>();
        (
            width,
            height,
            if red.len() == 1 {
                red[0]
            } else {
                "nowhere or everywhere"
            },
        )
    }

    #[test]
    fn a_photo_is_turned_as_its_camera_says_in_either_byte_order_so_a_sheet_is_never_sideways() {
        let orientations = [1, 2, 3, 4, 5, 6, 7, 8];

        let turned = orientations.map(|orientation| {
            read(&told_to_turn(orientation, orientation % 2 == 0))
                .map(|picture| red_corner_of(&picture))
        });

        assert_eq!(
            read(RED_CORNER).map(|picture| red_corner_of(&picture)),
            Ok((16, 8, "top left"))
        );
        assert_eq!(
            turned,
            [
                Ok((16, 8, "top left")),
                Ok((16, 8, "top right")),
                Ok((16, 8, "bottom right")),
                Ok((16, 8, "bottom left")),
                Ok((8, 16, "top left")),
                Ok((8, 16, "top right")),
                Ok((8, 16, "bottom right")),
                Ok((8, 16, "bottom left")),
            ]
        );
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
