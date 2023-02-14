//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

struct Entry<'a> {
    item: &'a str,
    caption: &'a str,
}
enum Part<'a> {
    Paragraph(&'a str),
    Usage(&'a str),
    Section {
        title: &'a str,
        entries: Vec<Entry<'a>>,
    },
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

fn parse<'a>(text: &'a str) -> Help<'a> {
    const USAGE: &str = "Usage: ";
    let mut help = Vec::new();
    let mut lines = text.lines().fuse();
    while let Some(line) = lines.next() {
        if line.ends_with(':') {
            let title = line;
            let mut entries = Vec::new();
            while let Some(entry) = lines.next() {
                let entry = entry.trim_end();
                if entry.is_empty() {
                    break;
                } else {
                    let Some(sp_sp) = entry.rfind("  ") else { panic!("No double space in {entry}") };
                    let (item, caption) = entry.split_at(sp_sp + 2);
                    entries.push(Entry { item, caption });
                }
            }
            help.push(Part::Section { title, entries });
            help.push(Part::Paragraph(""));
        } else {
            help.push(if line.starts_with(USAGE) {
                let line = &line[USAGE.len()..];
                let (_, args) = line.split_at(line.find(' ').unwrap_or(line.len()));
                Part::Usage(args)
            } else {
                Part::Paragraph(line)
            });
        }
    }
    if let Some(last) = help.last() {
        if let Part::Paragraph(text) = last {
            if text.is_empty() {
                help.pop();
            }
        }
    }
    Help(help)
}
