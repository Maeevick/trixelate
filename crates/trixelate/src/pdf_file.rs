//! Pages written as a PDF, each a picture printed at its dots per inch, for any reader to print at actual size.
//!
//! A page's picture is its PNG's compressed rows as they are: PDF reads them with the same filter and predictors
//! (ISO 32000-1, 7.4.4.4), so no compressor is needed beside the PNG's.

use std::collections::BTreeSet;

use crate::drawing::{Drawing, Mark};
use crate::lettering;
use crate::picture::Rgb;
use crate::sheet::{BLACK, Sheet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CannotWritePdf;

const POINTS_PER_INCH: f64 = 72.0;
const CATALOG: u32 = 1;
const PAGE_TREE: u32 = 2;
const INFO: u32 = 3;

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
        let (width, height) = (sheet.width(), sheet.height());
        let (across, down) = (
            points(width, sheet.dots_per_inch()),
            points(height, sheet.dots_per_inch()),
        );
        pdf.object(
            page,
            format!(
                "<< /Type /Page /Parent {PAGE_TREE} 0 R /MediaBox [0 0 {across} {down}] \
                 /Resources << /XObject << /Piece {picture} 0 R >> >> /Contents {contents} 0 R >>"
            )
            .as_bytes(),
        );
        pdf.stream(
            contents,
            "",
            format!("q {across} 0 0 {down} 0 0 cm /Piece Do Q").as_bytes(),
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
    Ok(pdf.ended(false))
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

    /// The PDF ended with its cross-reference table, each object's start in the order of their numbers, and its
    /// information when it has one.
    fn ended(mut self, informed: bool) -> Vec<u8> {
        let info = if informed {
            format!(" /Info {INFO} 0 R")
        } else {
            String::new()
        };
        self.starts.sort_unstable();
        let table_starts = self.bytes.len();
        let count = self.starts.len() + 1;
        let mut table = format!("xref\n0 {count}\n0000000000 65535 f \n");
        for (_, start) in &self.starts {
            table.push_str(&format!("{start:010} 00000 n \n"));
        }
        table.push_str(&format!(
            "trailer\n<< /Size {count} /Root {CATALOG} 0 R{info} >>\nstartxref\n{table_starts}\n%%EOF\n"
        ));
        self.bytes.extend_from_slice(table.as_bytes());
        self.bytes
    }
}

/// A page of a PDF: its size in points, and its drawing in PDF's operators, from its bottom left corner.
pub(crate) struct Page {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) content: Vec<u8>,
}

/// Four times the tangent of a quarter of a right angle, over three: how far a cubic curve's handles reach for it to
/// follow a quarter circle within three ten-thousandths of its radius.
const CIRCLE_HANDLE: f64 = 0.552_284_749_8;

/// A drawing as a page of its size, its marks as shapes: rectangles, circles, and each character a form of its own.
pub(crate) fn page_of(drawing: &Drawing) -> Page {
    let dots_per_inch = f64::from(drawing.dots_per_inch());
    let in_points = |dots: f64| dots * POINTS_PER_INCH / dots_per_inch;
    let height = f64::from(drawing.height());
    let mut content = String::new();
    for mark in drawing.marks() {
        match mark {
            Mark::Fill {
                left,
                top,
                width,
                height: tall,
                colour,
            } => content.push_str(&format!(
                "{} {} {} {} {} re f\n",
                colour_of(*colour),
                number(in_points(f64::from(*left))),
                number(in_points(height - f64::from(top + tall))),
                number(in_points(f64::from(*width))),
                number(in_points(f64::from(*tall))),
            )),
            Mark::Disc {
                disc,
                outline,
                colour,
            } => {
                let radius = disc.diameter / 2;
                let centre = (
                    in_points(f64::from(disc.centre_x) + 0.5),
                    in_points(height - f64::from(disc.centre_y) - 0.5),
                );
                content.push_str(&format!("{} ", colour_of(BLACK)));
                content.push_str(&circle(centre, in_points(f64::from(radius))));
                if radius > *outline {
                    content.push_str(&format!("{} ", colour_of(*colour)));
                    content.push_str(&circle(centre, in_points(f64::from(radius - outline))));
                }
            }
            Mark::Letters {
                text,
                scale,
                left,
                top,
                colour,
            } => {
                content.push_str(&format!("{}\n", colour_of(*colour)));
                let size = in_points(f64::from(*scale));
                let (_, tall) = lettering::CHARACTER;
                let bottom = in_points(height - f64::from(top + tall * scale));
                for (place, character) in (0..).zip(text.chars()) {
                    if lettering::ink(&character.to_string(), 1).is_empty() {
                        continue;
                    }
                    let x = left + place * lettering::ADVANCE * scale;
                    content.push_str(&format!(
                        "q {} 0 0 {} {} {} cm /L{} Do Q\n",
                        number(size),
                        number(size),
                        number(in_points(f64::from(x))),
                        number(bottom),
                        u32::from(character)
                    ));
                }
            }
        }
    }
    Page {
        width: in_points(f64::from(drawing.width())),
        height: in_points(height),
        content: content.into_bytes(),
    }
}

