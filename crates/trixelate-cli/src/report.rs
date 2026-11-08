//! What the command says once a sheet is written: what it is, where it went, how fast it came,
//! and how long a five-year-old will take to colour it in.

use std::path::Path;
use std::time::Duration;

use trixelate::layout::Legend;

pub fn written(legend: Legend, path: &Path, took: Duration, cells: u32) -> String {
    let _ = (legend, path, took, cells);
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use trixelate::layout::Model;

    #[test]
    fn a_sheet_says_where_it_went_how_fast_and_how_many_afternoons_its_cells_will_take() {
        let report = written(
            Legend::Coloured(Model::None),
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
    fn a_small_grid_is_coloured_before_the_juice_gets_warm_and_a_middling_one_in_a_rainy_afternoon()
    {
        let [small, middling] = [60, 150].map(|cells| {
            written(
                Legend::Freedom,
                Path::new("goblin.png"),
                Duration::ZERO,
                cells,
            )
        });

        assert!(small.ends_with(
            "60 cells to colour: at 3 a minute, a five-year-old needs about 20 min: done before the juice gets warm."
        ));
        assert!(middling.ends_with(
            "150 cells to colour: at 3 a minute, a five-year-old needs about 50 min: one rainy afternoon."
        ));
    }

    #[test]
    fn a_model_has_nothing_to_colour_and_a_twin_has_its_one_grid() {
        let [model, twin] = [Model::Full, Model::Twin].map(|model| {
            written(
                Legend::Coloured(model),
                Path::new("goblin.png"),
                Duration::ZERO,
                16,
            )
        });

        assert_eq!(
            model,
            "Model written to goblin.png in 0.00 s\nNothing to colour in a model: pin it up, and copy away."
        );
        assert!(twin.starts_with("Twin written to goblin.png in 0.00 s\n16 cells to colour"));
    }
}
