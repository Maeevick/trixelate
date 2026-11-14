//! Pages written as a PDF, each a picture printed at its dots per inch, for any reader to print at actual size.
//!
//! A page's picture is its PNG's compressed rows as they are: PDF reads them with the same filter and predictors
//! (ISO 32000-1, 7.4.4.4), so no compressor is needed beside the PNG's.

use crate::sheet::Sheet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CannotWritePdf;

const POINTS_PER_INCH: f64 = 72.0;
const CATALOG: u32 = 1;
const PAGE_TREE: u32 = 2;

/// Writes pages as a PDF, in their order, each page as big as its picture printed at its dots per inch.
pub(crate) fn write(pages: impl Iterator<Item = Sheet>) -> Result<Vec<u8>, CannotWritePdf> {
    let mut pdf = Pdf {
        bytes: b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(),
        starts: Vec::new(),
    };
    pdf.object(
        CATALOG,
        format!("<< /Type /Catalog /Pages {PAGE_TREE} 0 R >>").as_bytes(),
    );
    let mut kids = Vec::new();
    for (page, sheet) in (PAGE_TREE + 1..).step_by(3).zip(pages) {
        let (contents, picture) = (page + 1, page + 2);
        let size = format!(
            "{} {}",
            points(sheet.width(), sheet.dots_per_inch()),
            points(sheet.height(), sheet.dots_per_inch())
        );
        pdf.object(
            page,
            format!(
                "<< /Type /Page /Parent {PAGE_TREE} 0 R /MediaBox [0 0 {size}] \
                 /Resources << /XObject << /Piece {picture} 0 R >> >> /Contents {contents} 0 R >>"
            )
            .as_bytes(),
        );
        let (width, height) = (sheet.width(), sheet.height());
        pdf.stream(
            contents,
            "",
            format!(
                "q {} 0 0 {} 0 0 cm /Piece Do Q",
                points(width, sheet.dots_per_inch()),
                points(height, sheet.dots_per_inch())
            )
            .as_bytes(),
        );
        let png = sheet.png().map_err(|_| CannotWritePdf)?;
        pdf.stream(
            picture,
            &format!(
                "/Type /XObject /Subtype /Image /Width {width} /Height {height} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode \
                 /DecodeParms << /Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns {width} >> "
            ),
            &compressed_rows(&png).ok_or(CannotWritePdf)?,
        );
        kids.push(format!("{page} 0 R"));
    }
    pdf.object(
        PAGE_TREE,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            kids.len()
        )
        .as_bytes(),
    );
    Ok(pdf.ended())
}

/// A length in dots printed at so many dots per inch, in PDF's points, seventy-two to the inch.
fn points(dots: u32, dots_per_inch: u32) -> f64 {
    f64::from(dots) * POINTS_PER_INCH / f64::from(dots_per_inch)
}

/// A PNG's compressed rows: the data of its image chunks, one after the other.
fn compressed_rows(png: &[u8]) -> Option<Vec<u8>> {
    let mut rows = Vec::new();
    let mut chunks = png.get(8..)?;
    while let [a, b, c, d, kind @ ..] = chunks {
        let length = u32::from_be_bytes([*a, *b, *c, *d]) as usize;
        let data = kind.get(4..4 + length)?;
        if kind.starts_with(b"IDAT") {
            rows.extend_from_slice(data);
        }
        chunks = kind.get(4 + length + 4..)?;
    }
    Some(rows)
}

/// A PDF being written: its bytes so far, and where each object starts in them.
struct Pdf {
    bytes: Vec<u8>,
    starts: Vec<(u32, usize)>,
}

impl Pdf {
    fn object(&mut self, number: u32, body: &[u8]) {
        self.starts.push((number, self.bytes.len()));
        self.bytes
            .extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        self.bytes.extend_from_slice(body);
        self.bytes.extend_from_slice(b"\nendobj\n");
    }

    fn stream(&mut self, number: u32, dictionary: &str, data: &[u8]) {
        let mut body = format!("<< {dictionary}/Length {} >>\nstream\n", data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        self.object(number, &body);
    }

    /// The PDF ended with its cross-reference table, each object's start in the order of their numbers.
    fn ended(mut self) -> Vec<u8> {
        self.starts.sort_unstable();
        let table_starts = self.bytes.len();
        let count = self.starts.len() + 1;
        let mut table = format!("xref\n0 {count}\n0000000000 65535 f \n");
        for (_, start) in &self.starts {
            table.push_str(&format!("{start:010} 00000 n \n"));
        }
        table.push_str(&format!(
            "trailer\n<< /Size {count} /Root {CATALOG} 0 R >>\nstartxref\n{table_starts}\n%%EOF\n"
        ));
        self.bytes.extend_from_slice(table.as_bytes());
        self.bytes
    }
}
