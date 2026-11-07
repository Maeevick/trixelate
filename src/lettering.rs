//! The pixel digits colour numbers are written with, five dots wide and seven tall, one dot apart.

const GLYPH_WIDTH: u32 = 5;
const GLYPH_HEIGHT: u32 = 7;
const SPACE_BETWEEN_DIGITS: u32 = 1;

/// Each digit's rows from the top, the leftmost dot in the highest of five bits.
const DIGITS: [[u8; GLYPH_HEIGHT as usize]; 10] = [
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ],
    [
        0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
    ],
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    [
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ],
    [
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ],
];

/// The dots a number's digits ink, from the top left corner of its first digit, each dot a square of `scale` by `scale`.
pub fn ink(number: &str, scale: u32) -> Vec<(u32, u32)> {
    number
        .chars()
        .filter_map(|digit| digit.to_digit(10))
        .zip(0..)
        .flat_map(|(digit, place)| {
            let left = place * (GLYPH_WIDTH + SPACE_BETWEEN_DIGITS);
            glyph_dots(digit).map(move |(x, y)| (left + x, y))
        })
        .flat_map(|(x, y)| {
            (0..scale).flat_map(move |down| {
                (0..scale).map(move |across| (x * scale + across, y * scale + down))
            })
        })
        .collect()
}

fn glyph_dots(digit: u32) -> impl Iterator<Item = (u32, u32)> {
    let rows = DIGITS[digit as usize];
    (0..GLYPH_HEIGHT).flat_map(move |y| {
        (0..GLYPH_WIDTH)
            .filter(move |x| rows[y as usize] & (1 << (GLYPH_WIDTH - 1 - x)) != 0)
            .map(move |x| (x, y))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(dots: &[(u32, u32)], width: u32, height: u32) -> Vec<String> {
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| if dots.contains(&(x, y)) { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_digit_is_inked_five_dots_wide_and_seven_tall() {
        let dots = ink("2", 1);

        assert_eq!(
            drawn(&dots, 5, 7),
            [
                ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"
            ]
        );
    }

    #[test]
    fn two_digits_sit_side_by_side_one_dot_apart_and_a_scale_inks_each_dot_as_a_square() {
        let dots = ink("17", 2);

        assert_eq!(
            drawn(&dots, 22, 4),
            [
                "....##......##########",
                "....##......##########",
                "..####..............##",
                "..####..............##",
            ]
        );
    }
}
