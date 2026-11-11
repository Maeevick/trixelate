//! The Trunk hook that writes the release page's Content-Security-Policy: see the library. Debug builds keep none, as
//! `trunk serve` adds its own reloading script.

use std::process::ExitCode;
use std::{env, fs};

use trixelate_csp::{NoCharset, with_policy};

fn main() -> ExitCode {
    if env::var("TRUNK_PROFILE").as_deref() != Ok("release") {
        return ExitCode::SUCCESS;
    }
    let Ok(staging) = env::var("TRUNK_STAGING_DIR") else {
        eprintln!("error: Trunk gave no TRUNK_STAGING_DIR to write the policy in");
        return ExitCode::FAILURE;
    };
    let page = std::path::Path::new(&staging).join("index.html");
    let written = fs::read_to_string(&page)
        .map_err(|error| format!("cannot read {}: {error}", page.display()))
        .and_then(|built| {
            with_policy(&built).map_err(|NoCharset| {
                format!(
                    "{} lost its charset, which the policy is written after",
                    page.display()
                )
            })
        })
        .and_then(|with_its_policy| {
            fs::write(&page, with_its_policy)
                .map_err(|error| format!("cannot write {}: {error}", page.display()))
        });
    match written {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("error: {why}");
            ExitCode::FAILURE
        }
    }
}
