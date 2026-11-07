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
    fs::write(&path, png_file::write(&picture).expect("a PNG")).expect("the picture on disk");
    path
}

fn trixelate(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trixelate"))
        .args(arguments)
        .output()
        .expect("trixelate runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_picture_alone_becomes_an_a4_sheet_48_cells_wide_written_beside_it_and_the_command_says_where()
{
    let folder = folder("beside");
    let picture = a_picture_in(&folder);
    let forty_eight_wide = folder.join("forty-eight.png");

    let run = trixelate(&[picture.to_str().unwrap_or_default()]);
    trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "48",
        "--output",
        forty_eight_wide.to_str().unwrap_or_default(),
    ]);

    let sheet_path = folder.join("sunset.sheet.png");
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
fn a_full_model_is_written_beside_the_picture_apart_from_its_sheet() {
    let folder = folder("model");
    let picture = a_picture_in(&folder);

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--model",
    ]);

    let model_path = folder.join("sunset.model.png");
    let model = fs::read(&model_path)
        .ok()
        .and_then(|bytes| png_file::read(&bytes).ok());
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        model.map(|model| (model.width(), model.height())),
        Some((2480, 3508))
    );
    assert!(text(&run.stdout).contains(&model_path.display().to_string()));
    assert!(!folder.join("sunset.sheet.png").exists());
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

    let sheet = fs::read(folder.join("sunset.sheet.png"))
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
    fs::write(&path, png_file::write(&picture).expect("a PNG")).expect("the picture on disk");
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
fn a_twelve_colour_sheet_on_a5_matches_its_golden_master_dot_for_dot() {
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
fn a_free_legend_sheet_matches_its_golden_master_its_bubbles_left_blank() {
    let folder = folder("freedom");
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
        "--freedom",
    ]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(matches_golden(&sheet, "twelve-a5-freedom.sheet.png"));
}

#[test]
fn a_landscape_twin_beside_its_picture_matches_its_golden_master_model_left_sheet_right() {
    let folder = folder("twin");
    let picture = a_twelve_colour_picture_in(&folder);

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--colours",
        "12",
        "--format",
        "a5",
        "--landscape",
        "--twin",
    ]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(matches_golden(
        &folder.join("twelve.twin.png"),
        "twelve-a5-landscape.twin.png"
    ));
}

#[test]
fn an_existing_sheet_is_kept_unless_the_command_is_told_to_overwrite_it() {
    let folder = folder("overwrite");
    let picture = a_picture_in(&folder);
    let sheet = folder.join("sunset.sheet.png");
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
fn a_sheet_that_cannot_be_made_fails_saying_why_and_what_to_try() {
    let folder = folder("failures");
    let picture = a_picture_in(&folder);
    let twelve_colours = a_twelve_colour_picture_in(&folder);
    let not_a_png = folder.join("drawing.txt");
    fs::write(&not_a_png, "a drawing described in words").expect("a text file");
    let picture = picture.to_str().unwrap_or_default();
    let cases: [(&[&str], &str); 5] = [
        (&["missing.png", "--width", "4"], "cannot read missing.png"),
        (
            &[not_a_png.to_str().unwrap_or_default(), "--width", "4"],
            "is not a PNG picture",
        ),
        (
            &[picture, "--width", "4", "--margin", "105"],
            "try a smaller --margin",
        ),
        (&[picture, "--width", "900"], "try fewer cells with --width"),
        (
            &[
                twelve_colours.to_str().unwrap_or_default(),
                "--width",
                "4",
                "--colours",
                "12",
                "--margin",
                "65",
                "--format",
                "a5",
            ],
            "try fewer --colours",
        ),
    ];

    for (arguments, hint) in cases {
        let run = trixelate(arguments);

        assert_eq!(run.status.code(), Some(1), "{arguments:?}");
        assert!(text(&run.stderr).starts_with("error: "), "{arguments:?}");
        assert!(
            text(&run.stderr).contains(hint),
            "{arguments:?}: {}",
            text(&run.stderr)
        );
    }
}

#[test]
fn wrong_arguments_are_refused_as_misuse_before_any_picture_is_read() {
    let cases: [&[&str]; 7] = [
        &["missing.png", "--width", "0"],
        &["missing.png", "--width", "4", "--colours", "13"],
        &["missing.png"],
        &["missing.png", "--width", "4", "--height", "3"],
        &["missing.png", "--width", "4", "--freedom", "--model"],
        &["missing.png", "--width", "4", "--freedom", "--twin"],
        &["missing.png", "--width", "4", "--model", "--twin"],
    ];

    for arguments in cases {
        let run = trixelate(arguments);

        assert_eq!(run.status.code(), Some(2), "{arguments:?}");
        assert!(!text(&run.stderr).contains("cannot read"), "{arguments:?}");
    }
}
