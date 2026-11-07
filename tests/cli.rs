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
fn a_picture_becomes_an_a4_sheet_written_beside_it_and_the_command_says_where() {
    let folder = folder("beside");
    let picture = a_picture_in(&folder);

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--height",
        "3",
        "--colours",
        "3",
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
}
