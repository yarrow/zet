#![deny(
    warnings,
    clippy::all,
    clippy::pedantic,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_must_use
)]
#![allow(clippy::missing_errors_doc, clippy::semicolon_if_nothing_returned)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

struct Entry<'a> {
    item: &'a str,
    caption: &'a str,
}
enum Part<'a> {
    Usage(&'a str),
    Section {
        title: &'a str,
        entries: Vec<Entry<'a>>,
    },
    Paragraph(&'a str),
}
struct Help<'a>(Vec<Part<'a>>);

impl<'a> Help<'a> {
    fn print(&'a self) {
        let plain = Style::new();
        let heading = Style::new().yellow();
        let entry = Style::new().green();
        let version = std::env!("CARGO_PKG_VERSION");
        let name = entry.bold().style("zet");
        println!("{} {}", name, plain.style(version));
        for line in &self.0 {
            match line {
                Part::Paragraph(text) => println!("{}", plain.style(text)),
                Part::Usage(args) => {
                    println!("{}{}{}", heading.style("Usage: "), name, plain.style(args))
                }
                Part::Section { title, entries } => {
                    println!("{}", heading.style(title));
                    for Entry { item, caption } in entries {
                        println!("{} {}", entry.style(item), plain.style(caption));
                    }
                }
            };
        }
    }
}
fn main() {
    let input = include_str!("help.txt");
    let help = parse(input);
    help.print();
}

fn parse(text: &str) -> Help {
    const USAGE: &str = "Usage: ";
    let mut help = Vec::new();
    let mut lines = text.lines().fuse();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix(USAGE) {
            let (_, args) = rest.split_at(line.find(' ').unwrap_or(rest.len()));
            help.push(Part::Usage(args))
        } else if line.ends_with(':') {
            let title = line;
            let mut entries = Vec::new();
            let result = loop {
                let Some(entry) = lines.next() else { break None };
                let entry = entry.trim_end();
                if entry.is_empty() {
                    break Some(Part::Paragraph(""));
                }
                let Some(sp_sp) = entry.rfind("  ") else { panic!("No double space in {entry}") };
                let (item, caption) = entry.split_at(sp_sp + 2);
                entries.push(Entry { item, caption });
            };
            help.push(Part::Section { title, entries });
            if let Some(part) = result {
                help.push(part)
            }
        } else {
            help.push(Part::Paragraph(line))
        }
    }
    Help(help)
}
