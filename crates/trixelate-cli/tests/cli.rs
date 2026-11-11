//! The command line, run as a child would see its parent run it: on real files, from the built binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use trixelate::picture::Picture;
use trixelate::png_file;

const RED: [u8; 4] = [220, 30, 40, 255];
const YELLOW: [u8; 4] = [250, 220, 60, 255];
const BLUE: [u8; 4] = [20, 60, 200, 255];

/// A fresh, empty folder for one test, under cargo's temporary directory for integration tests.
fn folder(test: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("a test folder");
    folder
}

/// A four by three picture: a red sky over a yellow sun and a blue sea.
fn a_picture_in(folder: &Path) -> PathBuf {
    let pixels = [
        RED, RED, RED, RED, //
        RED, YELLOW, YELLOW, RED, //
        BLUE, BLUE, BLUE, BLUE,
    ];
    let picture = Picture::from_rgba(4, 3, pixels.as_flattened()).expect("a picture");
    let path = folder.join("sunset.png");
    fs::write(&path, png_file::write(&picture, 300).expect("a PNG")).expect("the picture on disk");
    path
}

/// The command run as from a script that already said yes to any warning, the warnings not being what most tests are about.
fn trixelate(arguments: &[&str]) -> Output {
    trixelate_asking(&[arguments, &["--yes"]].concat())
}

/// The command run with no one at a terminal to answer it, as from a script.
fn trixelate_asking(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trixelate"))
        .args(arguments)
        .output()
        .expect("the command runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_picture_alone_becomes_an_a4_sheet_48_cells_wide_written_beside_it_and_the_command_says_where()
{
    let folder = folder("beside");
    let picture = a_picture_in(&folder);
    let explicit = folder.join("explicit");
    let forty_eight_wide = explicit.join("sunset-48-8-a4.sheet.png");
    fs::create_dir_all(&explicit).unwrap_or_default();

    let run = trixelate(&[picture.to_str().unwrap_or_default()]);
    trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "48",
        "--output",
        forty_eight_wide.to_str().unwrap_or_default(),
    ]);

    let sheet_path = folder.join("sunset-48-8-a4.sheet.png");
    let sheet = fs::read(&sheet_path)
        .ok()
        .and_then(|bytes| png_file::read(&bytes).ok());
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        sheet.map(|sheet| (sheet.width(), sheet.height())),
        Some((2480, 3508))
    );
    assert!(text(&run.stdout).contains(&sheet_path.display().to_string()));
    assert_eq!(fs::read(&sheet_path).ok(), fs::read(&forty_eight_wide).ok());
}

#[test]
fn a_jpeg_photo_becomes_a_sheet_beside_it_as_a_png() {
    let folder = folder("jpeg");
    let photo = folder.join("photo.jpg");
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../trixelate/tests/fixtures/red-and-blue.jpg"),
        &photo,
    )
    .expect("the photo on disk");

    let run = trixelate(&[photo.to_str().unwrap_or_default(), "--width", "8"]);

    let sheet = fs::read(folder.join("photo-8-8-a4.sheet.png"))
        .ok()
        .and_then(|bytes| png_file::read(&bytes).ok());
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        sheet.map(|sheet| (sheet.width(), sheet.height())),
        Some((2480, 3508))
    );
}

#[test]
fn the_command_says_what_it_wrote_where_how_fast_and_how_long_a_five_year_old_will_take_to_colour_it()
 {
    let folder = folder("report");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();

    let sheet = text(&trixelate(&[picture, "--width", "4"]).stdout);

    assert!(sheet.starts_with(&format!(
        "Sheet written to {} in ",
        folder.join("sunset-4-8-a4.sheet.png").display()
    )));
    assert!(sheet.contains(" s\n"));
    assert!(sheet.contains("12 cells to colour"));
}

#[test]
fn a_sheet_of_few_cells_or_few_colours_is_warned_of_and_written_only_once_the_user_is_sure() {
    let folder = folder("warned");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();
    let sheet = folder.join("sunset-4-2-a4.sheet.png");

    let unanswered = trixelate_asking(&[picture, "--width", "4", "--colours", "2"]);
    let written_unanswered = sheet.exists();
    let sure = trixelate_asking(&[picture, "--width", "4", "--colours", "2", "-y"]);

    let warned = text(&unanswered.stderr);
    assert_eq!(unanswered.status.code(), Some(2));
    assert!(!written_unanswered);
    assert!(warned.contains(
        "warning: with so few cells across, it will be more playing with big cubes than colour-by-number pixel art"
    ));
    assert!(warned.contains(
        "warning: with so few colours, it will be more painting solid patches than colour-by-number pixel art"
    ));
    assert!(warned.contains("--yes"));
    assert!(sure.status.success(), "{}", text(&sure.stderr));
    assert!(text(&sure.stderr).contains("warning: with so few cells across"));
    assert!(sheet.exists());
}

