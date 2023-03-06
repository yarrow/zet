use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::{prelude::*, TempDir};
use itertools::Itertools;
use once_cell::sync::Lazy;
use zet::args::OpName::{self, *};

fn main_binary() -> Command {
    Command::cargo_bin("zet").unwrap()
}
fn run<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut app = main_binary();
    for arg_block in args {
        for arg in arg_block.as_ref().split_ascii_whitespace() {
            app.arg(arg);
        }
    }
    app
}
#[test]
fn prints_help_if_no_subcommand() {
    let output = main_binary().unwrap();
    assert!(String::from_utf8(output.stdout).unwrap().contains("Usage:"));
}

const UNION: &str = "union";
const INTERSECT: &str = "intersect";
const DIFF: &str = "diff";
const SINGLE_BY_FILE: &str = "single --by-file";
const MULTIPLE_BY_FILE: &str = "multiple --by-file";
const SUBCOMMANDS: [&str; 5] = [INTERSECT, UNION, DIFF, SINGLE_BY_FILE, MULTIPLE_BY_FILE];
const OP_NAMES: [OpName; 7] =
    [Intersect, Union, Diff, Single, SingleByFile, Multiple, MultipleByFile];
fn subcommand_for(op: OpName) -> &'static str {
    match op {
        Union => UNION,
        Intersect => INTERSECT,
        Diff => DIFF,
        Single => "single",
        SingleByFile => SINGLE_BY_FILE,
        Multiple => "multiple",
        MultipleByFile => MULTIPLE_BY_FILE,
    }
}

#[test]
fn subcommands_allow_empty_arg_list_and_produce_empty_output() {
    for subcommand in SUBCOMMANDS.iter() {
        let output = run([subcommand]).unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    }
}

#[test]
fn fail_on_missing_file() {
    for subcommand in SUBCOMMANDS.iter() {
        run([subcommand, "x"]).assert().failure();
    }
}

#[test]
fn fail_bad_subcommand() {
    run(["OwOwOwOwOw"]).assert().failure();
}

#[test]
fn zet_subcommand_x_y_z_matches_expected_output_for_all_operations() {
    let temp = TempDir::new().unwrap();

    let x_path: &str = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path: &str = &path_with(&temp, "y.txt", &y().join(""), Encoding::Plain);
    let z_path: &str = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);
    for &op in OP_NAMES.iter() {
        let sub = subcommand_for(op);
        let output = run([sub, x_path, y_path, z_path]).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            xpected(op).join(""),
            "Output from {sub} ({op:?}) doesn't match expected",
        );
    }
}

use std::fmt;
#[derive(Clone)]
struct TestInput {
    x: usize,
    y: usize,
    z: usize,
    tag: &'static str,
    expect: Vec<OpName>,
}
impl TestInput {
    fn should_be_in(&self, op: OpName) -> bool {
        self.expect.contains(&op)
    }
}
impl fmt::Debug for TestInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return writeln!(
            f,
            "{} {}{}{}{:?}",
            self.tag,
            show('x', self.x),
            show('y', self.y),
            show('z', self.z),
            self.expect
        );
        fn show(x: char, count: usize) -> String {
            if count == 0 {
                "".to_string()
            } else if count == 1 {
                format!("{x} ")
            } else {
                format!("{x}({count}) ")
            }
        }
    }
}
// Each TestInput record gets formatted to a unique string, to be put in files
// x.txt, y.txt, and/or z.txt.  The x, y, and z fields tell how many times to
// put the formatted record into each file, and expect field tells whether we
// expect the formatted record to appear in the output of the command associated
// with each OpName.
//
static INPUT: Lazy<Vec<TestInput>> = Lazy::new(|| {
    use OpName::{
        Diff as D, Intersect as I, Multiple as M, MultipleByFile as MBF, Single as S,
        SingleByFile as SBF, Union as U,
    };
    vec![
        TestInput { x: 1, y: 1, z: 1, tag: "In xyz", expect: vec![U, I, MBF, M] },
        TestInput { x: 3, y: 0, z: 0, tag: "In x 3 times", expect: vec![U, D, SBF, M] },
        TestInput { x: 1, y: 0, z: 0, tag: "In x once", expect: vec![U, D, S, SBF] },
        TestInput { x: 1, y: 1, z: 0, tag: "In xy", expect: vec![U, MBF, M] },
        TestInput { x: 1, y: 2, z: 0, tag: "In x. In y twice", expect: vec![U, MBF, M] },
        TestInput { x: 1, y: 0, z: 1, tag: "In xz", expect: vec![U, MBF, M] },
        TestInput { x: 1, y: 1, z: 1, tag: "In xyz also", expect: vec![U, I, MBF, M] },
        TestInput { x: 0, y: 1, z: 1, tag: "In yz", expect: vec![U, MBF, M] },
        TestInput { x: 0, y: 1, z: 0, tag: "In y once", expect: vec![U, S, SBF] },
        TestInput { x: 0, y: 0, z: 1, tag: "In z once", expect: vec![U, S, SBF] },
    ]
});
fn xpected(op: OpName) -> Vec<String> {
    INPUT.iter().filter(|inp| inp.should_be_in(op)).map(|inp| format!("{inp:?}")).collect()
}
fn text_for(xyz: impl Fn(&TestInput) -> usize) -> Vec<String> {
    let mut text = Vec::new();
    for line in INPUT.iter() {
        for _ in 0..xyz(line) {
            text.push(format!("{line:?}"));
        }
    }
    text
}
fn x() -> Vec<String> {
    text_for(|r| r.x)
}
fn y() -> Vec<String> {
    text_for(|r| r.y)
}
fn z() -> Vec<String> {
    text_for(|r| r.z)
}
// These tests of the expected results are sanity checks that the expected
// outputs are themselves correct.
#[test]
fn expected_union_output_is_the_concatentated_input_lines_in_order_with_no_duplicates() {
    let xyz = vec![x(), y(), z()].concat();
    let unique_input_lines: Vec<String> = xyz.into_iter().unique().collect();
    let union_lines = xpected(Union);
    assert!(union_lines.eq(&unique_input_lines));
}

