//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

enum Line<'a> {
    Paragraph(&'a str),
    Usage(&'a str),
    Heading(&'a str),
    Entry { item: &'a str, caption: &'a str },
}
struct Help<'a>(Vec<Line<'a>>);

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
                Line::Paragraph(text) => println!("{}", plain.style(text)),
                Line::Usage(args) => {
                    println!("{}{}{}", heading.style("Usage: "), name, plain.style(args),)
                }
                Line::Heading(text) => println!("{}", heading.style(text)),
                Line::Entry { item, caption } => {
                    println!("{} {}", entry.style(item), plain.style(caption));
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

fn parse<'a>(text: &'a str) -> Help<'a> {
    const USAGE: &str = "Usage: ";
    Help(
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
                    let (_, args) = line.split_at(line.find(' ').unwrap_or(line.len()));
                    Line::Usage(args)
                } else {
                    Line::Paragraph(line)
                }
            })
            .collect(),
    )
}
