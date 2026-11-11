//! The command line: reads a PNG picture, makes its colour-by-number sheet and writes it as a PNG.

mod report;

use std::fs;
use std::io::{BufRead, ErrorKind, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::error::ErrorKind as ArgumentError;
use clap::{CommandFactory, Parser, ValueEnum};
use report::Written;
use trixelate::layout::{self, Margin, NoRoom, PageFormat};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::picture_file::{MOST_MEGABYTES, MOST_MEGAPIXELS, Unreadable};
use trixelate::warning::warnings_for;
use trixelate::{
    Order, Refusal, make_preview, make_sheet, picture_file, png_file, preview_file_name,
    sheet_file_name,
};

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

    /// Cells across the grid, the rows following from the picture's proportions [default: the most a sheet on the paper holds: 128 on A0, 96 on A1, 80 on A2, 64 on A3, 48 on A4, 32 on A5]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    width: Option<u32>,

    /// Colours in the palette, from 2 to 24
    #[arg(short, long, visible_alias = "colors", default_value_t = ColourCount::IN_A_SMALL_BOX, value_parser = colour_count)]
    colours: ColourCount,

    /// The paper the sheet is drawn for: A3 to A5 at 300 dots per inch, A0 to A2 posters at 150
    #[arg(short, long, value_enum, default_value_t = Format::A4)]
    format: Format,

    /// Lay the paper on its side, for wide pictures [default: upright]
    #[arg(long)]
    landscape: bool,

    /// The blank paper around the sheet, in millimetres, from 5 for the title and the credit to 20, leaving the room to the cells
    #[arg(short, long, default_value_t = Margin::AROUND_A_SHEET, value_parser = margin)]
    margin: Margin,

    /// Where to write the sheet [default: beside the picture, as <picture>-<width>-<colours>-<format>.sheet.png, with -landscape before .sheet.png when asked, and .preview.png for a preview]
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Write the sheet's preview instead: the sheet as it looks once coloured in, every cell filled with its colour
    #[arg(long)]
    preview: bool,

    /// Overwrite the sheet if it already exists
    #[arg(long)]
    force: bool,

    /// Go ahead without asking when the sheet is warned of: under 16 cells across or 4 colours
    #[arg(short, long)]
    yes: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
}

impl From<Format> for PageFormat {
    fn from(format: Format) -> PageFormat {
        match format {
            Format::A0 => PageFormat::A0,
            Format::A1 => PageFormat::A1,
            Format::A2 => PageFormat::A2,
            Format::A3 => PageFormat::A3,
            Format::A4 => PageFormat::A4,
            Format::A5 => PageFormat::A5,
        }
    }
}

/// The cells across asked for, or else the most the paper holds.
fn cells_across(cli: &Cli) -> u32 {
    cli.width
        .unwrap_or_else(|| layout::most_cells_across(cli.format.into()))
}

/// The order the arguments make, or why the library refuses it, worded for the command line.
fn ordered(cli: &Cli) -> Result<Order, String> {
    let format = PageFormat::from(cli.format);
    let orientation = if cli.landscape {
        layout::Orientation::Landscape
    } else {
        layout::Orientation::Portrait
    };
    Order::new(cells_across(cli), cli.colours, format, orientation, cli.margin).map_err(|refusal| {
        match refusal {
            Refusal::NoCellAcross => "a grid needs at least one cell across".to_string(),
            Refusal::MoreCellsAcrossThanThePaperHolds { most } => format!(
                "an {} sheet holds at most {most} cells across: try a smaller --width or a bigger --format",
                format.name()
            ),
        }
    })
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

fn margin(raw: &str) -> Result<Margin, String> {
    let millimetres: u32 = raw.parse().map_err(|_| format!("{raw} is not a number"))?;
    Margin::new(millimetres).map_err(|_| {
        format!(
            "a margin is from {} mm, for the title and the credit, to {} mm, leaving the room to the cells",
            Margin::THINNEST,
            Margin::WIDEST
        )
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let order = match ordered(&cli) {
        Ok(order) => order,
        Err(refused) => Cli::command()
            .error(ArgumentError::ValueValidation, refused)
            .exit(),
    };
    let warnings = warnings_for(order.cells_across(), order.colours());
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
    match run(cli, order) {
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

/// How the command draws what it writes, from a picture, its order and its file name.
type Making = fn(&Picture, Order, &str) -> Result<Picture, NoRoom>;
/// How the command names what it writes, from its picture's file name and its order.
type Naming = fn(&str, &Order) -> String;

fn run(cli: Cli, order: Order) -> Result<String, String> {
    let started = Instant::now();
    let picture = read_picture(&cli.picture)?;
    let grid = order.grid_for(&picture).map_err(explain)?;
    let what = if cli.preview {
        Written::Preview
    } else {
        Written::Sheet
    };
    let (make, named): (Making, Naming) = match what {
        Written::Sheet => (make_sheet, sheet_file_name),
        Written::Preview => (make_preview, preview_file_name),
    };
    let sheet_path = cli
        .output
        .unwrap_or_else(|| beside(&cli.picture, &order, named));
    let file_name = sheet_path.file_name().unwrap_or_default().to_string_lossy();
    let sheet = make(&picture, order, &file_name).map_err(explain)?;
    let bytes = png_file::write(&sheet, order.format().dots_per_inch())
        .map_err(|_| "the sheet could not be encoded as a PNG".to_string())?;
    write_sheet(&sheet_path, &bytes, cli.force)?;
    Ok(report::written(
        what,
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

/// Reads a picture from a file, refusing anything but a file, and a file too heavy for any picture, before reading it.
fn read_picture(path: &Path) -> Result<Picture, String> {
    let cannot_read = |error: std::io::Error| format!("cannot read {}: {error}", path.display());
    let too_heavy = || {
        format!(
            "{} is over {MOST_MEGABYTES} MB, more than any picture a sheet needs: shrink it first",
            path.display()
        )
    };
    let most_bytes = MOST_MEGABYTES * 1_000_000;
    let file = fs::File::open(path).map_err(cannot_read)?;
    let metadata = file.metadata().map_err(cannot_read)?;
    if !metadata.is_file() {
        return Err(format!(
            "{} is not a file: give it a PNG or JPEG picture",
            path.display()
        ));
    }
    if metadata.len() > most_bytes {
        return Err(too_heavy());
    }
    let mut bytes = Vec::new();
    file.take(most_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(cannot_read)?;
    if bytes.len() as u64 > most_bytes {
        return Err(too_heavy());
    }
    picture_file::read(&bytes).map_err(|unreadable| match unreadable {
        Unreadable::NeitherPngNorJpeg => format!("{} is not a PNG or JPEG picture", path.display()),
        Unreadable::TooBig { megapixels } => format!(
            "{} is {megapixels} megapixels, and the goblins counting its dots gave up at {MOST_MEGAPIXELS}: shrink it under {MOST_MEGAPIXELS} megapixels first",
            path.display()
        ),
    })
}

/// Where a sheet or its preview goes when no output is given: beside its picture, named after it.
fn beside(picture: &Path, order: &Order, named: Naming) -> PathBuf {
    let name = picture.file_name().unwrap_or_default().to_string_lossy();
    picture.with_file_name(named(&name, order))
}

fn explain(no_room: NoRoom) -> String {
    match no_room {
        NoRoom::ForTheNumbersInTheCells => {
            "the cells are too small for their numbers: try fewer cells with --width, or a bigger --format"
        }
    }
    .to_string()
}
