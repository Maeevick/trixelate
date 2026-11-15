//! The command line: reads a PNG or JPEG picture, makes its colour-by-number sheet and writes it as a PDF, its preview
//! as a PDF too or its pieces as a PDF.

mod answer;
mod report;

use std::fs;
use std::io::{BufRead, ErrorKind, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Instant, SystemTime};

use answer::said_yes;
use clap::error::ErrorKind as ArgumentError;
use clap::{CommandFactory, Parser, ValueEnum};
use report::{Written, shown};
use trixelate::layout::{self, Margin, NoRoom, Orientation, PageFormat};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::picture_file::{MOST_MEGABYTES, MOST_MEGAPIXELS, Unreadable};
use trixelate::warning::warnings_for;
use trixelate::{
    Order, Refusal, colour, picture_file, pieces_file_name, pieces_pdf, preview_file_name,
    preview_pdf, sheet_file_name, sheet_pdf, stamped,
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

    /// Cells across the grid, the rows following from the picture's proportions, at most 80 on A2, 64 on A3, 48 on A4 and 32 on A5 [default: 64 on A2, 48 on A3, 32 on A4, 24 on A5]
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    width: Option<u32>,

    /// Colours in the palette, from 2 to 24 [default: 8 on A2 and A3, 4 on A4 and A5]
    #[arg(short, long, visible_alias = "colors", value_parser = colour_count)]
    colours: Option<ColourCount>,

    /// The paper the sheet is drawn for, at 300 dots per inch: A3 to A5, or an A2 poster
    #[arg(short, long, value_enum, default_value_t = Format::A4)]
    format: Format,

    /// Lay the paper on its side [default: the way the picture suits, on its side once it is a tenth wider than tall]
    #[arg(long)]
    landscape: bool,

    /// Stand the paper upright [default: the way the picture suits, upright unless it is a tenth wider than tall]
    #[arg(long, conflicts_with = "landscape")]
    portrait: bool,

    /// The blank paper around the sheet, in millimetres, from 5 for the title and the credit to 20, leaving the room to the cells
    #[arg(short, long, default_value_t = Margin::AROUND_A_SHEET, value_parser = margin)]
    margin: Margin,

    /// Where to write the sheet: a new file, or a folder to write it in under its default name [default: beside the picture, as <picture>-<width>-<colours>-<format>-<UTC second>.sheet.pdf, with -landscape after the format when asked, .preview.pdf for a preview and .pieces.pdf for pieces]
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Write the sheet's preview instead: the sheet as it looks once coloured in, every cell filled with its colour
    #[arg(long)]
    preview: bool,

    /// Write the sheet cut into A4 pieces instead, for a home printer, as one PDF to print at actual size: each piece marked where to cut it and saying its row and column, the next glued over the 10 mm they share
    #[arg(long, conflicts_with = "preview")]
    pieces: bool,

    /// Go ahead without asking when the sheet is warned of: under 16 cells across or 4 colours
    #[arg(short, long)]
    yes: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    A2,
    A3,
    A4,
    A5,
}

impl From<Format> for PageFormat {
    fn from(format: Format) -> PageFormat {
        match format {
            Format::A2 => PageFormat::A2,
            Format::A3 => PageFormat::A3,
            Format::A4 => PageFormat::A4,
            Format::A5 => PageFormat::A5,
        }
    }
}

/// The cells across asked for, or else the paper's usual.
fn cells_across(cli: &Cli) -> u32 {
    cli.width
        .unwrap_or_else(|| layout::usual_cells_across(cli.format.into()))
}

/// The colours asked for, or else the paper's usual.
fn colours(cli: &Cli) -> ColourCount {
    cli.colours
        .unwrap_or_else(|| layout::usual_colours(cli.format.into()))
}

/// The way up asked for, if any.
fn asked_way_up(cli: &Cli) -> Option<Orientation> {
    match (cli.landscape, cli.portrait) {
        (true, _) => Some(Orientation::Landscape),
        (_, true) => Some(Orientation::Portrait),
        _ => None,
    }
}

