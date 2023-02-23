use help_zet::help;
use help_zet::style::{self, ColorChoice};
fn main() {
    style::init();
    help::print(style::colored(ColorChoice::Auto));
}
