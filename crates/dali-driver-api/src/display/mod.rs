//! Hardware-neutral, bounded display contracts.

mod console;
mod dimensions;
mod driver;
mod text;

pub use console::DiagnosticsConsole;
pub use dimensions::DisplayDimensions;
pub use driver::DisplayDriver;
pub use text::{TextModeProperties, TextPosition};
