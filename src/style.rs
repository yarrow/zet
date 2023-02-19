use once_cell::sync::Lazy;
use std::sync::Mutex;

pub enum ColorChoice {
    Auto,
    Always,
    Never,
}
#[derive(Debug, Clone, Copy)]
pub struct Styles {
    app_start: &'static str,
    item_start: &'static str,
    title_start: &'static str,
    end: &'static str,
}

pub fn app_name(s: &str) -> String {
    let current = CHOSEN.lock().unwrap();
    format!("{}{}{}", current.app_start, s, current.end)
}
pub fn item(s: &str) -> String {
    let current = CHOSEN.lock().unwrap();
    format!("{}{}{}", current.item_start, s, current.end)
}
pub fn title(s: &str) -> String {
    let current = CHOSEN.lock().unwrap();
    format!("{}{}{}", current.title_start, s, current.end)
}

const ALWAYS: Styles = Styles {
    app_start: "\x1B[32;1m", // bold green
    item_start: "\x1B[32m",  // green
    title_start: "\x1B[33m", // yellow
    end: "\x1B[m",           // change back to normal text
};
const NEVER: Styles = Styles {
    app_start: "",
    item_start: "",
    title_start: "",
    end: "",
};
static AUTO: Lazy<Styles> = Lazy::new(|| {
    use enable_ansi_support::enable_ansi_support;
    use supports_color::Stream;
    let use_color = enable_ansi_support().is_ok() && supports_color::on(Stream::Stdout).is_some();
    if use_color {
        ALWAYS
    } else {
        NEVER
    }
});

static CHOSEN: Mutex<Styles> = Mutex::new(NEVER);
pub fn init() {
    Lazy::force(&AUTO);
}
pub fn set_color_choice(cc: ColorChoice) {
    *CHOSEN.lock().unwrap() = match cc {
        ColorChoice::Always => ALWAYS,
        ColorChoice::Never => NEVER,
        ColorChoice::Auto => *Lazy::<Styles>::get(&AUTO).unwrap_or(&NEVER),
    };
}