/// The characters a page writes with ink, each to be a form.
pub(crate) fn characters_of(drawing: &Drawing) -> BTreeSet<char> {
    drawing
        .marks()
        .iter()
        .filter_map(|mark| match mark {
            Mark::Letters { text, .. } => Some(text.chars()),
            _ => None,
        })
        .flatten()
        .filter(|character| !lettering::ink(&character.to_string(), 1).is_empty())
        .collect()
}

/// A filled circle from its centre and radius, in four quarters.
fn circle((x, y): (f64, f64), radius: f64) -> String {
    let handle = radius * CIRCLE_HANDLE;
    let n = |value: f64| number(value);
    format!(
        "{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c f\n",
        n(x + radius),
        n(y),
        n(x + radius),
        n(y + handle),
        n(x + handle),
        n(y + radius),
        n(x),
        n(y + radius),
        n(x - handle),
        n(y + radius),
        n(x - radius),
        n(y + handle),
        n(x - radius),
        n(y),
        n(x - radius),
        n(y - handle),
        n(x - handle),
        n(y - radius),
        n(x),
        n(y - radius),
        n(x + handle),
        n(y - radius),
        n(x + radius),
        n(y - handle),
        n(x + radius),
        n(y),
    )
}

/// A colour as PDF's fill colour, each channel a share of its most.
fn colour_of(Rgb(red, green, blue): Rgb) -> String {
    let share = |channel: u8| number_to(f64::from(channel) / 255.0, 3);
    format!("{} {} {} rg", share(red), share(green), share(blue))
}

/// A number written to the hundredth, as short as it reads.
fn number(value: f64) -> String {
    number_to(value, 2)
}

fn number_to(value: f64, decimals: usize) -> String {
    let written = format!("{value:.decimals$}");
    let trimmed = written.trim_end_matches('0').trim_end_matches('.');
    match trimmed {
        "-0" | "" => "0".to_string(),
        _ => trimmed.to_string(),
    }
}

/// Writes pages as a PDF, in their order, every character they write a form drawn in the fill colour of the moment.
/// What a PDF says of itself: its title, subject and keywords, what made it, and when.
pub(crate) struct About {
    pub(crate) title: String,
    pub(crate) subject: String,
    pub(crate) keywords: String,
    pub(crate) creator: String,
    pub(crate) producer: String,
    /// The moment it was made, in PDF's form: `D:20261111142915Z`.
    pub(crate) made: String,
}

/// A text string in UTF-16, any character written as it is.
fn text(value: &str) -> String {
    let units: String = value
        .encode_utf16()
        .map(|unit| format!("{unit:04X}"))
        .collect();
    format!("<FEFF{units}>")
}

pub(crate) fn write_pages(pages: &[Page], characters: &BTreeSet<char>, about: &About) -> Vec<u8> {
    let mut pdf = Pdf {
        bytes: b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(),
        starts: Vec::new(),
    };
    pdf.object(
        CATALOG,
        format!("<< /Type /Catalog /Pages {PAGE_TREE} 0 R >>").as_bytes(),
    );
    pdf.object(
        INFO,
        format!(
            "<< /Title {} /Subject {} /Keywords {} /Creator {} /Producer {} /CreationDate ({}) >>",
            text(&about.title),
            text(&about.subject),
            text(&about.keywords),
            text(&about.creator),
            text(&about.producer),
            about.made
        )
        .as_bytes(),
    );
    let mut forms = Vec::new();
    let (width, height) = lettering::CHARACTER;
    for (number, character) in (INFO + 1..).zip(characters) {
        let dots: String = lettering::ink(&character.to_string(), 1)
            .into_iter()
            .map(|(x, y)| format!("{x} {} 1 1 re\n", height - 1 - y))
            .collect();
        pdf.stream(
            number,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 {width} {height}] "),
            format!("{dots}f").as_bytes(),
        );
        forms.push(format!("/L{} {number} 0 R", u32::from(*character)));
    }
    let resources = format!("<< /XObject << {} >> >>", forms.join(" "));
    let first_page = INFO + 1 + characters.len() as u32;
    let mut kids = Vec::new();
    for (page, drawn) in (first_page..).step_by(2).zip(pages) {
        let contents = page + 1;
        pdf.object(
            page,
            format!(
                "<< /Type /Page /Parent {PAGE_TREE} 0 R /MediaBox [0 0 {} {}] /Resources {resources} /Contents {contents} 0 R >>",
                number(drawn.width),
                number(drawn.height)
            )
            .as_bytes(),
        );
        pdf.stream(contents, "", &drawn.content);
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
    pdf.ended(true)
}
