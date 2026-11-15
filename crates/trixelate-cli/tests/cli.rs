//! The command line, run as a child would see its parent run it: on real files, from the built binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use trixelate::picture::Picture;
use trixelate::png_file;
use trixelate::stamped;

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

/// The command run as from a script that already said yes to any warning and, unless asked on its side, upright, the
/// warnings and the way up not being what most tests are about.
fn trixelate(arguments: &[&str]) -> Output {
    let way_up: &[&str] = if arguments.contains(&"--landscape") {
        &[]
    } else {
        &["--portrait"]
    };
    trixelate_asking(&[arguments, &["--yes"], way_up].concat())
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

/// The file a run wrote beside its picture under its default name, whatever UTC second it was stamped with, then what it
/// is and its extension.
fn written_beside(folder: &Path, name: &str, what_and_its_extension: &str) -> Option<PathBuf> {
    let stamped = |file: &str| {
        file.strip_prefix(&format!("{name}-"))
            .and_then(|rest| rest.strip_suffix(&format!(".{what_and_its_extension}")))
            .is_some_and(|stamp| {
                stamp.len() == 16
                    && stamp.ends_with('Z')
                    && stamp.char_indices().all(|(at, character)| match at {
                        8 => character == 'T',
                        15 => true,
                        _ => character.is_ascii_digit(),
                    })
            })
    };
    fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|file| file.to_str())
                .is_some_and(stamped)
        })
}

#[test]
fn a_sheet_written_beside_its_picture_is_stamped_with_the_utc_second_it_was_made() {
    let folder = folder("stamped");
    let picture = a_picture_in(&folder);

    let run = trixelate(&[picture.to_str().unwrap_or_default(), "--width", "4"]);

    let sheet = written_beside(&folder, "sunset-4-4-a4", "sheet.pdf");
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(
        sheet
            .as_ref()
            .is_some_and(|sheet| text(&run.stdout).contains(&sheet.display().to_string())),
        "{}",
        text(&run.stdout)
    );
}

#[test]
fn a_stamped_sheet_beside_its_picture_or_in_a_folder_is_titled_without_its_stamp_the_same_sheet_as_one_named_plainly()
 {
    let folder = folder("titled");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();
    let (inside, plain) = (folder.join("inside"), folder.join("plain"));
    fs::create_dir_all(&inside).unwrap_or_default();
    fs::create_dir_all(&plain).unwrap_or_default();
    let named_plainly = plain.join("sunset-4-4-a4.sheet.pdf");

    trixelate(&[picture, "--width", "4"]);
    trixelate(&[
        picture,
        "--width",
        "4",
        "-o",
        inside.to_str().unwrap_or_default(),
    ]);
    trixelate(&[
        picture,
        "--width",
        "4",
        "-o",
        named_plainly.to_str().unwrap_or_default(),
    ]);

    let sheet_in = |folder: &Path| {
        written_beside(folder, "sunset-4-4-a4", "sheet.pdf")
            .and_then(|path| fs::read(path).ok())
            .map(undated)
    };
    let plainly = fs::read(&named_plainly).ok().map(undated);
    assert!(plainly.is_some());
    assert!(
        sheet_in(&folder) == plainly,
        "the sheet beside its picture differs from the one named plainly"
    );
    assert!(
        sheet_in(&inside) == plainly,
        "the sheet in a folder differs from the one named plainly"
    );
}

#[test]
fn a_picture_alone_becomes_an_a4_sheet_in_a_pdf_written_beside_it_and_the_command_says_where() {
    let folder = folder("beside");
    let picture = a_picture_in(&folder);

    let run = trixelate(&[picture.to_str().unwrap_or_default()]);

    let sheet_path = written_beside(&folder, "sunset-32-4-a4", "sheet.pdf").unwrap_or_default();
    let sheet = fs::read(&sheet_path).ok();
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        sheet.as_deref().and_then(page_size).as_deref(),
        Some(A4_UPRIGHT)
    );
    assert!(text(&run.stdout).contains(&sheet_path.display().to_string()));
}

#[test]
fn a_jpeg_photo_becomes_a_sheet_beside_it_as_a_pdf() {
    let folder = folder("jpeg");
    let photo = folder.join("photo.jpg");
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../trixelate/tests/fixtures/red-and-blue.jpg"),
        &photo,
    )
    .expect("the photo on disk");

    let run = trixelate(&[photo.to_str().unwrap_or_default(), "--width", "8"]);

    let sheet =
        written_beside(&folder, "photo-8-4-a4", "sheet.pdf").and_then(|path| fs::read(path).ok());
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert_eq!(
        sheet.as_deref().and_then(page_size).as_deref(),
        Some(A4_UPRIGHT)
    );
}

