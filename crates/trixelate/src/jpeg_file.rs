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

/// A JPEG as its decoder hands it back: its width and height, its pixels row after row as red, green, blue and alpha
/// bytes, and its EXIF block if it has one.
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub exif: Option<Vec<u8>>,
}

/// A JPEG decoded by zune-jpeg, whatever its colour space, or nothing.
pub fn decoded(bytes: &[u8]) -> Option<Decoded> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    let rgba = decoder.decode().ok()?;
    let info = decoder.info()?;
    Some(Decoded {
        width: u32::from(info.width),
        height: u32::from(info.height),
        rgba,
        exif: decoder.exif().cloned(),
    })
}

/// The picture a JPEG holds, as `decode` hands it back, in red, green and blue, turned upright as its camera says.
pub fn read(bytes: &[u8], decode: impl Fn(&[u8]) -> Option<Decoded>) -> Result<Picture, NotAJpeg> {
    let decoded = decode(bytes).ok_or(NotAJpeg)?;
    let stored =
        Picture::from_rgba(decoded.width, decoded.height, &decoded.rgba).map_err(|_| NotAJpeg)?;
    let how = decoded.exif.as_deref().and_then(how_stored);
    Ok(turned(stored, how.unwrap_or(HowStored::AsShown)))
}

/// How a camera stored a photo against the way it is shown, as the EXIF orientation 1 to 8 says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HowStored {
    /// 1: stored as shown.
    AsShown,
    /// 2: stored mirrored, left to right.
    MirroredLeftToRight,
    /// 3: stored upside down.
    UpsideDown,
    /// 4: stored mirrored, top to bottom.
    MirroredTopToBottom,
    /// 5: stored mirrored along its diagonal from the top left.
    MirroredAlongItsDiagonal,
    /// 6: stored a quarter turn anticlockwise.
    QuarterTurnedAnticlockwise,
    /// 7: stored mirrored along its diagonal from the top right.
    MirroredAlongItsOtherDiagonal,
    /// 8: stored a quarter turn clockwise.
    QuarterTurnedClockwise,
}

impl HowStored {
    /// How the EXIF orientation with this number says the photo was stored, none outside 1 to 8.
    fn numbered(number: u16) -> Option<HowStored> {
        use HowStored::*;
        [
            AsShown,
            MirroredLeftToRight,
            UpsideDown,
            MirroredTopToBottom,
            MirroredAlongItsDiagonal,
            QuarterTurnedAnticlockwise,
            MirroredAlongItsOtherDiagonal,
            QuarterTurnedClockwise,
        ]
        .get(usize::from(number).checked_sub(1)?)
        .copied()
    }

    /// Whether the photo was stored on its side, its width and height swapped.
    fn on_its_side(self) -> bool {
        use HowStored::*;
        matches!(
            self,
            MirroredAlongItsDiagonal
                | QuarterTurnedAnticlockwise
                | MirroredAlongItsOtherDiagonal
                | QuarterTurnedClockwise
        )
    }
}

/// The EXIF tag saying how a camera turned the photo it stored.
const ORIENTATION_TAG: u16 = 0x0112;

/// How an EXIF block's first directory says the photo was stored, in its own byte order.
fn how_stored(exif: &[u8]) -> Option<HowStored> {
    let big_endian = [b"MM", b"II"]
        .iter()
        .position(|order| exif.starts_with(*order))?
        == 0;
    let at = |offset: usize, length: usize| exif.get(offset..offset.checked_add(length)?);
    let number = |offset: usize| -> Option<u16> {
        let bytes = at(offset, 2)?.try_into().ok()?;
        Some(if big_endian {
            u16::from_be_bytes(bytes)
        } else {
            u16::from_le_bytes(bytes)
        })
    };
    let first_directory = {
        let bytes = at(4, 4)?.try_into().ok()?;
        (if big_endian {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        }) as usize
    };
    (0..usize::from(number(first_directory)?))
        .filter_map(|entry| first_directory.checked_add(2 + entry * 12))
        .find(|&entry| number(entry) == Some(ORIENTATION_TAG))
        .and_then(|entry| number(entry.checked_add(8)?))
        .and_then(HowStored::numbered)
}

