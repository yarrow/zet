use std::fs::File;
use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::{prelude::*, TempDir};
use indexmap::{IndexMap, IndexSet};
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

#[test]
#[allow(non_snake_case)]
fn prints_version_with_V_or_version_flag() {
    for version in ["-V", "--version"] {
        let output = run([version]).unwrap();
        let output = String::from_utf8(output.stdout).unwrap();
        assert!(output.contains("zet"));
        assert!(output.contains(std::env!("CARGO_PKG_VERSION")));
        assert!(output.lines().collect::<Vec<_>>().len() == 1);
    }
}
const OP_NAMES: [OpName; 7] =
    [Intersect, Union, Diff, Single, SingleByFile, Multiple, MultipleByFile];
fn subcommand_for(op: OpName) -> &'static str {
    match op {
        Union => "union",
        Intersect => "intersect",
        Diff => "diff",
        Single => "single",
        SingleByFile => "single --file",
        Multiple => "multiple",
        MultipleByFile => "multiple --files",
    }
}
fn subcommands() -> [&'static str; 7] {
    OP_NAMES.map(subcommand_for)
}
fn flagged_subcommands_for(op: OpName) -> Vec<String> {
    fn flag(name: &str) -> Vec<String> {
        let mut result = vec![name.to_string(), format!("{name} --count-none")];
        match name {
            "union" | "intersect" | "diff" => result.push(format!("{name} --files")),
            _ => {}
        }
        result
    }
    match op {
        Union => flag("union"),
        Intersect => flag("intersect"),
        Diff => flag("diff"),
        Single => flag("single"),
        SingleByFile => flag("single --file"),
        Multiple => flag("multiple"),
        MultipleByFile => flag("multiple --files"),
    }
}

#[test]
fn subcommands_allow_empty_arg_list_and_produce_empty_output() {
    for subcommand in subcommands() {
        let output = run([subcommand]).unwrap();
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    }
}

#[test]
fn fail_on_missing_file() {
    for subcommand in subcommands() {
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

    let x_path = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path = &path_with(&temp, "y.txt", &y().join(""), Encoding::Plain);
    let z_path = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);
    for op in OP_NAMES {
        for sub in flagged_subcommands_for(op) {
            let output = run([&sub, x_path, y_path, z_path]).unwrap();
            eprintln!("\n\n\n\n\n");
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                xpected(op).join(""),
                "Output from {sub} ({op:?}) doesn't match expected",
            );
        }
    }
}

#[test]
fn the_last_line_of_a_file_need_not_end_in_a_newline() {
    let temp = TempDir::new().unwrap();

    let x_path = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path = &path_with(&temp, "y.txt", y().join("").trim_end_matches('\n'), Encoding::Plain);
    let z_path = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);
    for op in OP_NAMES {
        let sub = subcommand_for(op);
        let output = run([sub, x_path, y_path, z_path]).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            xpected(op).join(""),
            "Output from {sub} ({op:?}) doesn't match expected with y.txt trimmed",
        );
    }
}

#[test]
fn zet_subcommand_with_count_lines_flag_x_y_z_matches_expected_output_for_all_operations() {
    let temp = TempDir::new().unwrap();

    let x_path = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path = &path_with(&temp, "y.txt", &y().join(""), Encoding::Plain);
    let z_path = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);
    for op in OP_NAMES {
        let sub = subcommand_for(op);
        let output = run([sub, "--count-lines", x_path, y_path, z_path]).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            xpected_with_count_lines(op).join(""),
            "Output from {sub} ({op:?}) doesn't match expected",
        );
    }
}

#[test]
fn zet_subcommand_with_count_files_flag_x_y_z_matches_expected_output_for_all_operations() {
    let temp = TempDir::new().unwrap();

    let x_path = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path = &path_with(&temp, "y.txt", &y().join(""), Encoding::Plain);
    let z_path = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);
    for op in OP_NAMES {
        let sub = subcommand_for(op);
        let output = run([sub, "--count-files", x_path, y_path, z_path]).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            xpected_with_count_files(op).join(""),
            "Output from {sub} ({op:?}) doesn't match expected",
        );
    }
}

