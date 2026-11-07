//! The command line: reads a PNG picture, makes its colour-by-number sheet and writes it as a PNG.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use trixelate::grid::GridSize;
use trixelate::layout::{self, Margin, NoRoom, PageFormat};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::{Order, make_sheet, png_file};

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

    /// The paper the sheet is drawn for, at 300 dots per inch
    #[arg(short, long, value_enum, default_value_t = Format::A4)]
    format: Format,

    /// Which way up the paper lies
    #[arg(long, value_enum, default_value_t = Orientation::Portrait)]
    orientation: Orientation,

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

#[derive(Clone, Copy, ValueEnum)]
enum Orientation {
    Portrait,
    Landscape,
}

impl From<Orientation> for layout::Orientation {
    fn from(orientation: Orientation) -> layout::Orientation {
        match orientation {
            Orientation::Portrait => layout::Orientation::Portrait,
            Orientation::Landscape => layout::Orientation::Landscape,
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
    match run(Cli::parse()) {
        Ok(written) => {
            println!("{written}");
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("error: {failure}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<String, String> {
    let picture = read_picture(&cli.picture)?;
    let grid = GridSize::new(cli.width, cli.height)
        .map_err(|_| "a grid needs at least one cell".to_string())?;
    let order = Order {
        grid,
        colours: cli.colours,
        format: cli.format.into(),
        orientation: cli.orientation.into(),
        margin: Margin(cli.margin),
    };
    let sheet = make_sheet(&picture, order).map_err(explain)?;
    let sheet_path = cli.output.unwrap_or_else(|| beside(&cli.picture));
    let bytes = png_file::write(&sheet)
        .map_err(|_| "the sheet could not be encoded as a PNG".to_string())?;
    write_sheet(&sheet_path, &bytes, cli.force)?;
    Ok(format!("Sheet written to {}", sheet_path.display()))
}

/// Writes the sheet, refusing to replace an existing file unless forced, even if one appears meanwhile.
fn write_sheet(path: &Path, bytes: &[u8], force: bool) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .create_new(!force)
        .open(path)
        .map_err(|error| match error.kind() {
            ErrorKind::AlreadyExists => format!(
                "{} already exists: pass --force to overwrite it",
                path.display()
            ),
            _ => format!("cannot write the sheet to {}: {error}", path.display()),
        })?;
    file.write_all(bytes)
        .map_err(|error| format!("cannot write the sheet to {}: {error}", path.display()))
}

fn read_picture(path: &Path) -> Result<Picture, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    png_file::read(&bytes).map_err(|_| format!("{} is not a PNG picture", path.display()))
}

/// Where a sheet goes when no output is given: beside its picture, as <picture>.sheet.png.
fn beside(picture: &Path) -> PathBuf {
    let stem = picture.file_stem().unwrap_or_default().to_string_lossy();
    picture.with_file_name(format!("{stem}.sheet.png"))
}

fn explain(no_room: NoRoom) -> String {
    match no_room {
        NoRoom::InsideTheMargin => "the margin leaves no room for the grid and its legend: try a smaller --margin",
        NoRoom::ForTheNumbersInTheCells => {
            "the cells are too small for their numbers: try fewer cells with --width and --height, or a bigger --format"
        }
        NoRoom::ForTheNumbersInTheLegend => {
            "the bubbles are too small for their numbers: try fewer --colours, or a smaller --margin"
        }
    }
    .to_string()
}
