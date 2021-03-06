use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::{prelude::*, TempDir};
use itertools::Itertools;

fn main_binary() -> Command {
    Command::cargo_bin("zet").unwrap()
}

#[test]
fn requires_subcommand() {
    main_binary().assert().failure();
}

const SUBCOMMANDS: [&str; 5] = ["intersect", "union", "diff", "single", "multiple"];

#[test]
fn subcommands_allow_empty_arg_list_and_produce_empty_output() {
    for subcommand in SUBCOMMANDS.iter() {
        let output = main_binary().arg(subcommand).unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    }
}

#[test]
fn fail_on_missing_file() {
    for subcommand in SUBCOMMANDS.iter() {
        main_binary().args(&[subcommand, "x"]).assert().failure();
    }
}

#[test]
fn fail_bad_subcommand() {
    main_binary().args(&["OwOwOwOwOw"]).assert().failure();
}

#[test]
fn zet_subcommand_x_y_z_matches_expected_output_for_all_subcommands() {
    let temp = TempDir::new().unwrap();

    let x_path: &str = &path_with(&temp, "x.txt", X, Encoding::Plain);
    let y_path: &str = &path_with(&temp, "y.txt", Y, Encoding::Plain);
    let z_path: &str = &path_with(&temp, "z.txt", Z, Encoding::Plain);
    for sub in SUBCOMMANDS.iter() {
        let output = main_binary().args(&[sub, &x_path, &y_path, &z_path]).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected(sub),
            "Output from {} doesn't match expected",
            sub
        );
    }
}

// We're testing with files (say x.txt, y.txt, and z.txt) whose contents are
// X, Y, and Z. Each line tells us which subset of the three files it appears
// in, and that determines for which subcommands `sub` it will appear in the
// output of
//
//      zet sub x.txt y.txt z.txt

// The contents of x.txt
const X: &str = "In x, y, z.  So: union, intersect, multiple
In x only, though it appears there more than once. So: union, diff, single
In x, y, z.  So: union, intersect, multiple
Just in x.  So: union, diff, single.
In x only, though it appears there more than once. So: union, diff, single
In x only, though it appears there more than once. So: union, diff, single
In x and y.  So: union, multiple
In x and z.  So: union, multiple
Also in x, y, z.  So: union, intersect, multiple
";

// The contents of y.txt
const Y: &str = "In x, y, z.  So: union, intersect, multiple
In x and y.  So: union, multiple
Also in x, y, z.  So: union, intersect, multiple
In y and z. So: union, multiple
Just in y. So: union, single
";

// The contents of z.txt
const Z: &str = "Just in z. So: union, single
Also in x, y, z.  So: union, intersect, multiple
Just in z. So: union, single
In y and z. So: union, multiple
Just in z. So: union, single
In x, y, z.  So: union, intersect, multiple
In x and z.  So: union, multiple
In x and z.  So: union, multiple
";

// For the expected output sections below, we want to begin each line at the
// first column, so we put the opening quote mark on the line above and ignore
// the newline character that this produces.
fn expected(subcommand: &str) -> &'static str {
    match subcommand {
        "union" => &UNION[1..],
        "intersect" => &INTERSECT[1..],
        "diff" => &DIFF[1..],
        "single" => &SINGLE[1..],
        "multiple" => &MULTIPLE[1..],
        _ => panic!("There is no subcommand {}", subcommand),
    }
}

// The expected output of `zet union x.txt y.txt z.txt`
const UNION: &str = "
In x, y, z.  So: union, intersect, multiple
In x only, though it appears there more than once. So: union, diff, single
Just in x.  So: union, diff, single.
In x and y.  So: union, multiple
In x and z.  So: union, multiple
Also in x, y, z.  So: union, intersect, multiple
In y and z. So: union, multiple
Just in y. So: union, single
Just in z. So: union, single
";

// The expected output of `zet intersect x.txt y.txt z.txt`
const INTERSECT: &str = "
In x, y, z.  So: union, intersect, multiple
Also in x, y, z.  So: union, intersect, multiple
";

// The expected output of `zet diff x.txt y.txt z.txt`
const DIFF: &str = "
In x only, though it appears there more than once. So: union, diff, single
Just in x.  So: union, diff, single.
";

// The expected output of `zet single x.txt y.txt z.txt`
const SINGLE: &str = "
In x only, though it appears there more than once. So: union, diff, single
Just in x.  So: union, diff, single.
Just in y. So: union, single
Just in z. So: union, single
";

// The expected output of `zet single x.txt y.txt z.txt`
const MULTIPLE: &str = "
In x, y, z.  So: union, intersect, multiple
In x and y.  So: union, multiple
In x and z.  So: union, multiple
Also in x, y, z.  So: union, intersect, multiple
In y and z. So: union, multiple
";

// These tests of the expected results allow us to reduce the amount of
// hand checking we need to make sure the expected outputs are themselves
// correct.
#[test]
fn union_output_is_the_concatentated_input_lines_in_order_with_no_duplicates() {
    let xyz = X.to_string() + Y + Z;
    let unique_input_lines = xyz.lines().unique();
    let union_lines = expected("union").lines();
    assert!(union_lines.eq(unique_input_lines));
}

#[test]
fn output_is_subsequence_of_union_output_for_all_subcommands() {
    let union = expected("union");
    for sub in SUBCOMMANDS.iter() {
        assert!(
            is_subsequence(expected(sub), union),
            "Expected result for {} is not a subsequence of the expected result for union",
            sub
        );
    }
}

#[test]
fn each_line_occurs_at_most_once_in_the_output_of_any_subcommand() {
    for sub in SUBCOMMANDS.iter() {
        let all = expected(sub).lines();
        let uniq = all.clone().unique();
        assert!(all.eq(uniq), "Output of {} has duplicate lines", sub);
    }
}

