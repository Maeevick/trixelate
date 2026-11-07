//! Pictures read from PNG bytes and sheets written as PNG bytes, at 300 dots per inch.

use std::io::Cursor;

use png::{BitDepth, ColorType, Decoder, Encoder, PixelDimensions, Transformations, Unit};

use crate::picture::{Picture, Rgb};

const DOTS_PER_METRE_AT_300_DOTS_PER_INCH: u32 = 11_811;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAPng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CannotWritePng;

/// Reads a picture from a PNG of any colour type and depth.
pub fn read(bytes: &[u8]) -> Result<Picture, NotAPng> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| NotAPng)?;
    let mut buffer = vec![0; reader.output_buffer_size().ok_or(NotAPng)?];
    let frame = reader.next_frame(&mut buffer).map_err(|_| NotAPng)?;
    let samples = &buffer[..frame.buffer_size()];
    let rgba: Vec<u8> = samples
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[red, green, blue]| [red, green, blue, 255])
        .collect();
    Picture::from_rgba(frame.width, frame.height, &rgba).map_err(|_| NotAPng)
}

/// Writes a sheet as an RGB PNG that tells printers its 300 dots per inch.
pub fn write(sheet: &Picture) -> Result<Vec<u8>, CannotWritePng> {
    let rgb: Vec<u8> = (0..sheet.height())
        .flat_map(|y| (0..sheet.width()).map(move |x| sheet.pixel(x, y)))
        .flat_map(|Rgb(red, green, blue)| [red, green, blue])
        .collect();
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, sheet.width(), sheet.height());
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_pixel_dims(Some(PixelDimensions {
        xppu: DOTS_PER_METRE_AT_300_DOTS_PER_INCH,
        yppu: DOTS_PER_METRE_AT_300_DOTS_PER_INCH,
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

        let bytes = write(&sheet).unwrap_or_default();

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
}
