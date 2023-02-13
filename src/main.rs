//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

enum Line<'a> {
    Paragraph(&'a str),
    Usage { command: &'a str, args: &'a str },
    Heading(&'a str),
    Entry { item: &'a str, caption: &'a str },
}
impl<'a> Line<'a> {
    fn print(&'a self) {
        let plain = Style::new();
        let heading = Style::new().yellow();
        let entry = Style::new().green();
        match self {
            Line::Paragraph(text) => println!("{}", plain.style(text)),
            Line::Usage { command, args } => println!(
                "{}{}{}",
                heading.style("Usage: "),
                entry.style(command),
                plain.style(args),
            ),
            Line::Heading(text) => println!("{}", heading.style(text)),
            Line::Entry { item, caption } => {
                println!("{} {}", entry.style(item), plain.style(caption));
            }
        };
    }
}
fn main() {
    let input = include_str!("help.txt");
    let help = parse(input);
    for line in help {
        line.print()
    }
}

fn parse<'a>(text: &'a str) -> Vec<Line<'a>> {
    const USAGE: &str = "Usage: ";
    text.lines()
        .map(|line| {
            if line.ends_with(':') {
                Line::Heading(line)
            } else if line.starts_with(' ') {
                let Some(sp_sp) = line.rfind("  ") else { panic!("No double space in {line}") };
                let (item, caption) = line.split_at(sp_sp + 2);
                Line::Entry { item, caption }
            } else if line.starts_with(USAGE) {
                let line = &line[USAGE.len()..];
                let (command, args) = line.split_at(line.find(' ').unwrap_or(line.len()));
                Line::Usage { command, args }
            } else {
                Line::Paragraph(line)
            }
        })
        .collect()
}
