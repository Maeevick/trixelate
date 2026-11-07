//! Pictures read from PNG bytes and sheets written as PNG bytes, at 300 dots per inch.

use crate::picture::Picture;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAPng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CannotWritePng;

/// Reads a picture from a PNG of any colour type and depth.
pub fn read(bytes: &[u8]) -> Result<Picture, NotAPng> {
    let _ = bytes;
    Err(NotAPng)
}

/// Writes a sheet as an RGB PNG that tells printers its 300 dots per inch.
pub fn write(sheet: &Picture) -> Result<Vec<u8>, CannotWritePng> {
    let _ = sheet;
    Ok(vec![])
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
