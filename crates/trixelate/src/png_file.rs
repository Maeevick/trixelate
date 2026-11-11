//! Pictures read from PNG bytes and sheets written as PNG bytes, at 300 dots per inch.

use std::io::Cursor;

use png::{BitDepth, ColorType, Decoder, Encoder, PixelDimensions, Transformations, Unit};

use crate::picture::{Picture, Rgb};

const TENTHS_OF_MILLIMETRE_PER_INCH: u32 = 254;
const TENTHS_OF_MILLIMETRE_PER_METRE: u32 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAPng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CannotWritePng;

/// A PNG's width and height, read from its header alone.
pub(crate) fn size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    let info = decoder.read_header_info().ok()?;
    Some((info.width, info.height))
}

/// Reads a picture from a PNG of any colour type and depth.
pub fn read(bytes: &[u8]) -> Result<Picture, NotAPng> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| NotAPng)?;
    let mut buffer = vec![0; reader.output_buffer_size().ok_or(NotAPng)?];
    let frame = reader.next_frame(&mut buffer).map_err(|_| NotAPng)?;
    let samples = &buffer[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        ColorType::Grayscale => samples
            .iter()
            .flat_map(|&grey| [grey, grey, grey, 255])
            .collect(),
        ColorType::GrayscaleAlpha => samples
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[grey, alpha]| [grey, grey, grey, alpha])
            .collect(),
        ColorType::Rgba => samples.to_vec(),
        ColorType::Rgb | ColorType::Indexed => samples
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[red, green, blue]| [red, green, blue, 255])
            .collect(),
    };
    Picture::from_rgba(frame.width, frame.height, &rgba).map_err(|_| NotAPng)
}

/// Writes a sheet as an RGB PNG that tells printers how many dots per inch it is printed at.
pub fn write(sheet: &Picture, dots_per_inch: u32) -> Result<Vec<u8>, CannotWritePng> {
    let dots_per_metre = (dots_per_inch * TENTHS_OF_MILLIMETRE_PER_METRE
        + TENTHS_OF_MILLIMETRE_PER_INCH / 2)
        / TENTHS_OF_MILLIMETRE_PER_INCH;
    let rgb: Vec<u8> = sheet
        .pixels()
        .iter()
        .flat_map(|&Rgb(red, green, blue)| [red, green, blue])
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, sheet.width(), sheet.height());
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_pixel_dims(Some(PixelDimensions {
        xppu: dots_per_metre,
        yppu: dots_per_metre,
        unit: Unit::Meter,
    }));
    let mut writer = encoder.write_header().map_err(|_| CannotWritePng)?;
    writer.write_image_data(&rgb).map_err(|_| CannotWritePng)?;
    writer.finish().map_err(|_| CannotWritePng)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_written_as_png_reads_back_the_same_and_tells_its_300_dots_per_inch() {
        let rgba = [
            220, 30, 40, 255, 250, 220, 60, 255, 20, 60, 200, 255, 255, 255, 255, 255,
        ];
        let sheet = Picture::from_rgba(2, 2, &rgba).expect("a picture");

        let bytes = write(&sheet, 300).unwrap_or_default();

        let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
        let dots_per_metre = decoder
            .read_info()
            .ok()
            .and_then(|reader| reader.info().pixel_dims);
        assert_eq!(read(&bytes).ok(), Some(sheet));
        assert_eq!(
            dots_per_metre.map(|dims| (dims.xppu, dims.yppu, dims.unit)),
            Some((11811, 11811, png::Unit::Meter))
        );
    }

    #[test]
    fn a_poster_written_as_png_tells_printers_its_150_dots_per_inch() {
        let sheet = Picture::from_rgba(1, 1, &[220, 30, 40, 255]).expect("a picture");

        let bytes = write(&sheet, 150).unwrap_or_default();

        let decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
        let dots_per_metre = decoder
            .read_info()
            .ok()
            .and_then(|reader| reader.info().pixel_dims);
        assert_eq!(
            dots_per_metre.map(|dims| (dims.xppu, dims.yppu, dims.unit)),
            Some((5906, 5906, png::Unit::Meter))
        );
    }

    fn one_pixel_png(
        colour: ColorType,
        depth: BitDepth,
        samples: &[u8],
        palette: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(colour);
        encoder.set_depth(depth);
        if let Some(palette) = palette {
            encoder.set_palette(palette.to_vec());
        }
        let mut writer = encoder.write_header().expect("a header");
        writer.write_image_data(samples).expect("the pixel");
        writer.finish().expect("a PNG");
        bytes
    }

    #[test]
    fn a_png_of_any_colour_type_and_depth_reads_as_colours_laid_on_white_paper() {
        let pngs = [
            one_pixel_png(ColorType::Grayscale, BitDepth::Eight, &[100], None),
            one_pixel_png(ColorType::GrayscaleAlpha, BitDepth::Eight, &[100, 0], None),
            one_pixel_png(ColorType::Rgba, BitDepth::Eight, &[10, 20, 30, 255], None),
            one_pixel_png(ColorType::Indexed, BitDepth::Eight, &[0], Some(&[9, 8, 7])),
            one_pixel_png(
                ColorType::Rgb,
                BitDepth::Sixteen,
                &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc],
                None,
            ),
        ];

        let pixels = pngs.map(|png| read(&png).map(|picture| picture.pixel(0, 0)));

        assert_eq!(
            pixels,
            [
                Ok(Rgb(100, 100, 100)),
                Ok(Rgb::WHITE),
                Ok(Rgb(10, 20, 30)),
                Ok(Rgb(9, 8, 7)),
                Ok(Rgb(0x12, 0x56, 0x9a)),
            ]
        );
    }
}
