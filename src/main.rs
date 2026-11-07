//! The command line: reads a PNG picture, makes its colour-by-number sheet and writes it as a PNG.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use trixelate::layout::PageFormat;
use trixelate::palette::ColourCount;

/// Turns a picture into a colour-by-number pixel sheet, ready to print.
#[derive(Parser)]
#[command(
    version,
    after_help = "Exit codes: 0 the sheet is written, 1 the picture or the sheet failed, 2 the arguments are wrong."
)]
struct Cli {
    /// The PNG picture to turn into a sheet
    picture: PathBuf,

    /// Cells across the grid
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    width: u32,

    /// Cells down the grid
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    height: u32,

    /// Colours in the palette, from 2 to 30
    #[arg(short, long, visible_alias = "colors", default_value = "8", value_parser = colour_count)]
    colours: ColourCount,

    /// The paper the sheet is drawn for, upright, at 300 dots per inch
    #[arg(short, long, value_enum, default_value_t = Format::A4)]
    format: Format,

    /// The blank paper around the sheet, in millimetres
    #[arg(short, long, default_value_t = 20)]
    margin: u32,

    /// Where to write the sheet [default: beside the picture, as <picture>.sheet.png]
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Overwrite the sheet if it already exists
    #[arg(long)]
    force: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    A3,
    A4,
    A5,
}

impl From<Format> for PageFormat {
    fn from(format: Format) -> PageFormat {
        match format {
            Format::A3 => PageFormat::A3,
            Format::A4 => PageFormat::A4,
            Format::A5 => PageFormat::A5,
        }
    }
}

fn colour_count(raw: &str) -> Result<ColourCount, String> {
    let count: usize = raw.parse().map_err(|_| format!("{raw} is not a number"))?;
    ColourCount::new(count).map_err(|_| {
        format!(
            "a sheet holds from {} to {} colours",
            ColourCount::FEWEST,
            ColourCount::MOST
        )
    })
}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    ExitCode::SUCCESS
}