#[test]
fn a_landscape_sheet_lies_on_its_side() {
    let folder = folder("landscape");
    let picture = a_picture_in(&folder);

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--landscape",
    ]);

    let sheet = fs::read(folder.join("sunset-4-8-a4-landscape.sheet.png"))
        .ok()
        .and_then(|bytes| png_file::read(&bytes).ok());
    assert_eq!(
        sheet.map(|sheet| (sheet.width(), sheet.height())),
        Some((3508, 2480)),
        "{}",
        text(&run.stderr)
    );
}

/// Twelve colours in a four by three picture, so the sheet writes every digit from 0 to 9.
fn a_twelve_colour_picture_in(folder: &Path) -> PathBuf {
    let pixels: Vec<u8> = (0..12u8)
        .flat_map(|index| [index * 21, 255 - index * 21, (index % 3) * 120, 255])
        .collect();
    let picture = Picture::from_rgba(4, 3, &pixels).expect("a picture");
    let path = folder.join("twelve.png");
    fs::write(&path, png_file::write(&picture, 300).expect("a PNG")).expect("the picture on disk");
    path
}

/// Compares a sheet with its golden master, or writes the master when TRIXELATE_BLESS is set.
fn matches_golden(sheet: &Path, golden: &str) -> bool {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(golden);
    if std::env::var_os("TRIXELATE_BLESS").is_some() {
        fs::copy(sheet, &golden).expect("the golden master written");
    }
    let read = |path: &Path| {
        fs::read(path)
            .ok()
            .and_then(|bytes| png_file::read(&bytes).ok())
    };
    read(sheet).is_some() && read(sheet) == read(&golden)
}

#[test]
fn a_twelve_colour_sheet_on_a5_matches_its_golden_master_dot_for_dot_its_model_in_the_corner_and_its_title_on_top()
 {
    let folder = folder("golden");
    let picture = a_twelve_colour_picture_in(&folder);
    let sheet = folder.join("twelve.sheet.png");

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--colours",
        "12",
        "--format",
        "a5",
        "--output",
        sheet.to_str().unwrap_or_default(),
    ]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(matches_golden(&sheet, "twelve-a5.sheet.png"));
}

#[test]
fn asked_for_a_preview_the_command_writes_the_sheet_coloured_in_beside_its_picture_named_as_a_preview()
 {
    let folder = folder("preview");
    let picture = a_twelve_colour_picture_in(&folder);
    let preview = folder.join("twelve-4-12-a5.preview.png");

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--colours",
        "12",
        "--format",
        "a5",
        "--preview",
    ]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains(&preview.display().to_string()));
    assert!(matches_golden(&preview, "twelve-a5.preview.png"));
}

#[test]
fn an_existing_sheet_is_kept_unless_the_command_is_told_to_overwrite_it() {
    let folder = folder("overwrite");
    let picture = a_picture_in(&folder);
    let sheet = folder.join("sunset-4-8-a4.sheet.png");
    fs::write(&sheet, "a sheet already coloured in").expect("an existing sheet");
    let arguments = [picture.to_str().unwrap_or_default(), "--width", "4"];

    let kept = trixelate(&arguments);
    let kept_sheet = fs::read_to_string(&sheet).ok();
    let overwritten = trixelate(&[arguments.as_slice(), &["--force"]].concat());

    assert_eq!(
        (kept.status.code(), kept_sheet.as_deref()),
        (Some(1), Some("a sheet already coloured in"))
    );
    assert!(text(&kept.stderr).contains("--force"));
    assert!(overwritten.status.success());
    assert!(
        fs::read(&sheet)
            .ok()
            .and_then(|bytes| png_file::read(&bytes).ok())
            .is_some()
    );
}

#[test]
fn forcing_an_overwrite_is_refused_with_an_output_so_a_typo_never_replaces_the_picture_with_its_sheet()
 {
    let folder = folder("force-output");
    let picture = a_picture_in(&folder);
    let original = fs::read(&picture).expect("the picture");
    let picture = picture.to_str().unwrap_or_default();

    let run = trixelate(&[picture, "--width", "4", "-o", picture, "--force"]);

    assert_eq!(run.status.code(), Some(2));
    assert!(
        text(&run.stderr).contains("--force"),
        "{}",
        text(&run.stderr)
    );
    assert_eq!(fs::read(picture).ok(), Some(original));
}