#[test]
fn zet_subcommand_with_count_flag_or_c_flag_follows_files_flag() {
    let temp = TempDir::new().unwrap();

    let x_path = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path = &path_with(&temp, "y.txt", &y().join(""), Encoding::Plain);
    let z_path = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);
    for op in OP_NAMES {
        let sub = subcommand_for(op);
        for flag in ["-c", "--count"] {
            let output = run([sub, flag, x_path, y_path, z_path]).unwrap();
            let expected = if sub.contains("--file") {
                xpected_with_count_files(op)
            } else {
                xpected_with_count_lines(op)
            };
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                expected.join(""),
                "Output from {sub} ({op:?}) doesn't match expected",
            );
        }
    }
}

#[test]
fn zet_reads_stdin_when_given_a_dash() {
    let temp = TempDir::new().unwrap();

    let x_path = &path_with(&temp, "x.txt", &x().join(""), Encoding::Plain);
    let y_path = &path_with(&temp, "y.txt", &y().join(""), Encoding::Plain);
    let z_path = &path_with(&temp, "z.txt", &z().join(""), Encoding::Plain);

    let y = File::open(y_path).unwrap();
    let output = run([subcommand_for(Union), x_path, "-", z_path]).stdin(y).unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        xpected(Union).join(""),
        "Output from dash-as-stdin doesn't match expected",
    );
}

#[test]
fn zet_reads_stdin_when_there_are_no_file_arguments() {
    let temp = TempDir::new().unwrap();

    let path = &path_with(&temp, "stdin.txt", &[x(), y(), z()].concat().join(""), Encoding::Plain);

    let std_in = File::open(path).unwrap();
    let output = run([subcommand_for(Multiple)]).stdin(std_in).unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        xpected(Multiple).join(""),
        "Output from dash-as-stdin doesn't match expected",
    );
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
                String::new()
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
fn xpected_with_count_lines(op: OpName) -> Vec<String> {
    let xyz = [x(), y(), z()].concat();
    let mut count_of = IndexMap::new();
    for line in xyz {
        count_of.entry(line).and_modify(|v| *v += 1).or_insert(1);
    }
    xpected_with_count(op, &count_of)
}
fn xpected_with_count(op: OpName, count_of: &IndexMap<String, usize>) -> Vec<String> {
    INPUT
        .iter()
        .filter(|inp| inp.should_be_in(op))
        .map(|inp| {
            let line = format!("{inp:?}");
            format!("{} {line}", count_of[&line])
        })
        .collect()
}

fn xpected_with_count_files(op: OpName) -> Vec<String> {
    fn in_file(lines: &Vec<String>) -> IndexSet<String> {
        let mut seen = IndexSet::new();
        for line in lines {
            seen.insert(line.clone());
        }
        seen
    }
    let x_seen = in_file(&x());
    let y_seen = in_file(&y());
    let z_seen = in_file(&z());
    let mut count_of = IndexMap::<String, usize>::new();
    for seen in [x_seen, y_seen, z_seen] {
        for line in seen.iter() {
            *count_of.entry(line.clone()).or_insert(0) += 1;
        }
    }
    xpected_with_count(op, &count_of)
}

// These tests of the expected results are sanity checks that the expected
// outputs are themselves correct.
mod test_the_tests {
    use super::*;
    #[test]
    fn expected_union_output_is_the_concatentated_input_lines_in_order_with_no_duplicates() {
        let xyz = [x(), y(), z()].concat();
        let unique_input_lines: Vec<String> = xyz.into_iter().unique().collect();
        let union_lines = xpected(Union);
        assert!(union_lines.eq(&unique_input_lines));
    }

