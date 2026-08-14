//! Console I/O: prompts, headers, success/warning lines. Rust equivalent
//! of the legacy `src/Traits/ConsoleTools.php` + `tools/colors.sh`.

pub mod prompt;
pub mod style;

pub use prompt::Prompter;
pub use style::Style;
