use crate::styles::{app_name, as_item, as_title, ColorChoice, StyledStr};
use anstream;
use anyhow::{bail, Result};
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
    item: StyledStr<'a>,
    caption: &'a str,
}

fn name() -> StyledStr<'static> {
    app_name("zet")
}

pub(crate) fn version() -> String {
    let version = std::env!("CARGO_PKG_VERSION");
    let name = name();
    format!("{name} {version}")
}

pub(crate) fn print(color_choice: &ColorChoice) -> Result<()> {
    let color_choice = match color_choice {
        ColorChoice::Always => anstream::ColorChoice::Always,
        ColorChoice::Auto => anstream::ColorChoice::Auto,
        ColorChoice::Never => anstream::ColorChoice::Never,
    };
    let mut stdout = anstream::AutoStream::new(std::io::stdout().lock(), color_choice);
    match fallable_print(&mut stdout) {
        Err(e) => bail!("failed printing to stdout: {e}"),
        Ok(_) => Ok(()),
    }
}
fn fallable_print(stdout: &mut dyn std::io::Write) -> std::io::Result<usize> {
    let input = include_str!("help.txt");
    let help = parse(input);
    writeln!(stdout, "{}", version())?;
    for help_item in help {
        match help_item {
            HelpItem::Paragraph(text) => {
                for line in wrap(text, &C.wrap_options) {
                    writeln!(stdout, "{line}")?;
                }
            }
            HelpItem::Usage(args) => writeln!(stdout, "{}{}{}", as_title("Usage: "), name(), args)?,
            HelpItem::Section(s) => {
                writeln!(stdout, "{}", as_title(s.title))?;
                s.print_entries(stdout)?;
            }
        };
    }
    Ok(0)
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
                entries.push(Entry { item: as_item(item), caption });
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
    fn print_entries(self, stdout: &mut dyn std::io::Write) -> std::io::Result<usize> {
        let fits_in_line = self.entries.iter().all(Entry::fits_in_line);
        if fits_in_line {
            for entry in &self.entries {
                writeln!(stdout, "{}{}", entry.item, entry.caption)?;
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
                writeln!(stdout, "{line}")?;
            }
        }
        fn badness<T>(vv: &[Vec<T>]) -> usize {
            vv.iter().fold(0, |total, v| {
                let m = v.len().saturating_sub(2);
                total + v.len() + m * 2
            })
        }
        Ok(0)
    }
    fn next_line_help_indent(&self) -> &'a str {
        let max_indent =
            self.entries.iter().map(|e| e.item.indented_by()).fold(0, std::cmp::Ord::max);
        let indent_len = (max_indent + 4).min(BLANKS.len());
        &BLANKS[..indent_len]
    }
    fn next_line_help_lines(&self) -> Vec<Vec<Cow<'a, str>>> {
        let mut result = Vec::new();
        let indent = self.next_line_help_indent();
        for entry in &self.entries {
            result.push(vec![Cow::from(entry.item.to_string())]);
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
    fn fits_in_line(&self) -> bool {
        self.item.len() + self.caption.len() <= C.line_width
    }
    fn next_line_caption(&self, indent: &'a str) -> Vec<Cow<'a, str>> {
        wrap(self.caption, C.wrap_options.clone().initial_indent(indent).subsequent_indent(indent))
    }
    fn same_line_help(&self) -> Vec<Cow<'a, str>> {
        let first = &self.item.to_string();
        let rest = &BLANKS[..(self.item.len() + 4).min(BLANKS.len())];
        let options = C.wrap_options.clone().initial_indent(first).subsequent_indent(rest);
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

    Constants { line_width, wrap_options }
});
