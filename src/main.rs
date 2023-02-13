//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::{Style, Styled};

enum Line<'a> {
    Paragraph(&'a str),
    Usage(&'a str),
    Heading(&'a str),
    Entry { item: &'a str, caption: &'a str },
}
impl<'a> Line<'a> {
    fn styled(&'a self) -> Styled<&'a str> {
        match self {
            Line::Paragraph(text) => Style::new().style(text),
            Line::Usage(text) => Style::new().bold().yellow().style(text),
            Line::Heading(text) => Style::new().bold().yellow().style(text),
            Line::Entry { item, caption } => Style::new().green().style(item),
        }
    }
}
fn main() {
    let input = include_str!("help.txt");
    let help = parse(input);
    for line in help {
        println!("{}", line.styled())
    }
}

fn parse<'a>(text: &'a str) -> Vec<Line<'a>> {
    text.lines()
        .map(|line| {
            if line.ends_with(':') {
                Line::Heading(line)
            } else if line.starts_with(' ') {
                Line::Entry {
                    item: line,
                    caption: "",
                }
            } else if line.starts_with("Usage: ") {
                Line::Usage(line)
            } else {
                Line::Paragraph(line)
            }
        })
        .collect()
}
