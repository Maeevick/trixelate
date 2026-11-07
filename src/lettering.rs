//! The pixel digits colour numbers are written with, five dots wide and seven tall, one dot apart.

/// The dots a number's digits ink, from the top left corner of its first digit, each dot a square of `scale` by `scale`.
pub fn ink(number: &str, scale: u32) -> Vec<(u32, u32)> {
    let _ = (number, scale);
    vec![]
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
}
