//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

enum Part<'a> {
    Paragraph(&'a str),
    Usage(&'a str),
    Section { title: &'a str },
    Entry { item: &'a str, caption: &'a str },
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
                    println!("{}{}{}", heading.style("Usage: "), name, plain.style(args),)
                }
                Part::Section { title } => println!("{}", heading.style(title)),
                Part::Entry { item, caption } => {
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
    let mut help = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        help.push(if line.ends_with(':') {
            Part::Section { title: line }
        } else if line.starts_with(' ') {
            let Some(sp_sp) = line.rfind("  ") else { panic!("No double space in {line}") };
            let (item, caption) = line.split_at(sp_sp + 2);
            Part::Entry { item, caption }
        } else if line.starts_with(USAGE) {
            let line = &line[USAGE.len()..];
            let (_, args) = line.split_at(line.find(' ').unwrap_or(line.len()));
            Part::Usage(args)
        } else {
            Part::Paragraph(line)
        });
    }
    Help(help)
}