fn is_subsequence(needles: &str, haystack: &str) -> bool {
    let needles = needles.lines();
    let mut haystack = haystack.lines();
    'next_needle: for needle in needles {
        while let Some(hay) = haystack.next() {
            if needle == hay {
                continue 'next_needle;
            }
        }
        return false;
    }
    true
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Encoding {
    Plain,
    UTF8,
    LE16,
    BE16,
}
use Encoding::*;

fn path_with(temp: &TempDir, name: &str, contents: &str, enc: Encoding) -> String {
    let f = temp.child(name);
    match enc {
        Plain => f.write_str(contents).unwrap(),
        UTF8 => {
            f.write_str((UTF8_BOM.to_owned() + contents).as_str()).unwrap();
        }
        LE16 => f.write_binary(utf_16le(contents).as_slice()).unwrap(),
        BE16 => f.write_binary(utf_16be(contents).as_slice()).unwrap(),
    }
    f.path().to_str().unwrap().to_string()
}
const UTF8_BOM: &str = "\u{FEFF}";

fn utf_16le(source: &str) -> Vec<u8> {
    let mut result = b"\xff\xfe".to_vec();
    for b in source.as_bytes().iter() {
        result.push(*b);
        result.push(0);
    }
    result
}

fn utf_16be(source: &str) -> Vec<u8> {
    let mut result = b"\xfe\xff".to_vec();
    for b in source.as_bytes().iter() {
        result.push(0);
        result.push(*b);
    }
    result
}
#[test]
fn zet_accepts_all_encodings_and_remembers_the_first_file_has_a_byte_order_mark() {
    let temp = TempDir::new().unwrap();

    for enc in [Plain, UTF8, LE16, BE16].iter() {
        let x_path: &str = &path_with(&temp, "x.txt", X, *enc);
        let y_path: &str = &path_with(&temp, "y.txt", Y, LE16);
        let z_path: &str = &path_with(&temp, "z.txt", Z, BE16);
        let output = main_binary().args(&["union", &x_path, &y_path, &z_path]).unwrap();
        let result_string = String::from_utf8(output.stdout).unwrap();
        let mut result = &result_string[..];
        if *enc == Plain {
            assert_ne!(&result[..3], UTF8_BOM, "Unexpected BOM");
        } else {
            assert_eq!(&result[..3], UTF8_BOM, "Expected BOM not found: {:?}", *enc);
            result = &result[3..];
        }
        assert_eq!(result, expected("union"), "Output from {:?} doesn't match expected", *enc);
    }
}

#[test]
fn single_argument_just_prints_the_unique_lines_for_all_but_multiple() {
    const EXPECTED: &str = "x\nX\nEx\nEks\n";
    const XX: &str = "x\nX\nEx\nEks\nx\nx\nX\n";

    let temp = TempDir::new().unwrap();
    let x = temp.child("x.txt");
    x.write_str(&(XX.to_owned() + XX)).unwrap();

    for subcommand in SUBCOMMANDS.iter() {
        let output = main_binary().args(&[subcommand, x.path().to_str().unwrap()]).unwrap();
        let result = String::from_utf8(output.stdout).unwrap();
        assert_eq!(result, if subcommand == &"multiple" { "" } else { EXPECTED });
    }
}

#[test]
fn the_last_line_of_a_file_need_not_end_in_a_newline() {
    const EXPECTED: &str = "x\nX\nEx\nEks\na\n";
    const XX: &str = "x\nX\nEx\nEks\nx\nx\nX\n";

    let temp = TempDir::new().unwrap();
    let x = temp.child("x.txt");
    x.write_str(&(XX.to_owned() + XX + "a")).unwrap();
    let x_path = x.path().to_str().unwrap();

    for subcommand in SUBCOMMANDS.iter() {
        let mut subcommand_with_args = vec![subcommand, &x_path];
        if subcommand == &"multiple" {
            subcommand_with_args.push(&x_path)
        }
        let output = main_binary().args(&subcommand_with_args).unwrap();
        let result = String::from_utf8(output.stdout).unwrap();
        assert_eq!(result, EXPECTED);
    }
}

#[test]
fn zet_terminates_every_output_line_with_the_line_terminator_of_the_first_input_line() {
    fn terminate_with(eol: &str, bare: &Vec<&str>) -> String {
        bare.iter().map(|b| b.to_string() + eol).join("")
    }
    let (a, b, c) = ("a".to_string(), "b\r\nB\nbB", "c\nC\r\ncC\r\n");
    let bare = vec!["a", "b", "B", "bB", "c", "C", "cC"];
    let temp = TempDir::new().unwrap();
    for eol in ["", "\n", "\r\n"].iter() {
        let expected_eol = if *eol == "" { "\n" } else { eol };
        let expected = terminate_with(expected_eol, &bare);
        let a = a.clone() + *eol;
        for enc in [Plain, UTF8, LE16, BE16].iter() {
            let expected = if *enc == Plain {
                expected.clone()
            } else {
                UTF8_BOM.to_owned() + &expected.clone()
            };
            let a_path: &str = &path_with(&temp, "a.txt", &a, *enc);
            let b_path: &str = &path_with(&temp, "b.txt", b, LE16);
            let c_path: &str = &path_with(&temp, "c.txt", &c, BE16);
            let output = main_binary().args(&["union", &a_path, &b_path, &c_path]).unwrap();
            let result_string = String::from_utf8(output.stdout).unwrap();
            assert_eq!(result_string, expected, "for eol '{}', encoding {:?}", eol, enc);
        }
    }
}
