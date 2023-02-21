use once_cell::sync::Lazy;

pub enum ColorChoice {
    Auto,
    Always,
    Never,
}
#[derive(Debug, Clone, Copy)]
pub struct StyleSheet {
    app_start: &'static str,
    item_start: &'static str,
    title_start: &'static str,
    end: &'static str,
}

impl StyleSheet {
    pub fn app_name(&self, s: &str) -> String {
        format!("{}{}{}", self.app_start, s, self.end)
    }
    pub fn item(&self, s: &str) -> String {
        format!("{}{}{}", self.item_start, s, self.end)
    }
    pub fn title(&self, s: &str) -> String {
        format!("{}{}{}", self.title_start, s, self.end)
    }
}

const ESC: u8 = b'\x1B';
const GREEN: &str = "\x1B[32m";
const BOLD_GREEN: &str = "\x1B[32;1m";
const YELLOW: &str = "\x1B[33m";
const RESET: &str = "\x1B[m";
const ALWAYS: StyleSheet = StyleSheet {
    app_start: BOLD_GREEN,
    item_start: GREEN,
    title_start: YELLOW,
    end: RESET,
};
const NEVER: StyleSheet = StyleSheet {
    app_start: "",
    item_start: "",
    title_start: "",
    end: "",
};
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

pub fn colored(cc: ColorChoice) -> &'static StyleSheet {
    match cc {
        ColorChoice::Always => &ALWAYS,
        ColorChoice::Never => &NEVER,
        ColorChoice::Auto => Lazy::<StyleSheet>::get(&AUTO).unwrap_or(&NEVER),
    }
}

pub fn display_width(s: &str) -> usize {
    use bstr::ByteSlice;
    let s = s.as_bytes();
    if s.len() < GREEN.len() + RESET.len() {
        return s.len();
    }
    if s[0] == ESC && s[1] == b'[' && s.ends_with_str(RESET) {
        match s.find_byte(b'm') {
            Some(m) => s.len() - (m + 1 + RESET.len()),
            None => s.len(),
        }
    } else {
        s.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_display_width() {
        let contents = "abc";
        for choice in [ALWAYS, NEVER] {
            assert_eq!(display_width(&choice.app_name(&contents)), contents.len());
            assert_eq!(display_width(&choice.item(&contents)), contents.len());
            assert_eq!(display_width(&choice.title(&contents)), contents.len());
        }
    }
}
