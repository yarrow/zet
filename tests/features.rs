#![cfg_attr(debug_assertions, allow(dead_code, unused))]
extern crate assert_cmd;
extern crate assert_fs;
extern crate predicates;

#[macro_use]
extern crate indoc;

use std::process::Command;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn requires_subcommand() {
    Command::main_binary().unwrap().assert().failure();
}

#[test]
fn intersect_allows_empty_arg_list() {
    Command::main_binary()
        .unwrap()
        .arg("intersect")
        .assert()
        .success();
}

#[test]
fn fail_on_missing_file() {
    Command::main_binary()
        .unwrap()
        .args(&["intersect", "x"])
        .assert()
        .failure();
}

const X:  &str = "x\nX\nEx\nEks\n";
const XX: &str = "x\nX\nEx\nEks\nx\nx\nX\n";

#[test]
fn single_argument_just_prints_the_unique_lines() {
    let temp = assert_fs::TempDir::new().unwrap();
    let x = temp.child("x.txt");
    x.write_str(&(XX.to_owned()+XX)).unwrap();
    let output = Command::main_binary()
        .unwrap()
        .args(&["intersect", x.path().to_str().unwrap()])
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), XX);
}
