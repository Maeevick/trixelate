//! What the command says once a sheet is written: where it went, how fast it came,
//! and how long a five-year-old will take to colour it in.

use std::path::Path;
use std::time::Duration;

/// How many cells a five-year-old colours in a minute, picking each crayon included.
const CELLS_A_MINUTE: u32 = 3;
/// How long a five-year-old keeps colouring in one go.
const MINUTES_AN_AFTERNOON: u32 = 30;

/// What the command wrote: the sheet to colour in, or its preview, coloured in.
#[derive(Clone, Copy)]
pub enum Written {
    Sheet,
    Preview,
}

/// The report of a sheet or its preview written to a path, after the time it took, its grid holding a number of cells.
pub fn written(_what: Written, path: &Path, took: Duration, cells: u32) -> String {
    format!(
        "Sheet written to {} in {:.2} s\n{}",
        path.display(),
        took.as_secs_f64(),
        how_long_to_colour(cells)
    )
}

fn how_long_to_colour(cells: u32) -> String {
    let minutes = cells.div_ceil(CELLS_A_MINUTE);
    let time = match minutes {
        ..=20 => format!("about {minutes} min: done before the juice gets warm"),
        21..=90 => format!("about {minutes} min: one rainy afternoon"),
        _ => format!(
            "about {} h {:02} min, or {} afternoons of half an hour, snack breaks not included",
            minutes / 60,
            minutes % 60,
            minutes.div_ceil(MINUTES_AN_AFTERNOON)
        ),
    };
    format!(
        "{} cells to colour: at {CELLS_A_MINUTE} a minute, a five-year-old needs {time}.",
        in_thousands(cells)
    )
}

/// A count written with a comma between its thousands, as an English reader expects.
fn in_thousands(count: u32) -> String {
    let digits = count.to_string();
    digits
        .chars()
        .enumerate()
        .flat_map(|(place, digit)| {
            let comma = place > 0 && (digits.len() - place).is_multiple_of(3);
            comma.then_some(',').into_iter().chain([digit])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_says_where_it_went_how_fast_and_how_many_afternoons_its_cells_will_take() {
        let report = written(
            Written::Sheet,
            Path::new("goblin-48-8-a4.sheet.png"),
            Duration::from_millis(213),
            2304,
        );

        assert_eq!(
            report,
            "Sheet written to goblin-48-8-a4.sheet.png in 0.21 s\n\
             2,304 cells to colour: at 3 a minute, a five-year-old needs about 12 h 48 min, \
             or 26 afternoons of half an hour, snack breaks not included."
        );
    }

    #[test]
    fn a_preview_says_it_is_one_where_it_went_and_how_fast() {
        let report = written(
            Written::Preview,
            Path::new("goblin-48-8-a4.preview.png"),
            Duration::from_millis(213),
            2304,
        );

        assert!(report.starts_with("Preview written to goblin-48-8-a4.preview.png in 0.21 s\n"));
    }

    #[test]
    fn a_small_grid_is_coloured_before_the_juice_gets_warm_and_a_middling_one_in_a_rainy_afternoon()
    {
        let [small, middling] = [60, 150].map(|cells| {
            written(
                Written::Sheet,
                Path::new("goblin.png"),
                Duration::ZERO,
                cells,
            )
        });

        assert!(small.ends_with(
            "\n60 cells to colour: at 3 a minute, a five-year-old needs about 20 min: done before the juice gets warm."
        ));
        assert!(middling.ends_with(
            "\n150 cells to colour: at 3 a minute, a five-year-old needs about 50 min: one rainy afternoon."
        ));
    }
}
