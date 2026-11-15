//! Reading back the PDFs the crate writes, for its tests: its objects, their dictionaries, and its pages drawn dot by dot.

use std::collections::BTreeMap;

use crate::picture::Rgb;

/// Every object of a PDF by its number, from where its cross-reference table says it starts, which must be there.
pub(crate) fn objects_of(pdf: &[u8]) -> BTreeMap<u32, &[u8]> {
    let text = String::from_utf8_lossy(pdf);
    let table_at: usize = text
        .rsplit_once("startxref\n")
        .and_then(|(_, rest)| rest.lines().next())
        .and_then(|line| line.parse().ok())
        .unwrap_or_default();
    let table = String::from_utf8_lossy(&pdf[table_at..]);
    let mut lines = table.lines();
    assert_eq!(lines.next(), Some("xref"));
    let count: u32 = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|count| count.parse().ok())
        .unwrap_or_default();
    lines
        .skip(1)
        .take(count as usize - 1)
        .zip(1..)
        .map(|(entry, number)| {
            let at: usize = entry[..10].parse().unwrap_or_default();
            let object = &pdf[at..];
            assert!(object.starts_with(format!("{number} 0 obj\n").as_bytes()));
            (number, object)
        })
        .collect()
}

pub(crate) fn dictionary_of(objects: &BTreeMap<u32, &[u8]>, number: u32) -> String {
    let object = String::from_utf8_lossy(objects.get(&number).copied().unwrap_or_default());
    let end = [object.find("stream\n"), object.find("endobj")]
        .into_iter()
        .flatten()
        .min();
    object[..end.unwrap_or_default()].to_string()
}

pub(crate) fn number_after(text: &str, key: &str) -> u32 {
    text.split_once(key)
        .and_then(|(_, rest)| rest.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|number| number.parse().ok())
        .unwrap_or_default()
}