#[test]
fn the_command_says_what_it_wrote_where_how_fast_and_how_long_a_five_year_old_will_take_to_colour_it()
 {
    let folder = folder("report");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();

    let sheet = text(&trixelate(&[picture, "--width", "4"]).stdout);

    let written = written_beside(&folder, "sunset-4-4-a4", "sheet.pdf").unwrap_or_default();
    assert!(sheet.starts_with(&format!("Sheet written to {} in ", written.display())));
    assert!(sheet.contains(" s\n"));
    assert!(sheet.contains("12 cells to colour"));
}

#[test]
fn a_sheet_of_few_cells_or_few_colours_is_warned_of_and_written_only_once_the_user_is_sure() {
    let folder = folder("warned");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();
    let written = || written_beside(&folder, "sunset-4-2-a4-landscape", "sheet.pdf").is_some();

    let unanswered = trixelate_asking(&[picture, "--width", "4", "--colours", "2"]);
    let written_unanswered = written();
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
    assert!(written());
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

    let sheet = written_beside(&folder, "sunset-4-4-a4-landscape", "sheet.pdf")
        .and_then(|path| fs::read(path).ok());
    assert_eq!(
        sheet.as_deref().and_then(page_size).as_deref(),
        Some(A4_ON_ITS_SIDE),
        "{}",
        text(&run.stderr)
    );
}

#[test]
fn a_sheet_lies_the_way_its_picture_suits_unless_asked_upright() {
    let folder = folder("suiting");
    let wide = a_picture_in(&folder);
    let wide = wide.to_str().unwrap_or_default();

    let suiting = trixelate_asking(&[wide, "--width", "4", "--yes"]);
    let upright = trixelate_asking(&[wide, "--width", "4", "--yes", "--portrait"]);

    let page = |name| {
        written_beside(&folder, name, "sheet.pdf")
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| page_size(&bytes))
    };
    assert!(suiting.status.success(), "{}", text(&suiting.stderr));
    assert!(upright.status.success(), "{}", text(&upright.stderr));
    assert_eq!(
        [page("sunset-4-4-a4-landscape"), page("sunset-4-4-a4")],
        [
            Some(A4_ON_ITS_SIDE.to_string()),
            Some(A4_UPRIGHT.to_string())
        ]
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

/// Compares a sheet with its golden master, a PDF byte for byte but for the moment it was made and a PNG dot for dot,
/// or writes the master when TRIXELATE_BLESS is set.
fn matches_golden(sheet: &Path, golden: &str) -> bool {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(golden);
    if std::env::var_os("TRIXELATE_BLESS").is_some() {
        fs::copy(sheet, &golden).expect("the golden master written");
    }
    if golden
        .extension()
        .is_some_and(|extension| extension == "pdf")
    {
        let read = |path: &Path| fs::read(path).ok().map(undated);
        return read(sheet).is_some() && read(sheet) == read(&golden);
    }
    let read = |path: &Path| {
        fs::read(path)
            .ok()
            .and_then(|bytes| png_file::read(&bytes).ok())
    };
    read(sheet).is_some() && read(sheet) == read(&golden)
}

/// A PDF without the moment it was made, for two made at different moments to compare.
fn undated(pdf: Vec<u8>) -> Vec<u8> {
    let text = String::from_utf8_lossy(&pdf).into_owned();
    match text.split_once("/CreationDate (D:") {
        Some((before, after)) => {
            let (_, after) = after.split_once(')').unwrap_or_default();
            format!("{before}{after}").into_bytes()
        }
        None => pdf,
    }
}

#[test]
fn a_twelve_colour_sheet_on_a5_matches_its_golden_master_dot_for_dot_its_miniature_in_the_corner_and_its_title_on_top()
 {
    let folder = folder("golden");
    let picture = a_twelve_colour_picture_in(&folder);
    let sheet = folder.join("twelve.sheet.pdf");

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
    assert!(matches_golden(&sheet, "twelve-a5.sheet.pdf"));
}

#[test]
fn a_twelve_colour_sheet_on_a5_on_its_side_matches_its_golden_master_dot_for_dot_its_legend_along_the_bottom()
 {
    let folder = folder("golden-landscape");
    let picture = a_twelve_colour_picture_in(&folder);
    let sheet = folder.join("twelve-landscape.sheet.pdf");

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--colours",
        "12",
        "--format",
        "a5",
        "--landscape",
        "--output",
        sheet.to_str().unwrap_or_default(),
    ]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(matches_golden(&sheet, "twelve-a5-landscape.sheet.pdf"));
}

