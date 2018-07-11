#![cfg_attr(debug_assertions, allow(dead_code, unused))]
extern crate assert_cmd;
extern crate assert_fs;
extern crate predicates;

#[macro_use]
extern crate maplit;
#[macro_use]
extern crate indoc;

use std::process::Command;
use std::collections::HashMap;

use assert_cmd::prelude::*;
use assert_fs::{TempDir, prelude::*};
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

fn path_with(temp: &TempDir, name: &str, contents: &str) -> String {
    let f = temp.child(name);
    f.write_str(contents).unwrap();
    f.path().to_str().unwrap().to_string()
}


#[test]
fn single_argument_just_prints_the_unique_lines() {
    const X:  &str = "x\nX\nEx\nEks\n";
    const XX: &str = "x\nX\nEx\nEks\nx\nx\nX\n";

    let temp = TempDir::new().unwrap();
    let x_path = path_with(&temp, "x.txt", &(XX.to_owned()+XX));
    let output = Command::main_binary()
        .unwrap()
        .args(&["intersect", &x_path])
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), X);
}
#[test]
fn xsingle_argument_just_prints_the_unique_lines() {
    const X:  &str = "x\nX\nEx\nEks\n";
    const XX: &str = "x\nX\nEx\nEks\nx\nx\nX\n";

    let temp = TempDir::new().unwrap();
    let x = temp.child("x.txt");
    x.write_str(&(XX.to_owned()+XX)).unwrap();
    let output = Command::main_binary()
        .unwrap()
        .args(&["intersect", x.path().to_str().unwrap()])
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), X);
}

#[ignore]
#[test]
fn intersect_prints_lines_in_the_intersection_in_order_they_appear_in_first_file() {
    const XX: &str = "x\nX\nEx\nEks\nx\nx\nX\n";
    const YY: &str = "Ex\nx\ny\nY\nEy\nEks\ny\ny\nY\n";
    const ZZ: &str = "Eks\nx\nx\nz\nZ\nEz\nEks\nz\nz\nZ\n";
    const X_INTERSECTION: &str = "x\nEks\n";
    const Z_INTERSECTION: &str = "Eks\nx\n";

    let temp = TempDir::new().unwrap();
    let x_path = path_with(&temp, "x.txt", &XX);
    let y_path = path_with(&temp, "y.txt", &YY);
    let z_path = path_with(&temp, "z.txt", &ZZ);
    
    let output = Command::main_binary()
        .unwrap()
        .args(&["intersect", &x_path, &y_path, &z_path])
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), X_INTERSECTION);
    
    let output = Command::main_binary()
        .unwrap()
        .args(&["intersect", &z_path, &y_path, &x_path])
        .unwrap();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), Z_INTERSECTION);
}
