use anstyle::{AnsiColor, Color, Style};
use clap::ValueEnum;
use std::fmt;

#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}
const GREEN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));
const BOLD_GREEN: Style = GREEN.bold();
const YELLOW: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

#[must_use]
pub(crate) fn app_name(content: &str) -> StyledStr<'_> {
    StyledStr { prefix: BOLD_GREEN, content }
}
#[must_use]
pub(crate) fn as_item(content: &str) -> StyledStr<'_> {
    StyledStr { prefix: GREEN, content }
}
#[must_use]
pub(crate) fn as_title(content: &str) -> StyledStr<'_> {
    StyledStr { prefix: YELLOW, content }
}

pub(crate) struct StyledStr<'a> {
    prefix: Style,
    content: &'a str,
}
impl StyledStr<'_> {
    #[must_use]
    pub fn len(&self) -> usize {
        self.content.len()
    }
    #[must_use]
    pub fn indented_by(&self) -> usize {
        use bstr::ByteSlice;
        self.content.as_bytes().find_not_byteset(b" ").unwrap_or(self.len())
    }
}
impl fmt::Display for StyledStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.prefix.render(), self.content, self.prefix.render_reset())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_len() {
        let contents = "abc";
        assert_eq!(app_name(contents).len(), contents.len());
        assert_eq!(as_item(contents).len(), contents.len());
        assert_eq!(as_title(contents).len(), contents.len());
    }
}