#[test]
fn asked_for_a_preview_the_command_writes_the_sheet_coloured_in_beside_its_picture_named_as_a_preview()
 {
    let folder = folder("preview");
    let picture = a_twelve_colour_picture_in(&folder);
    let titled_as_its_master = folder.join("twelve-4-12-a5.preview.png");
    let arguments = [
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "--colours",
        "12",
        "--format",
        "a5",
        "--preview",
    ];

    let beside = trixelate(&arguments);
    let output = titled_as_its_master.to_str().unwrap_or_default();
    let named = trixelate(&[arguments.as_slice(), &["--output", output]].concat());

    let preview = written_beside(&folder, "twelve-4-12-a5", "preview.png").unwrap_or_default();
    assert!(beside.status.success(), "{}", text(&beside.stderr));
    assert!(text(&beside.stdout).contains(&preview.display().to_string()));
    assert!(named.status.success(), "{}", text(&named.stderr));
    assert!(matches_golden(
        &titled_as_its_master,
        "twelve-a5.preview.png"
    ));
}

#[test]
fn an_output_naming_a_folder_or_no_file_at_all_gets_the_sheet_under_its_stamped_name_inside_even_in_a_missing_folder()
 {
    let folder = folder("output-folder");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();
    let inside = folder.join("sheets");
    fs::create_dir_all(&inside).expect("a folder for sheets");
    let up_from_inside = inside.join("..");
    let up_from_nowhere = folder.join("nowhere").join("..");

    let into_a_folder = trixelate(&[
        picture,
        "--width",
        "4",
        "-o",
        inside.to_str().unwrap_or_default(),
    ]);
    let into_no_file = trixelate(&[
        picture,
        "--width",
        "5",
        "-o",
        up_from_inside.to_str().unwrap_or_default(),
    ]);
    let into_no_file_nowhere = trixelate(&[
        picture,
        "--width",
        "6",
        "-o",
        up_from_nowhere.to_str().unwrap_or_default(),
    ]);

    assert!(
        into_a_folder.status.success(),
        "{}",
        text(&into_a_folder.stderr)
    );
    assert!(
        into_no_file.status.success(),
        "{}",
        text(&into_no_file.stderr)
    );
    assert!(written_beside(&inside, "sunset-4-4-a4", "sheet.pdf").is_some());
    assert!(written_beside(&folder, "sunset-5-4-a4", "sheet.pdf").is_some());
    assert!(
        text(&into_no_file_nowhere.stderr).contains("nowhere/../sunset-6-4-a4-"),
        "{}",
        text(&into_no_file_nowhere.stderr)
    );
}

#[test]
fn a_sheet_made_in_a_second_already_taken_is_written_under_the_next_number_overwriting_nothing() {
    let folder = folder("second-taken");
    let picture = a_picture_in(&folder);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let taken: Vec<PathBuf> = (now - 1..now + 30)
        .map(|second| folder.join(stamped("sunset-4-4-a4.sheet.pdf", second, 0)))
        .collect();
    for path in &taken {
        fs::write(path, "a sheet made earlier that second").expect("a sheet already there");
    }

    let run = trixelate(&[picture.to_str().unwrap_or_default(), "--width", "4"]);

    let numbered = fs::read_dir(&folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.ends_with("Z-2.sheet.pdf"));
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(numbered.is_some_and(|name| text(&run.stdout).contains(&name)));
    assert!(taken.iter().all(|path| {
        fs::read(path).ok().as_deref() == Some(b"a sheet made earlier that second".as_slice())
    }));
}

#[test]
fn a_folder_the_sheet_cannot_be_written_in_is_said_so_rather_than_taken_for_a_full_second() {
    use std::os::unix::fs::PermissionsExt;
    let folder = folder("locked");
    let picture = a_picture_in(&folder);
    let locked = folder.join("locked");
    fs::create_dir_all(&locked).expect("a folder to lock");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("the folder locked");

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--width",
        "4",
        "-o",
        locked.to_str().unwrap_or_default(),
    ]);

    let said = text(&run.stderr);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("the folder unlocked");
    assert!(!run.status.success());
    assert!(said.contains("cannot write the sheet to"), "{said}");
    assert!(said.contains("Permission denied"), "{said}");
}