#[test]
fn a_sheet_that_cannot_be_made_fails_saying_why_and_what_to_try() {
    let folder = folder("failures");
    let not_a_png = folder.join("drawing.txt");
    fs::write(&not_a_png, "a drawing described in words").expect("a text file");
    let a_ribbon = folder.join("ribbon.png");
    let ribbon = Picture::from_rgba(1, 100, &RED.repeat(100)).expect("a picture");
    fs::write(&a_ribbon, png_file::write(&ribbon, 300).expect("a PNG"))
        .expect("the picture on disk");
    let cases: [(&[&str], &str); 3] = [
        (&["missing.png", "--width", "4"], "cannot read missing.png"),
        (
            &[not_a_png.to_str().unwrap_or_default(), "--width", "4"],
            "is not a PNG or JPEG picture",
        ),
        (
            &[a_ribbon.to_str().unwrap_or_default(), "--width", "48"],
            "try fewer cells with --width",
        ),
    ];

    for (arguments, hint) in cases {
        let run = trixelate(arguments);

        assert_eq!(run.status.code(), Some(1), "{arguments:?}");
        assert!(
            text(&run.stderr)
                .lines()
                .last()
                .is_some_and(|last| last.starts_with("error: ")),
            "{arguments:?}"
        );
        assert!(
            text(&run.stderr).contains(hint),
            "{arguments:?}: {}",
            text(&run.stderr)
        );
    }
}

#[test]
fn a_folder_or_a_file_over_a_hundred_megabytes_is_refused_before_it_is_read() {
    let folder = folder("not-read");
    let heavy = folder.join("heavy.png");
    fs::File::create(&heavy)
        .and_then(|file| file.set_len(100_000_001))
        .expect("a sparse file just over 100 MB");
    let cases = [
        (folder.to_str().unwrap_or_default(), "is not a file"),
        (heavy.to_str().unwrap_or_default(), "is over 100 MB"),
    ];

    for (picture, hint) in cases {
        let run = trixelate(&[picture, "--width", "4"]);

        assert_eq!(run.status.code(), Some(1), "{picture}");
        assert!(text(&run.stderr).contains(hint), "{}", text(&run.stderr));
    }
}

/// The dots per metre a PNG tells printers, read from its pHYs chunk.
fn dots_per_metre(png: &[u8]) -> Option<u32> {
    let chunk = png.windows(4).position(|name| name == b"pHYs")?;
    let across = png.get(chunk + 4..chunk + 8)?;
    Some(u32::from_be_bytes(across.try_into().ok()?))
}

#[test]
fn a_poster_on_a2_takes_80_cells_across_on_a_page_printed_at_150_dots_per_inch() {
    let folder = folder("poster");
    let picture = a_picture_in(&folder);
    let poster = folder.join("sunset-80-8-a2.sheet.png");

    let run = trixelate(&[picture.to_str().unwrap_or_default(), "--format", "a2"]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    let bytes = fs::read(&poster).unwrap_or_default();
    let page = png_file::read(&bytes).map(|sheet| (sheet.width(), sheet.height()));
    assert_eq!(page.ok(), Some((2480, 3508)));
    assert_eq!(dots_per_metre(&bytes), Some(5906));
}

#[test]
fn without_a_width_the_grid_takes_the_most_cells_across_its_paper_holds() {
    let folder = folder("default-width");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();

    let on_a4 = trixelate(&[picture]);
    let on_a5 = trixelate(&[picture, "--format", "a5"]);

    assert!(on_a4.status.success(), "{}", text(&on_a4.stderr));
    assert!(on_a5.status.success(), "{}", text(&on_a5.stderr));
    assert!(folder.join("sunset-48-8-a4.sheet.png").exists());
    assert!(folder.join("sunset-32-8-a5.sheet.png").exists());
}

#[test]
fn more_cells_across_than_the_paper_holds_are_refused_as_misuse() {
    let folder = folder("beyond");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();

    let on_a4 = trixelate(&[picture, "--width", "49"]);
    let on_a5 = trixelate(&[picture, "--width", "33", "--format", "a5"]);

    assert_eq!(
        [&on_a4, &on_a5].map(|run| run.status.code()),
        [Some(2), Some(2)]
    );
    assert!(text(&on_a4.stderr).contains("an A4 sheet holds at most 48 cells across"));
    assert!(text(&on_a5.stderr).contains("an A5 sheet holds at most 32 cells across"));
}

#[test]
fn wrong_arguments_are_refused_as_misuse_before_any_picture_is_read() {
    let cases: [&[&str]; 12] = [
        &["missing.png", "--width", "0"],
        &["missing.png", "--width", "4", "--colours", "25"],
        &["--width", "4"],
        &["missing.png", "--width", "4", "--height", "3"],
        &["missing.png", "--width", "4", "--with-model"],
        &["missing.png", "--width", "4", "--model"],
        &["missing.png", "--width", "4", "--twin"],
        &["missing.png", "--width", "4", "--mini", "1"],
        &["missing.png", "--width", "4", "--freedom"],
        &["missing.png", "--width", "4", "--margin", "4"],
        &["missing.png", "--width", "4", "--margin", "21"],
        &["missing.png", "--width", "4", "--chaos"],
    ];

    for arguments in cases {
        let run = trixelate(arguments);

        assert_eq!(run.status.code(), Some(2), "{arguments:?}");
        assert!(!text(&run.stderr).contains("cannot read"), "{arguments:?}");
    }
}
