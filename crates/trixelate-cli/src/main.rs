//! The command line: reads a PNG picture, makes its colour-by-number sheet and writes it as a PNG.

mod report;

use std::fs;
use std::io::{BufRead, ErrorKind, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::error::ErrorKind as ArgumentError;
use clap::{CommandFactory, Parser, ValueEnum};
use trixelate::grid::GridSize;
use trixelate::layout::{self, Margin, NoRoom, PageFormat};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::warning::warnings_for;
use trixelate::{Order, make_sheet, picture_file, png_file, sheet_file_name};

/// Turns a picture into a colour-by-number pixel sheet, ready to print.
#[derive(Parser)]
#[command(
    name = "trixelate",
    version,
    after_help = "Exit codes: 0 the sheet is written, 1 the picture or the sheet failed, 2 the arguments are wrong."
)]
struct Cli {
    /// The PNG or JPEG picture to turn into a sheet
    picture: PathBuf,

    /// Cells across the grid, the rows following from the picture's proportions
    #[arg(long, default_value_t = 48, value_parser = clap::value_parser!(u32).range(1..))]
    width: u32,

    /// Colours in the palette, from 2 to 24
    #[arg(short, long, visible_alias = "colors", default_value = "8", value_parser = colour_count)]
    colours: ColourCount,

    /// The paper the sheet is drawn for, at 300 dots per inch
    #[arg(short, long, value_enum, default_value_t = Format::A4)]
    format: Format,

    /// Lay the paper on its side, for wide pictures [default: upright]
    #[arg(long)]
    landscape: bool,

    /// The blank paper around the sheet, in millimetres
    #[arg(short, long, default_value_t = 20)]
    margin: u32,

    /// No legend: the numbers only group the cells, and the child picks the colours
    #[arg(long, group = "look")]
    freedom: bool,

    /// Paint the sheet in its colours, a model to copy from
    #[arg(long, group = "look")]
    model: bool,

    /// Put the model and the sheet on one page: one above the other in portrait, side by side in landscape
    #[arg(long, group = "look")]
    twin: bool,

    /// Where to write the sheet [default: beside the picture, as <picture>-<width>-<colours>-<format>.sheet.png, .model.png or .twin.png, with -landscape and -freedom when asked]
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Overwrite the sheet if it already exists
    #[arg(long)]
    force: bool,

    /// Go ahead without asking when the sheet is warned of: under 16 cells across or 4 colours
    #[arg(short, long)]
    yes: bool,

    /// Go beyond the most cells across the paper holds for a child: 128 on A3, 96 on A4, 64 on A5, half in a twin
    #[arg(long)]
    chaos: bool,
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

/// The legend asked for, coloured unless free, with the model asked for, if any; clap lets one through at most.
fn legend_of(cli: &Cli) -> layout::Legend {
    match (cli.freedom, cli.model, cli.twin) {
        (true, _, _) => layout::Legend::Freedom,
        (_, true, _) => layout::Legend::Coloured(layout::Model::Full),
        (_, _, true) => layout::Legend::Coloured(layout::Model::Twin),
        (false, false, false) => layout::Legend::Coloured(layout::Model::None),
    }
}

/// Refuses more cells across than the paper holds for a child, unless chaos is asked.
fn within_the_limits(cli: &Cli) -> Result<(), String> {
    let legend = legend_of(cli);
    let most = layout::most_cells_across(cli.format.into(), legend);
    if cli.chaos || cli.width <= most {
        return Ok(());
    }
    let paper = match cli.format {
        Format::A3 => "A3",
        Format::A4 => "A4",
        Format::A5 => "A5",
    };
    let kind = match legend {
        layout::Legend::Coloured(layout::Model::None) | layout::Legend::Freedom => "sheet",
        layout::Legend::Coloured(layout::Model::Full) => "model",
        layout::Legend::Coloured(layout::Model::Twin) => "twin",
    };
    Err(format!(
        "an {paper} {kind} holds at most {most} cells across: try a smaller --width, a bigger --format, or --chaos to go beyond"
    ))
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
    let cli = Cli::parse();
    if let Err(beyond) = within_the_limits(&cli) {
        Cli::command()
            .error(ArgumentError::ValueValidation, beyond)
            .exit();
    }
    let warnings = warnings_for(cli.width, cli.colours);
    for warning in &warnings {
        eprintln!("warning: {}", warning.says());
    }
    if !warnings.is_empty() && !cli.yes {
        if !std::io::stdin().is_terminal() {
            Cli::command()
                .error(
                    ArgumentError::MissingRequiredArgument,
                    "nobody is at a terminal to say whether to go ahead: pass --yes to go ahead without asking",
                )
                .exit();
        }
        if !sure() {
            eprintln!("Nothing written.");
            return ExitCode::FAILURE;
        }
    }
    match run(cli) {
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

/// Asks whether to go ahead, yes only when answered so.
fn sure() -> bool {
    eprint!("Are you sure? [y/N] ");
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer).is_ok()
        && matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

fn run(cli: Cli) -> Result<String, String> {
    let started = Instant::now();
    let legend = legend_of(&cli);
    let picture = read_picture(&cli.picture)?;
    let grid = GridSize::keeping_the_proportions_of(&picture, cli.width)
        .map_err(|_| "a grid needs at least one cell".to_string())?;
    let order = Order {
        grid,
        colours: cli.colours,
        format: cli.format.into(),
        orientation: if cli.landscape {
            layout::Orientation::Landscape
        } else {
            layout::Orientation::Portrait
        },
        margin: Margin(cli.margin),
        legend,
    };
    let sheet = make_sheet(&picture, order).map_err(explain)?;
    let sheet_path = cli.output.unwrap_or_else(|| beside(&cli.picture, &order));
    let bytes = png_file::write(&sheet)
        .map_err(|_| "the sheet could not be encoded as a PNG".to_string())?;
    write_sheet(&sheet_path, &bytes, cli.force)?;
    Ok(report::written(
        legend,
        &sheet_path,
        started.elapsed(),
        grid.columns() * grid.rows(),
    ))
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
    picture_file::read(&bytes)
        .map_err(|_| format!("{} is not a PNG or JPEG picture", path.display()))
}

/// Where a sheet goes when no output is given: beside its picture, named after it.
fn beside(picture: &Path, order: &Order) -> PathBuf {
    let name = picture.file_name().unwrap_or_default().to_string_lossy();
    picture.with_file_name(sheet_file_name(&name, order))
}

fn explain(no_room: NoRoom) -> String {
    match no_room {
        NoRoom::InsideTheMargin => "the margin leaves no room for the grid and its legend: try a smaller --margin",
        NoRoom::ForTheNumbersInTheCells => {
            "the cells are too small for their numbers: try fewer cells with --width, or a bigger --format"
        }
        NoRoom::ForTheNumbersInTheLegend => {
            "the bubbles are too small for their numbers: try fewer --colours, or a smaller --margin"
        }
    }
    .to_string()
}