/// A stored picture turned and flipped as its EXIF orientation says, so it shows upright.
fn turned(stored: Picture, how: HowStored) -> Picture {
    let (width, height) = (stored.width(), stored.height());
    let (shown_width, shown_height) = if how.on_its_side() {
        (height, width)
    } else {
        (width, height)
    };
    let stored_at = |x: u32, y: u32| match how {
        HowStored::AsShown => (x, y),
        HowStored::MirroredLeftToRight => (width - 1 - x, y),
        HowStored::UpsideDown => (width - 1 - x, height - 1 - y),
        HowStored::MirroredTopToBottom => (x, height - 1 - y),
        HowStored::MirroredAlongItsDiagonal => (y, x),
        HowStored::QuarterTurnedAnticlockwise => (y, height - 1 - x),
        HowStored::MirroredAlongItsOtherDiagonal => (width - 1 - y, height - 1 - x),
        HowStored::QuarterTurnedClockwise => (width - 1 - y, x),
    };
    let pixels = (0..shown_height)
        .flat_map(|y| (0..shown_width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let (x, y) = stored_at(x, y);
            stored.pixel(x, y)
        })
        .collect();
    Picture::painted(shown_width, shown_height, pixels)
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

    /// The red-cornered JPEG, sixteen by eight, red in its top left quarter and blue elsewhere, its EXIF holding these
    /// tags with their short values, in order, written big-endian as most cameras do, or little-endian.
    fn with_exif(tags: &[(u16, u16)], big_endian: bool) -> Vec<u8> {
        type Number = fn(u16) -> [u8; 2];
        let (order, number): (&[u8], Number) = if big_endian {
            (b"MM\x00\x2a\x00\x00\x00\x08", u16::to_be_bytes)
        } else {
            (b"II\x2a\x00\x08\x00\x00\x00", u16::to_le_bytes)
        };
        let one = if big_endian {
            [0, 0, 0, 1]
        } else {
            [1, 0, 0, 0]
        };
        let entries: Vec<u8> = tags
            .iter()
            .flat_map(|&(tag, value)| {
                [
                    number(tag).as_slice(),
                    &number(3),
                    &one,
                    &number(value),
                    &[0, 0],
                ]
                .concat()
            })
            .collect();
        let tiff = [order, &number(tags.len() as u16), &entries, &[0, 0, 0, 0]].concat();
        with_exif_block(&tiff)
    }

    /// The red-cornered JPEG carrying this EXIF block, whatever it holds.
    fn with_exif_block(tiff: &[u8]) -> Vec<u8> {
        let length = (2 + 6 + tiff.len()) as u16;
        [
            &RED_CORNER[..2],
            &[0xFF, 0xE1],
            &length.to_be_bytes(),
            b"Exif\x00\x00",
            tiff,
            &RED_CORNER[2..],
        ]
        .concat()
    }

    /// The red-cornered JPEG told by its EXIF to be shown turned as a camera says, the orientation its only tag.
    fn told_to_turn(orientation: u16, big_endian: bool) -> Vec<u8> {
        with_exif(&[(ORIENTATION_TAG, orientation)], big_endian)
    }

    /// Where the red quarter shows: the corners of the picture as read that are red, with its width and height.
    fn red_corner_of(picture: &Picture) -> (u32, u32, String) {
        let (width, height) = (picture.width(), picture.height());
        let corners = [
            ((1, 1), "top left"),
            ((width - 2, 1), "top right"),
            ((1, height - 2), "bottom left"),
            ((width - 2, height - 2), "bottom right"),
        ];
        let red: Vec<&str> = corners
            .iter()
            .filter(|&&((x, y), _)| about(picture.pixel(x, y), Rgb(220, 30, 40)))
            .map(|&(_, corner)| corner)
            .collect();
        (width, height, red.join(" and "))
    }

    #[test]
    fn a_photo_is_turned_as_its_camera_says_in_either_byte_order_so_a_sheet_is_never_sideways() {
        let orientations = [1, 2, 3, 4, 5, 6, 7, 8];

        let turned = orientations.map(|orientation| {
            read(&told_to_turn(orientation, orientation % 2 == 0), decoded)
                .map(|picture| red_corner_of(&picture))
        });

        assert_eq!(
            read(RED_CORNER, decoded).map(|picture| red_corner_of(&picture)),
            Ok((16, 8, "top left".to_string()))
        );
        assert_eq!(
            turned,
            [
                Ok((16, 8, "top left".to_string())),
                Ok((16, 8, "top right".to_string())),
                Ok((16, 8, "bottom right".to_string())),
                Ok((16, 8, "bottom left".to_string())),
                Ok((8, 16, "top left".to_string())),
                Ok((8, 16, "top right".to_string())),
                Ok((8, 16, "bottom right".to_string())),
                Ok((8, 16, "bottom left".to_string())),
            ]
        );
    }

    #[test]
    fn a_photo_is_turned_by_the_orientation_its_camera_writes_after_its_make_and_model_and_kept_without_one()
     {
        let (make, model) = (0x010F, 0x0110);
        let photos = [
            with_exif(&[(make, 0), (model, 0), (ORIENTATION_TAG, 6)], true),
            with_exif(&[(make, 0), (model, 0), (ORIENTATION_TAG, 8)], false),
            with_exif(&[(make, 0), (model, 0)], true),
        ];

        let turned =
            photos.map(|photo| read(&photo, decoded).map(|picture| red_corner_of(&picture)));

        assert_eq!(
            turned,
            [
                Ok((8, 16, "top right".to_string())),
                Ok((8, 16, "bottom left".to_string())),
                Ok((16, 8, "top left".to_string())),
            ]
        );
    }

    #[test]
    fn a_photo_whose_exif_names_no_orientation_from_one_to_eight_is_read_as_stored() {
        let photos = [
            told_to_turn(0, true),
            told_to_turn(9, false),
            told_to_turn(u16::MAX, true),
        ];

        let read = photos.map(|photo| read(&photo, decoded).map(|picture| red_corner_of(&picture)));

        assert_eq!(read, [(); 3].map(|_| Ok((16, 8, "top left".to_string()))));
    }

    #[test]
    fn a_photo_whose_exif_block_is_malformed_or_points_past_its_end_is_read_as_stored_not_turned() {
        let tiff = [
            b"MM\x00\x2a\x00\x00\x00\x08".as_slice(),
            &[0, 1],
            &[0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0],
            &[0, 0, 0, 0],
        ]
        .concat();
        let pointing_to = |offset: &[u8]| [&tiff[..4], offset, &tiff[8..]].concat();
        let blocks = [
            tiff.clone(),
            [b"XX", &tiff[2..]].concat(),
            tiff[..19].to_vec(),
            [&tiff[..8], &[0xFF, 0xFF, 0x01, 0x0F], &tiff[12..]].concat(),
            pointing_to(&[0xFF, 0xFF, 0xFF, 0xFF]),
            pointing_to(&[0xFF, 0xFF, 0xFF, 0xF0]),
            tiff[..6].to_vec(),
        ];

        let read = blocks.map(|block| {
            read(&with_exif_block(&block), decoded).map(|picture| red_corner_of(&picture))
        });

        let as_stored = Ok((16, 8, "top left".to_string()));
        assert_eq!(read[0], Ok((8, 16, "top right".to_string())));
        assert_eq!(read[1..], [(); 6].map(|_| as_stored.clone()));
    }

    #[test]
    fn a_jpeg_red_on_its_left_and_blue_on_its_right_reads_so_within_its_compression() {
        let picture = read(RED_AND_BLUE, decoded);

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

    #[test]
    fn a_jpeg_its_decoder_fails_on_or_hands_back_no_pixel_or_pixels_short_of_their_size_is_no_picture()
     {
        let failing = |_: &[u8]| None;
        let empty = |_: &[u8]| {
            Some(Decoded {
                width: 0,
                height: 0,
                rgba: Vec::new(),
                exif: None,
            })
        };
        let short = |_: &[u8]| {
            Some(Decoded {
                width: 2,
                height: 1,
                rgba: vec![255; 4],
                exif: None,
            })
        };

        let read = [
            read(RED_AND_BLUE, failing).err(),
            read(RED_AND_BLUE, empty).err(),
            read(RED_AND_BLUE, short).err(),
        ];

        assert_eq!(read, [Some(NotAJpeg); 3]);
    }
}
