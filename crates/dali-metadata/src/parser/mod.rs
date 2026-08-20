//! Strict, bounded parser for canonical metadata bodies.

mod cursor;
mod root;
mod targets;

#[cfg(test)]
mod tests;

pub use root::parse_root_signed;
pub use targets::parse_targets_signed;

/// Errors returned by the canonical metadata parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The input ended before a complete value was available.
    UnexpectedEnd,
    /// A punctuation or field token did not match the canonical grammar.
    UnexpectedToken,
    /// A string contains invalid UTF-8 or forbidden escape/control bytes.
    InvalidString,
    /// A number is not a canonical unsigned integer.
    InvalidNumber,
    /// A fixed-width hexadecimal value is malformed.
    InvalidHex,
    /// A field name is unknown or appears out of canonical order.
    InvalidFieldOrder,
    /// A fixed-capacity record list is full.
    TooManyRecords,
    /// A value violates the metadata contract.
    InvalidValue,
    /// Bytes remain after one complete document.
    TrailingBytes,
}
