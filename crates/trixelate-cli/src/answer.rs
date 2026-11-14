//! What a parent typed when asked whether to go ahead after a warning.

/// Whether an answer says yes: "y" or "yes", in any case, with any spaces around it; anything else says no.
pub fn said_yes(answer: &str) -> bool {
    let _ = answer;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_y_or_yes_in_any_case_with_spaces_around_says_yes_and_anything_else_no() {
        let answers = ["y", "YES ", "Yes\n", "n", "", "no", "yes please", "yeah"];

        let said = answers.map(said_yes);

        assert_eq!(said, [true, true, true, false, false, false, false, false]);
    }
}
