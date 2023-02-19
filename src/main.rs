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
#![allow(
    clippy::missing_errors_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::items_after_statements,
    clippy::needless_pass_by_value
)]
//#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]
#![allow(dead_code, unused_imports, unused_variables)]

use help_zet::style;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use terminal_size::{terminal_size, Height, Width};
use textwrap::{self, wrap};

enum HelpItem<'a> {
    Usage(&'a str),
    Paragraph(&'a str),
    Section(Section<'a>),
}
struct Section<'a> {
    title: &'a str,
    entries: Vec<Entry<'a>>,
}
struct Entry<'a> {
    item: &'a str,
    caption: &'a str,
}

fn print_help() {
    let input = include_str!("help.txt");
    let help = parse(input);
    let version = std::env!("CARGO_PKG_VERSION");
    let name = style::app_name("zet");
    println!("{name} {version}");
    for help_item in help {
        match help_item {
            HelpItem::Paragraph(text) => wrap(text, &C.wrap_options)
                .iter()
                .for_each(|line| println!("{line}")),
            HelpItem::Usage(args) => {
                println!("{}{}{}", style::title("Usage: "), name, args)
            }
            HelpItem::Section(s) => s.print(),
        };
    }
}

fn parse(text: &str) -> Vec<HelpItem> {
    const USAGE: &str = "Usage: ";
    let mut help = Vec::new();
    let mut lines = text.lines().fuse();
    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix(USAGE) {
            let (_, args) = rest.split_at(rest.find(' ').unwrap_or(rest.len()));
            help.push(HelpItem::Usage(args))
        } else if line.ends_with(':') {
            let title = line;
            let mut entries = Vec::new();
            let result = loop {
                let Some(entry) = lines.next() else { break None };
                let entry = entry.trim_end();
                if entry.is_empty() {
                    break Some(HelpItem::Paragraph(""));
                }
                let Some(sp_sp) = entry.rfind("  ") else { panic!("No double space in {entry}") };
                let (item, caption) = entry.split_at(sp_sp + 2);
                entries.push(Entry { item, caption });
            };
            help.push(HelpItem::Section(Section { title, entries }));
            if let Some(part) = result {
                help.push(part)
            }
        } else {
            help.push(HelpItem::Paragraph(line))
        }
    }
    help
}

impl<'a> Section<'a> {
    fn print(&self) {
        println!("{}", style::title(self.title));
        let fits_in_line = self.entries.iter().all(Entry::fits_in_line);
        if fits_in_line {
            for entry in &self.entries {
                println!("{}{}", entry.styled_item(), entry.caption);
            }
        } else {
            let same_line_help = self.same_line_help_lines();
            let next_line_help = self.next_line_help_lines();
            let help = if badness(&same_line_help) <= badness(&next_line_help) {
                &same_line_help
            } else {
                &next_line_help
            };
            for line in help.iter().flatten() {
                println!("{line}");
            }
        }
        fn badness<T>(vv: &[Vec<T>]) -> usize {
            vv.iter().fold(0, |total, v| {
                let m = v.len().saturating_sub(2);
                total + v.len() + m * 2
            })
        }
    }
    fn next_line_help_indent(&self) -> &'a str {
        let max_blank_prefix_size = self
            .entries
            .iter()
            .map(Entry::blank_prefix_size)
            .fold(0, std::cmp::Ord::max);
        let indent_len = (max_blank_prefix_size + 4).min(BLANKS.len());
        &BLANKS[..indent_len]
    }
    fn next_line_help_lines(&self) -> Vec<Vec<Cow<'a, str>>> {
        let mut result = Vec::new();
        let indent = self.next_line_help_indent();
        for entry in &self.entries {
            result.push(vec![Cow::Owned(entry.styled_item())]);
            result.push(entry.next_line_caption(indent));
        }
        result
    }
    fn same_line_help_lines(&self) -> Vec<Vec<Cow<'a, str>>> {
        self.entries.iter().map(Entry::same_line_help).collect()
    }
}

const BLANKS: &str = "                                                        ";
impl<'a> Entry<'a> {
    fn styled_item(&self) -> String {
        style::item(self.item)
    }
    fn fits_in_line(&self) -> bool {
        self.item.len() + self.caption.len() <= C.line_width
    }
    fn blank_prefix_size(&self) -> usize {
        use bstr::ByteSlice;
        self.item
            .as_bytes()
            .find_not_byteset(b" ")
            .unwrap_or(self.item.len())
    }
    fn next_line_caption(&self, indent: &'a str) -> Vec<Cow<'a, str>> {
        wrap(
            self.caption,
            C.wrap_options
                .clone()
                .initial_indent(indent)
                .subsequent_indent(indent),
        )
    }
    fn same_line_help(&self) -> Vec<Cow<'a, str>> {
        let rest = &BLANKS[..(self.item.len() + 4).min(BLANKS.len())];
        let first = self.styled_item();
        let options = C
            .wrap_options
            .clone()
            .initial_indent(&first)
            .subsequent_indent(rest);
        wrap(self.caption, options)
    }
}

struct Constants<'a> {
    line_width: usize,
    wrap_options: textwrap::Options<'a>,
}
static C: Lazy<Constants> = Lazy::new(|| {
    fn from_env() -> Option<usize> {
        std::env::var_os("COLUMNS")?.to_str()?.parse::<usize>().ok()
    }
    let line_width = if let Some((Width(width), Height(_))) = terminal_size() {
        width as usize
    } else {
        from_env().unwrap_or(100)
    };
    let wrap_options = textwrap::Options::new(line_width);

    Constants {
        line_width,
        wrap_options,
    }
});

fn main() {
    style::init();
    style::set_color_choice(style::ColorChoice::Auto);
    print_help();
}