#[test]
fn an_existing_file_is_never_overwritten_not_even_the_picture_and_there_is_no_force_to_ask_for() {
    let folder = folder("never-overwritten");
    let picture = a_picture_in(&folder);
    let original = fs::read(&picture).expect("the picture");
    let existing = folder.join("coloured-in.png");
    fs::write(&existing, "a sheet already coloured in").expect("an existing sheet");
    let picture = picture.to_str().unwrap_or_default();

    let onto_a_sheet = trixelate(&[
        picture,
        "--width",
        "4",
        "-o",
        existing.to_str().unwrap_or_default(),
    ]);
    let onto_the_picture = trixelate(&[picture, "--width", "4", "-o", picture]);
    let forced = trixelate(&[picture, "--width", "4", "--force"]);

    assert_eq!(onto_a_sheet.status.code(), Some(1));
    assert!(text(&onto_a_sheet.stderr).contains("already exists"));
    assert_eq!(
        fs::read_to_string(&existing).ok().as_deref(),
        Some("a sheet already coloured in")
    );
    assert_eq!(onto_the_picture.status.code(), Some(1));
    assert_eq!(fs::read(picture).ok(), Some(original));
    assert_eq!(forced.status.code(), Some(2));
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
    let (heavy, just_light_enough) = (folder.join("heavy.png"), folder.join("light.png"));
    for (path, bytes) in [(&heavy, 100_000_001), (&just_light_enough, 100_000_000)] {
        fs::File::create(path)
            .and_then(|file| file.set_len(bytes))
            .expect("a sparse file around 100 MB");
    }
    let cases = [
        (folder.to_str().unwrap_or_default(), "is not a file"),
        (heavy.to_str().unwrap_or_default(), "is over 100 MB"),
        (
            just_light_enough.to_str().unwrap_or_default(),
            "is not a PNG or JPEG picture",
        ),
    ];

    for (picture, hint) in cases {
        let run = trixelate(&[picture, "--width", "4"]);

        assert_eq!(run.status.code(), Some(1), "{picture}");
        assert!(text(&run.stderr).contains(hint), "{}", text(&run.stderr));
    }
}

/// An A4 page upright and on its side, in points, as a PDF's media box gives it.
const A4_UPRIGHT: &str = "595.2 841.92";
const A4_ON_ITS_SIDE: &str = "841.92 595.2";

/// The width and height of a PDF's first page, in points as written in its media box, if it is a PDF.
fn page_size(pdf: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(pdf.strip_prefix(b"%PDF-")?);
    let (_, rest) = text.split_once("/MediaBox [0 0 ")?;
    let (size, _) = rest.split_once(']')?;
    Some(size.to_string())
}

#[test]
fn a_poster_on_a2_takes_64_cells_across_and_8_colours_on_a_page_of_its_size() {
    let folder = folder("poster");
    let picture = a_picture_in(&folder);
    let run = trixelate(&[picture.to_str().unwrap_or_default(), "--format", "a2"]);

    assert!(run.status.success(), "{}", text(&run.stderr));
    let bytes = written_beside(&folder, "sunset-64-8-a2", "sheet.pdf")
        .and_then(|poster| fs::read(poster).ok())
        .unwrap_or_default();
    assert_eq!(page_size(&bytes).as_deref(), Some("1190.64 1683.84"));
}

#[test]
fn without_a_width_or_colours_a_sheet_takes_its_papers_usual_32_cells_and_4_colours_on_a4_24_and_4_on_a5()
 {
    let folder = folder("default-width");
    let picture = a_picture_in(&folder);
    let picture = picture.to_str().unwrap_or_default();

    let on_a4 = trixelate(&[picture]);
    let on_a5 = trixelate(&[picture, "--format", "a5"]);

    assert!(on_a4.status.success(), "{}", text(&on_a4.stderr));
    assert!(on_a5.status.success(), "{}", text(&on_a5.stderr));
    assert!(written_beside(&folder, "sunset-32-4-a4", "sheet.pdf").is_some());
    assert!(written_beside(&folder, "sunset-24-4-a5", "sheet.pdf").is_some());
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
    let cases: [&[&str]; 8] = [
        &["missing.png", "--width", "0"],
        &["missing.png", "--width", "4", "--colours", "25"],
        &["missing.png", "--width", "4", "--colours", "two"],
        &["--width", "4"],
        &["missing.png", "--width", "4", "--margin", "4"],
        &["missing.png", "--width", "4", "--margin", "21"],
        &["missing.png", "--width", "4", "--format", "letter"],
        &["missing.png", "--pieces", "--preview"],
    ];

    for arguments in cases {
        let run = trixelate(arguments);

        assert_eq!(run.status.code(), Some(2), "{arguments:?}");
        assert!(!text(&run.stderr).contains("cannot read"), "{arguments:?}");
    }
}

#[test]
fn asked_for_pieces_the_command_writes_a_pdf_of_a4_pages_beside_its_picture_named_as_pieces_and_says_how_many()
 {
    let folder = folder("pieces");
    let picture = a_picture_in(&folder);

    let run = trixelate(&[
        picture.to_str().unwrap_or_default(),
        "--format",
        "a2",
        "--pieces",
    ]);

    let pdf = written_beside(&folder, "sunset-64-8-a2", "pieces.pdf")
        .and_then(|pieces| fs::read(pieces).ok())
        .unwrap_or_default();
    let pages = String::from_utf8_lossy(&pdf)
        .matches("/Type /Page ")
        .count();
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(pages, 8);
    assert!(
        text(&run.stdout).contains(": 8 A4 pages, 2 across and 4 down, to print at actual size")
    );
}
