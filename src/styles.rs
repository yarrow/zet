use clap::ValueEnum;
use enable_ansi_support::enable_ansi_support;
use once_cell::sync::OnceCell;
use std::fmt;

#[derive(Debug, Clone, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}
#[derive(Debug, Clone, Copy)]
pub struct StyleSheet {
    app_prefix: Option<&'static str>,
    item_prefix: Option<&'static str>,
    title_prefix: Option<&'static str>,
    error_prefix: Option<&'static str>,
    literal_prefix: Option<&'static str>,
}
impl StyleSheet {
    #[must_use]
    pub fn app_name<'a>(&self, content: &'a str) -> StyledStr<'a> {
        StyledStr { prefix: self.app_prefix, content }
    }
    #[must_use]
    pub fn item<'a>(&self, content: &'a str) -> StyledStr<'a> {
        StyledStr { prefix: self.item_prefix, content }
    }
    #[must_use]
    pub fn title<'a>(&self, content: &'a str) -> StyledStr<'a> {
        StyledStr { prefix: self.title_prefix, content }
    }
    #[must_use]
    pub fn error<'a>(&self, content: &'a str) -> StyledStr<'a> {
        StyledStr { prefix: self.error_prefix, content }
    }
    #[must_use]
    pub fn literal<'a>(&self, content: &'a str) -> StyledStr<'a> {
        StyledStr { prefix: self.literal_prefix, content }
    }
}

pub struct StyledStr<'a> {
    prefix: Option<&'static str>,
    content: &'a str,
}
impl<'a> StyledStr<'a> {
    #[must_use]
    pub fn len(&self) -> usize {
        self.content.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    #[must_use]
    pub fn indented_by(&self) -> usize {
        use bstr::ByteSlice;
        self.content.as_bytes().find_not_byteset(b" ").unwrap_or(self.len())
    }
}
impl<'a> fmt::Display for StyledStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(prefix) = self.prefix {
            write!(f, "{}{}{}", prefix, self.content, RESET)
        } else {
            write!(f, "{}", self.content)
        }
    }
}

const BOLD_RED: &str = "\x1B[31;1m";
const GREEN: &str = "\x1B[32m";
const BOLD_GREEN: &str = "\x1B[32;1m";
const YELLOW: &str = "\x1B[33m";
const RESET: &str = "\x1B[m";

const NEVER: StyleSheet = StyleSheet {
    app_prefix: None,
    item_prefix: None,
    title_prefix: None,
    error_prefix: None,
    literal_prefix: None,
};
const ALWAYS: StyleSheet = StyleSheet {
    app_prefix: Some(BOLD_GREEN),
    item_prefix: Some(GREEN),
    title_prefix: Some(YELLOW),
    error_prefix: Some(BOLD_RED),
    literal_prefix: Some(YELLOW),
};
fn auto() -> StyleSheet {
    use supports_color::Stream;
    let use_color = enable_ansi_support().is_ok() && supports_color::on(Stream::Stdout).is_some();
    if use_color {
        ALWAYS
    } else {
        NEVER
    }
}

static COLOR_CHOICE: OnceCell<ColorChoice> = OnceCell::new();
static STYLE_SHEET: OnceCell<StyleSheet> = OnceCell::new();
pub(crate) fn set_color_choice(cc: ColorChoice) {
    COLOR_CHOICE.set(cc).expect("set_color_choice may only be called once");
}
pub(crate) fn global_style() -> &'static StyleSheet {
    if let Some(style) = STYLE_SHEET.get() {
        style
    } else {
        let cc = COLOR_CHOICE.get().expect("Please call set_color_choice before calling style");
        let style = match cc {
            ColorChoice::Always => {
                let _ = enable_ansi_support();
                ALWAYS
            }
            ColorChoice::Never => NEVER,
            ColorChoice::Auto => auto(),
        };
        let _ = STYLE_SHEET.set(style); // .set only errors if the value was already set
        STYLE_SHEET.get().expect("This can't happen: STYLE_SHEET was unset after being set")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_len() {
        let contents = "abc";
        for choice in [ALWAYS, NEVER] {
            assert_eq!(choice.app_name(contents).len(), contents.len());
            assert_eq!(choice.item(contents).len(), contents.len());
            assert_eq!(choice.title(contents).len(), contents.len());
        }
    }
}