/// The order the arguments make, upright until the picture is read unless asked otherwise, or why the library refuses
/// it, worded for the command line.
fn ordered(cli: &Cli) -> Result<Order, String> {
    let format = PageFormat::from(cli.format);
    let orientation = asked_way_up(cli).unwrap_or(Orientation::Portrait);
    let order = Order::new(cells_across(cli), colours(cli), format, orientation, cli.margin)
        .map_err(|refusal| match refusal {
            Refusal::NoCellAcross => "a grid needs at least one cell across".to_string(),
            Refusal::MoreCellsAcrossThanThePaperHolds { most } => format!(
                "an {} sheet holds at most {most} cells across: try a smaller --width or a bigger --format",
                format.name()
            ),
        })?;
    if cli.pieces && order.pieces().is_none() {
        return Err(format!(
            "an {} sheet prints whole at home: --pieces is for an A3 or A2 poster",
            format.name()
        ));
    }
    Ok(order)
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
    match run(cli, order, SystemTime::now()) {
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
    std::io::stdin().lock().read_line(&mut answer).is_ok() && said_yes(&answer)
}

/// What the command is asked to write: the sheet, its preview or its pieces.
#[derive(Clone, Copy)]
enum Asked {
    Sheet,
    Preview,
    Pieces,
}

/// How the command names what it writes, from its picture's file name and its order.
type Naming = fn(&str, &Order) -> String;

fn run(cli: Cli, order: Order, now: SystemTime) -> Result<String, String> {
    let started = Instant::now();
    let picture = read_picture(&cli.picture)?;
    let order = order.lying(asked_way_up(&cli).unwrap_or_else(|| Orientation::suiting(&picture)));
    let grid = order.grid_for(&picture).map_err(explain)?;
    let asked = match (cli.preview, cli.pieces) {
        (true, _) => Asked::Preview,
        (_, true) => Asked::Pieces,
        _ => Asked::Sheet,
    };
    let named: Naming = match asked {
        Asked::Sheet => sheet_file_name,
        Asked::Preview => preview_file_name,
        Asked::Pieces => pieces_file_name,
    };
    let default_title = named(
        &cli.picture
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        &order,
    );
    let seconds = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let (claimed, title) = match cli.output {
        Some(folder) if folder.is_dir() || folder.file_name().is_none() => (
            claim_stamped(|name| folder.join(name), &default_title, seconds)?,
            default_title,
        ),
        Some(file) => {
            let title = file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (claim_named(file)?, title)
        }
        None => (
            claim_stamped(
                |name| cli.picture.with_file_name(name),
                &default_title,
                seconds,
            )?,
            default_title,
        ),
    };
    let colouring = colour(&picture, order).map_err(explain)?;
    let (what, bytes) = match asked {
        Asked::Sheet => (
            Written::Sheet,
            sheet_pdf(&colouring, order, &title, seconds).map_err(explain)?,
        ),
        Asked::Preview => (
            Written::Preview,
            preview_pdf(&colouring, order, &title, seconds).map_err(explain)?,
        ),
        Asked::Pieces => {
            let whole = || format!("an {} sheet prints whole at home", order.format().name());
            let pieces = order.pieces().ok_or_else(whole)?;
            let pdf = pieces_pdf(&colouring, order, &title, seconds)
                .map_err(explain)?
                .ok_or_else(whole)?;
            let (across, down) = (pieces.across(), pieces.down());
            (Written::Pieces { across, down }, pdf)
        }
    };
    let sheet_path = claimed.write(&bytes)?;
    Ok(report::written(
        what,
        &sheet_path,
        started.elapsed(),
        grid.columns() * grid.rows(),
    ))
}

/// How many sheets of one picture, with the same settings, can be written in one second before trixelate gives up.
const MOST_SHEETS_IN_A_SECOND: u32 = 100;

/// A new file claimed for the sheet before it is made, so a name already taken costs nothing, and removed again unless
/// the sheet is written into it in full, so a failure never leaves an empty or broken sheet behind.
struct Claimed {
    path: PathBuf,
    file: fs::File,
    written: bool,
}

impl Claimed {
    /// A new file at a path, never one already there, nor a link to another.
    fn new(path: PathBuf) -> std::io::Result<Claimed> {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(Claimed {
            path,
            file,
            written: false,
        })
    }

    /// Writes the sheet into its file in full, and says where it went.
    fn write(mut self, bytes: &[u8]) -> Result<PathBuf, String> {
        self.file
            .write_all(bytes)
            .map_err(|error| cannot_write(&self.path, error))?;
        self.written = true;
        Ok(self.path.clone())
    }
}

impl Drop for Claimed {
    fn drop(&mut self) {
        if !self.written {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// The file named as the output, refused when it already exists.
fn claim_named(path: PathBuf) -> Result<Claimed, String> {
    Claimed::new(path.clone()).map_err(|error| match error.kind() {
        ErrorKind::AlreadyExists => format!(
            "{} already exists, and trixelate never overwrites a file: choose another --output",
            shown(&path)
        ),
        _ => cannot_write(&path, error),
    })
}

/// A file trixelate names itself, its title stamped with the second it was made, numbered after the stamp when sheets
/// made that second already took the name.
fn claim_stamped(
    place: impl Fn(&str) -> PathBuf,
    title: &str,
    seconds: u64,
) -> Result<Claimed, String> {
    for made_before in 0..MOST_SHEETS_IN_A_SECOND {
        let path = place(&stamped(title, seconds, made_before));
        match Claimed::new(path.clone()) {
            Ok(claimed) => return Ok(claimed),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(cannot_write(&path, error)),
        }
    }
    Err(format!(
        "{MOST_SHEETS_IN_A_SECOND} sheets named {} already exist from that second, and trixelate never overwrites a file: try again in a second",
        shown(&place(&stamped(title, seconds, 0)))
    ))
}

fn cannot_write(path: &Path, error: std::io::Error) -> String {
    format!("cannot write the sheet to {}: {error}", shown(path))
}

/// Reads a picture from a file, refusing anything but a file, and a file too heavy for any picture, before reading it.
fn read_picture(path: &Path) -> Result<Picture, String> {
    let cannot_read = |error: std::io::Error| format!("cannot read {}: {error}", shown(path));
    let too_heavy = || {
        format!(
            "{} is over {MOST_MEGABYTES} MB, more than any picture a sheet needs: shrink it first",
            shown(path)
        )
    };
    let most_bytes = MOST_MEGABYTES * 1_000_000;
    let file = fs::File::open(path).map_err(cannot_read)?;
    let metadata = file.metadata().map_err(cannot_read)?;
    if !metadata.is_file() {
        return Err(format!(
            "{} is not a file: give it a PNG or JPEG picture",
            shown(path)
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
        Unreadable::NeitherPngNorJpeg => format!("{} is not a PNG or JPEG picture", shown(path)),
        Unreadable::TooBig { megapixels } => format!(
            "{} is {megapixels} megapixels, and the goblins counting its dots gave up at {MOST_MEGAPIXELS}: shrink it under {MOST_MEGAPIXELS} megapixels first",
            shown(path)
        ),
    })
}

fn explain(no_room: NoRoom) -> String {
    match no_room {
        NoRoom::ForTheNumbersInTheCells => {
            "the cells are too small for their numbers: try fewer cells with --width, or a bigger --format"
        }
    }
    .to_string()
}