    #[test]
    fn each_line_occurs_at_most_once_in_the_expected_output_of_any_subcommand() {
        for op in OP_NAMES {
            let all = xpected(op);
            let uniq: Vec<String> = all.iter().unique().cloned().collect();
            assert!(all.eq(&uniq), "Output of {op:?} has duplicate lines");
        }
    }

    #[test]
    fn expected_output_is_subsequence_of_union_output_for_all_subcommands() {
        let union = xpected(Union);
        for op in OP_NAMES {
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
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Encoding {
    Plain,
    UTF8,
    LE16,
    BE16,
}

fn path_with(temp: &TempDir, name: &str, contents: &str, enc: Encoding) -> String {
    use Encoding::*;
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
    for b in source.as_bytes() {
        result.push(*b);
        result.push(0);
    }
    result
}

fn utf_16be(source: &str) -> Vec<u8> {
    let mut result = b"\xfe\xff".to_vec();
    for b in source.as_bytes() {
        result.push(0);
        result.push(*b);
    }
    result
}
#[test]
fn zet_accepts_all_encodings_and_remembers_the_first_file_has_a_byte_order_mark() {
    use Encoding::*;
    let temp = TempDir::new().unwrap();

    for enc in [Plain, UTF8, LE16, BE16] {
        let x_path = &path_with(&temp, "x.txt", &x().join(""), enc);
        let y_path = &path_with(&temp, "y.txt", &y().join(""), LE16);
        let z_path = &path_with(&temp, "z.txt", &z().join(""), BE16);
        let output = run([subcommand_for(Union), x_path, y_path, z_path]).unwrap();
        let result_string = String::from_utf8(output.stdout).unwrap();
        let mut result = &result_string[..];
        if enc == Plain {
            assert_ne!(&result[..3], UTF8_BOM, "Unexpected BOM");
        } else {
            assert_eq!(&result[..3], UTF8_BOM, "Expected BOM not found: {enc:?}");
            result = &result[3..];
        }
        assert_eq!(result, xpected(Union).join(""), "Output from {enc:?} doesn't match expected");
    }
}

#[test]
fn the_optimize_to_union_code_in_main_only_does_so_when_its_ok() {
    const INPUT: &str = "a3\nb2\nc1\na3\na3\nb2\nd1\n";

    let temp = TempDir::new().unwrap();
    let x = temp.child("x.txt");
    x.write_str(INPUT).unwrap();

    for op in OP_NAMES {
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
fn zet_terminates_every_output_line_with_the_line_terminator_of_the_first_input_line() {
    use Encoding::*;
    fn terminate_with(eol: &str, bare: &[&str]) -> String {
        bare.iter().map(|&b| b.to_string() + eol).join("")
    }
    let (a, b, c) = ("a".to_string(), "b\r\nB\nbB", "c\nC\r\ncC\r\n");
    let bare = vec!["a", "b", "B", "bB", "c", "C", "cC"];
    let temp = TempDir::new().unwrap();
    for eol in ["", "\n", "\r\n"] {
        let expected_eol = if eol.is_empty() { "\n" } else { eol };
        let expected = terminate_with(expected_eol, &bare);
        let a = a.clone() + eol;
        for enc in [Plain, UTF8, LE16, BE16] {
            let expected = if enc == Plain {
                expected.clone()
            } else {
                UTF8_BOM.to_owned() + &expected.clone()
            };
            let a_path = &path_with(&temp, "a.txt", &a, enc);
            let b_path = &path_with(&temp, "b.txt", b, LE16);
            let c_path = &path_with(&temp, "c.txt", c, BE16);
            let output = run([subcommand_for(Union), a_path, b_path, c_path]).unwrap();
            let result_string = String::from_utf8(output.stdout).unwrap();
            assert_eq!(result_string, expected, "for eol '{eol}', encoding {enc:?}");
        }
    }
}
