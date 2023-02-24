use once_cell::sync::Lazy;
use std::fmt;

pub enum ColorChoice {
    Auto,
    Always,
    Never,
}
#[derive(Debug, Clone, Copy)]
pub struct StyleSheet {
    app_prefix: &'static str,
    item_prefix: &'static str,
    title_prefix: &'static str,
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
}

pub struct StyledStr<'a> {
    prefix: &'static str,
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
        if self.prefix.is_empty() {
            write!(f, "{}", self.content)
        } else {
            write!(f, "{}{}{}", self.prefix, self.content, RESET)
        }
    }
}

const GREEN: &str = "\x1B[32m";
const BOLD_GREEN: &str = "\x1B[32;1m";
const YELLOW: &str = "\x1B[33m";
const RESET: &str = "\x1B[m";
const ALWAYS: StyleSheet =
    StyleSheet { app_prefix: BOLD_GREEN, item_prefix: GREEN, title_prefix: YELLOW };
const NEVER: StyleSheet = StyleSheet { app_prefix: "", item_prefix: "", title_prefix: "" };
static AUTO: Lazy<StyleSheet> = Lazy::new(|| {
    use enable_ansi_support::enable_ansi_support;
    use supports_color::Stream;
    let use_color = enable_ansi_support().is_ok() && supports_color::on(Stream::Stdout).is_some();
    if use_color {
        ALWAYS
    } else {
        NEVER
    }
});

pub fn init() {
    Lazy::force(&AUTO);
}

#[must_use]
pub fn colored(cc: ColorChoice) -> &'static StyleSheet {
    match cc {
        ColorChoice::Always => &ALWAYS,
        ColorChoice::Never => &NEVER,
        ColorChoice::Auto => Lazy::<StyleSheet>::get(&AUTO).unwrap_or(&NEVER),
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
