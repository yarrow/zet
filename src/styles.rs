//! Colors for `zet`'s help: `clap 4`'s help format, but `clap 3`'s colors.

use clap::builder::styling::{AnsiColor, Style, Styles};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}
impl From<ColorChoice> for clap::ColorChoice {
    fn from(choice: ColorChoice) -> Self {
        match choice {
            ColorChoice::Auto => clap::ColorChoice::Auto,
            ColorChoice::Always => clap::ColorChoice::Always,
            ColorChoice::Never => clap::ColorChoice::Never,
        }
    }
}
impl From<ColorChoice> for anstream::ColorChoice {
    fn from(choice: ColorChoice) -> Self {
        match choice {
            ColorChoice::Auto => anstream::ColorChoice::Auto,
            ColorChoice::Always => anstream::ColorChoice::Always,
            ColorChoice::Never => anstream::ColorChoice::Never,
        }
    }
}

const GREEN: Style = AnsiColor::Green.on_default();
const YELLOW: Style = AnsiColor::Yellow.on_default();

/// The style for `zet`'s own name
pub(crate) const APP_NAME: Style = GREEN.bold();
/// The style for section titles, like `Usage:` and `Options:`
pub(crate) const TITLE: Style = YELLOW;
/// The style for the names of commands and options
pub(crate) const ITEM: Style = GREEN;

/// The styles `clap` uses for help and error messages
pub(crate) const STYLES: Styles =
    Styles::styled().header(TITLE).usage(TITLE).literal(ITEM).placeholder(ITEM);