#[test]
fn each_line_occurs_at_most_once_in_the_expected_output_of_any_subcommand() {
    for &op in OP_NAMES.iter() {
        let all = xpected(op);
        let uniq: Vec<String> = all.iter().unique().cloned().collect();
        assert!(all.eq(&uniq), "Output of {op:?} has duplicate lines");
    }
}

#[test]
fn expected_output_is_subsequence_of_union_output_for_all_subcommands() {
    let union = xpected(Union);
    for &op in OP_NAMES.iter() {
        assert!(
            is_subsequence(&xpected(op), &union),
            "Expected result for {op:?} is not a subsequence of the expected result for Union",
        );
    }
    fn is_subsequence(needles: &Vec<String>, haystack: &Vec<String>) -> bool {
        'next_needle: for needle in needles {
            for hay in haystack {
                if *needle == *hay {
                    continue 'next_needle;
                }
            }
            return false;
        }
        true
    }
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
        let x_path: &str = &path_with(&temp, "x.txt", &x().join(""), *enc);
        let y_path: &str = &path_with(&temp, "y.txt", &y().join(""), LE16);
        let z_path: &str = &path_with(&temp, "z.txt", &z().join(""), BE16);
        let output = run([UNION, x_path, y_path, z_path]).unwrap();
        let result_string = String::from_utf8(output.stdout).unwrap();
        let mut result = &result_string[..];
        if *enc == Plain {
            assert_ne!(&result[..3], UTF8_BOM, "Unexpected BOM");
        } else {
            assert_eq!(&result[..3], UTF8_BOM, "Expected BOM not found: {:?}", *enc);
            result = &result[3..];
        }
        assert_eq!(
            result,
            xpected(Union).join(""),
            "Output from {:?} doesn't match expected",
            *enc
        );
    }
}

#[test]
fn the_optimize_to_union_code_in_main_only_does_so_when_its_ok() {
    const INPUT: &str = "a3\nb2\nc1\na3\na3\nb2\nd1\n";

    let temp = TempDir::new().unwrap();
    let x = temp.child("x.txt");
    x.write_str(INPUT).unwrap();

    for &op in OP_NAMES.iter() {
        let output = run([subcommand_for(op), x.path().to_str().unwrap()]).unwrap();
        let result = String::from_utf8(output.stdout).unwrap();
        let expected = match op {
            Intersect | Union | Diff | SingleByFile => "a3\nb2\nc1\nd1\n",
            Single => "c1\nd1\n",
            Multiple => "a3\nb2\n",
            MultipleByFile => "",
        };
        assert_eq!(result, expected, "Expected {op:?} result to be '{expected}'");
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
        if subcommand == &MULTIPLE_BY_FILE {
            subcommand_with_args.push(&x_path)
        }
        let output = run(&subcommand_with_args).unwrap();
        let result = String::from_utf8(output.stdout).unwrap();
        assert_eq!(result, EXPECTED);
    }
}

#[test]
fn zet_terminates_every_output_line_with_the_line_terminator_of_the_first_input_line() {
    fn terminate_with(eol: &str, bare: &[&str]) -> String {
        bare.iter().map(|b| b.to_string() + eol).join("")
    }
    let (a, b, c) = ("a".to_string(), "b\r\nB\nbB", "c\nC\r\ncC\r\n");
    let bare = vec!["a", "b", "B", "bB", "c", "C", "cC"];
    let temp = TempDir::new().unwrap();
    for eol in ["", "\n", "\r\n"].iter() {
        let expected_eol = if eol.is_empty() { "\n" } else { eol };
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
            let c_path: &str = &path_with(&temp, "c.txt", c, BE16);
            let output = run([UNION, a_path, b_path, c_path]).unwrap();
            let result_string = String::from_utf8(output.stdout).unwrap();
            assert_eq!(result_string, expected, "for eol '{eol}', encoding {enc:?}");
        }
    }
}
