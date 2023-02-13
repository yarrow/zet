//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

struct Chunk<'a> {
    style: Style,
    text: &'a str,
}

fn main() {
    let input = include_str!("help.txt");
    let help = parse(input);
    for line in help {
        println!("{}", line.style.style(line.text));
    }
}

fn parse<'a>(text: &'a str) -> Vec<Chunk<'a>> {
    let color = vec![Style::new().blue(), Style::new().green()];
    let mut c = 0;
    let lines = text
        .lines()
        .map(|line| {
            c = (c + 1) & 1;
            Chunk {
                style: color[c],
                text: line,
            }
        })
        .collect();
    lines
}
