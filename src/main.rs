//use owo_colors::{OwoColorize, Stream::Stdout, Style, Styled};
use owo_colors::Style;

struct Chunk<'a> {
    style: Style,
    text: &'a str,
}

fn main() {
    let input = include_str!("help.txt");
    let help = parse(input);
    println!("{}", help.style.style(help.text));
}

fn parse<'a>(text: &'a str) -> Chunk<'a> {
    Chunk {
        style: Style::new().blue(),
        text,
    }
}
