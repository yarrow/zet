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
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;
use terminal_size::{terminal_size, Height, Width};

struct Entry<'a> {
    item: &'a str,
    caption: &'a str,
}
struct Section<'a> {
    title: &'a str,
    entries: Vec<Entry<'a>>,
}
enum Part<'a> {
    Usage(&'a str),
    Paragraph(&'a str),
    Section(Section<'a>),
}

const OPTION_INDENT: &str = "\n          ";
const OTHER_INDENT: &str = "\n      ";
impl<'a> Section<'a> {
    fn next_line_indent(&'a self) -> &'static str {
        if self.title.starts_with("Options") {
            OPTION_INDENT
        } else {
            OTHER_INDENT
        }
    }
    fn next_line_help(&self) -> bool {
        let term_width = term_width();
        self.entries
            .iter()
            .any(|e| e.item.len() + e.caption.len() > term_width)
    }
}
struct Help<'a>(Vec<Part<'a>>);

fn term_width() -> usize {
    fn inner() -> Option<usize> {
        std::env::var_os("COLUMNS")?.to_str()?.parse::<usize>().ok()
    }
    if let Some((Width(width), Height(_))) = terminal_size() {
        width as usize
    } else {
        inner().unwrap_or(80)
    }
}
impl<'a> Help<'a> {
    fn print(&'a self) {
        let term_width = term_width();
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
                Part::Section(s) => {
                    let between = if s.next_line_help() {
                        s.next_line_indent()
                    } else {
                        ""
                    };
                    println!("{}", heading.style(s.title));
                    for Entry { item, caption } in &s.entries {
                        println!("{}{}{}", entry.style(item), between, plain.style(caption));
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
            let (_, args) = rest.split_at(rest.find(' ').unwrap_or(rest.len()));
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
            help.push(Part::Section(Section { title, entries }));
            if let Some(part) = result {
                help.push(part)
            }
        } else {
            help.push(Part::Paragraph(line))
        }
    }
    Help(help)
}