pub(crate) fn numbers_referred_to(text: &str, key: &str) -> Vec<u32> {
    text.split_once(key)
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(references, _)| {
            references
                .split(" 0 R")
                .filter_map(|number| number.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Each page of a PDF drawn dot by dot at so many dots per inch, white until drawn on: its width, its height and its
/// dots row after row. Only what the crate writes is read: filled rectangles and circles, colours, saved and restored
/// states, scalings and moves, and forms drawn in them.
pub(crate) fn pages_drawn(pdf: &[u8], dots_per_inch: u32) -> Vec<(u32, u32, Vec<Rgb>)> {
    let objects = objects_of(pdf);
    let tree = number_after(&dictionary_of(&objects, 1), "/Pages ");
    numbers_referred_to(&dictionary_of(&objects, tree), "/Kids [")
        .into_iter()
        .map(|page| {
            let dictionary = dictionary_of(&objects, page);
            let media_box: Vec<f64> = dictionary
                .split_once("/MediaBox [")
                .and_then(|(_, rest)| rest.split_once(']'))
                .map(|(numbers, _)| {
                    numbers
                        .split_whitespace()
                        .filter_map(|n| n.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
            let scale = f64::from(dots_per_inch) / 72.0;
            let (width, height) = (
                (media_box[2] * scale).round() as u32,
                (media_box[3] * scale).round() as u32,
            );
            let forms: BTreeMap<String, u32> = dictionary
                .split_once("/XObject <<")
                .and_then(|(_, rest)| rest.split_once(">>"))
                .map(|(entries, _)| {
                    entries
                        .split(" 0 R")
                        .filter_map(|entry| {
                            let (name, number) = entry.trim().split_once(' ')?;
                            Some((name.to_string(), number.parse().ok()?))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mut drawn = Drawn {
                width,
                height,
                page_height: media_box[3],
                scale,
                dots: vec![Rgb::WHITE; width as usize * height as usize],
            };
            let contents = number_after(&dictionary, "/Contents ");
            drawn.run(
                &stream_of(&objects, contents),
                &mut State::default(),
                &forms,
                &objects,
            );
            (width, height, drawn.dots)
        })
        .collect()
}

/// A stream object's data, as long as its dictionary says.
pub(crate) fn stream_of(objects: &BTreeMap<u32, &[u8]>, number: u32) -> String {
    let dictionary = dictionary_of(objects, number);
    let length = number_after(&dictionary, "/Length ") as usize;
    let object = objects.get(&number).copied().unwrap_or_default();
    let starts = dictionary.len() + "stream\n".len();
    String::from_utf8_lossy(&object[starts..starts + length]).into_owned()
}

/// What a page's drawing keeps between operators: its scaling and move, its colour, and its states saved.
#[derive(Clone, Copy)]
struct Matrix {
    across: f64,
    down: f64,
    right: f64,
    up: f64,
}

#[derive(Clone)]
struct State {
    matrix: Matrix,
    colour: Rgb,
}

impl Default for State {
    fn default() -> State {
        State {
            matrix: Matrix {
                across: 1.0,
                down: 1.0,
                right: 0.0,
                up: 0.0,
            },
            colour: Rgb(0, 0, 0),
        }
    }
}

struct Drawn {
    width: u32,
    height: u32,
    page_height: f64,
    scale: f64,
    dots: Vec<Rgb>,
}

/// The operators read, each with how many operands it takes: the last draws a form by name.
const READ: [(&str, usize); 9] = [
    ("q", 0),
    ("Q", 0),
    ("cm", 6),
    ("rg", 3),
    ("re", 4),
    ("m", 2),
    ("c", 6),
    ("f", 0),
    ("Do", 0),
];

/// A part of a path: a rectangle, or the points of a curved outline.
enum Part {
    Rectangle(f64, f64, f64, f64),
    Points(Vec<(f64, f64)>),
}

impl Drawn {
    /// A point in the current scaling and move, in dots from the page's top left corner.
    fn dot(&self, matrix: Matrix, (x, y): (f64, f64)) -> (f64, f64) {
        (
            (matrix.across * x + matrix.right) * self.scale,
            (self.page_height - (matrix.down * y + matrix.up)) * self.scale,
        )
    }

    fn run(
        &mut self,
        content: &str,
        state: &mut State,
        forms: &BTreeMap<String, u32>,
        objects: &BTreeMap<u32, &[u8]>,
    ) {
        let mut saved = Vec::new();
        let mut operands: Vec<f64> = Vec::new();
        let mut path: Vec<Part> = Vec::new();
        let mut name = String::new();
        for token in content.split_whitespace() {
            if let Ok(number) = token.parse() {
                operands.push(number);
                continue;
            }
            if token.starts_with('/') {
                name = token.to_string();
                continue;
            }
            assert!(
                READ.contains(&(token, operands.len())),
                "{token} unread, with {operands:?}"
            );
            match (token, operands.as_slice()) {
                ("q", []) => saved.push(state.clone()),
                ("Q", []) => *state = saved.pop().expect("a saved state"),
                ("cm", &[across, 0.0, 0.0, down, right, up]) => {
                    let m = state.matrix;
                    state.matrix = Matrix {
                        across: m.across * across,
                        down: m.down * down,
                        right: m.across * right + m.right,
                        up: m.down * up + m.up,
                    };
                }
                ("rg", &[red, green, blue]) => {
                    let channel = |share: f64| (share * 255.0).round() as u8;
                    state.colour = Rgb(channel(red), channel(green), channel(blue));
                }
                ("re", &[x, y, width, height]) => path.push(Part::Rectangle(x, y, width, height)),
                ("m", &[x, y]) => path.push(Part::Points(vec![(x, y)])),
                ("c", &[x1, y1, x2, y2, x3, y3]) => {
                    if let Some(Part::Points(points)) = path.last_mut() {
                        points.extend([(x1, y1), (x2, y2), (x3, y3)]);
                    }
                }
                ("f", []) => {
                    for part in path.drain(..) {
                        self.fill(&part, state);
                    }
                }
                _ => {
                    let form = forms.get(&name).copied().expect("a form drawn by name");
                    let mut inside = state.clone();
                    self.run(&stream_of(objects, form), &mut inside, forms, objects);
                }
            }
            operands.clear();
        }
    }

    fn fill(&mut self, part: &Part, state: &State) {
        let (left, top, right, bottom) = match part {
            Part::Rectangle(x, y, width, height) => {
                let (left, bottom) = self.dot(state.matrix, (*x, *y));
                let (right, top) = self.dot(state.matrix, (x + width, y + height));
                (left, top, right, bottom)
            }
            Part::Points(points) => {
                let dots: Vec<_> = points.iter().map(|&p| self.dot(state.matrix, p)).collect();
                let least =
                    |pick: fn(&(f64, f64)) -> f64| dots.iter().map(pick).fold(f64::MAX, f64::min);
                let most =
                    |pick: fn(&(f64, f64)) -> f64| dots.iter().map(pick).fold(f64::MIN, f64::max);
                (least(|d| d.0), least(|d| d.1), most(|d| d.0), most(|d| d.1))
            }
        };
        let disc = matches!(part, Part::Points(_)).then(|| {
            let radius = ((right - left) / 2.0).round();
            let centre = (
                ((left + right) / 2.0 - 0.5).round(),
                ((top + bottom) / 2.0 - 0.5).round(),
            );
            (centre, radius)
        });
        // A rectangle covers the dots between its rounded edges; a disc, every dot whose centre is within its radius,
        // those its circle passes through included, as the dots of a page are drawn.
        let rounded = |edge: f64| {
            if disc.is_some() {
                edge.floor()
            } else {
                edge.round()
            }
        };
        let span = |from: f64, to: f64, most: u32| {
            (rounded(from).max(0.0) as u32)..(to.round().max(0.0) as u32).min(most)
        };
        for y in span(top, bottom, self.height) {
            for x in span(left, right, self.width) {
                let inside = disc.is_none_or(|((centre_x, centre_y), radius)| {
                    (f64::from(x) - centre_x).powi(2) + (f64::from(y) - centre_y).powi(2)
                        <= radius.powi(2)
                });
                if inside {
                    self.dots[(y * self.width + x) as usize] = state.colour;
                }
            }
        }
    }
}

/// A PDF's information, by key: its text strings decoded from UTF-16, its dates as written.
pub(crate) fn info_of(pdf: &[u8]) -> BTreeMap<String, String> {
    let objects = objects_of(pdf);
    let trailer = String::from_utf8_lossy(pdf)
        .rsplit_once("trailer")
        .map(|(_, trailer)| trailer.to_string())
        .unwrap_or_default();
    let dictionary = dictionary_of(&objects, number_after(&trailer, "/Info "));
    let mut info = BTreeMap::new();
    let mut rest = dictionary.as_str();
    while let Some((_, after)) = rest.split_once('/') {
        let key_ends = after.find([' ', '<', '(']).unwrap_or(after.len());
        let (key, value) = after.split_at(key_ends);
        let value = value.trim_start();
        let (text, after) = if let Some(hex) = value.strip_prefix('<') {
            let (hex, after) = hex.split_once('>').unwrap_or_default();
            let units: Vec<u16> = (0..hex.len())
                .step_by(4)
                .filter_map(|at| u16::from_str_radix(hex.get(at..at + 4)?, 16).ok())
                .collect();
            assert_eq!(units.first(), Some(&0xFEFF), "a text string in UTF-16");
            (String::from_utf16_lossy(&units[1..]), after)
        } else {
            let literal = value.strip_prefix('(').unwrap_or_default();
            let (literal, after) = literal.split_once(')').unwrap_or_default();
            (literal.to_string(), after)
        };
        info.insert(key.to_string(), text);
        rest = after;
    }
    info
}

/// Each page of a PDF, row after row: its media box as written, and its drawing.
pub(crate) fn pages_of(pdf: &[u8]) -> Vec<(String, String)> {
    let objects = objects_of(pdf);
    let tree = number_after(&dictionary_of(&objects, 1), "/Pages ");
    numbers_referred_to(&dictionary_of(&objects, tree), "/Kids [")
        .into_iter()
        .map(|page| {
            let dictionary = dictionary_of(&objects, page);
            let media_box = dictionary
                .split_once("/MediaBox [")
                .and_then(|(_, rest)| rest.split_once(']'))
                .map(|(media_box, _)| media_box.to_string())
                .unwrap_or_default();
            let contents = number_after(&dictionary, "/Contents ");
            (media_box, stream_of(&objects, contents))
        })
        .collect()
}
