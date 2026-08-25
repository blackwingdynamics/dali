//! Typed display geometry.

/// Compile-time display dimensions supplied by a target profile or adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayDimensions<const WIDTH: usize, const HEIGHT: usize>;

impl<const WIDTH: usize, const HEIGHT: usize> DisplayDimensions<WIDTH, HEIGHT> {
    /// Returns the configured pixel width.
    pub const fn width(self) -> usize {
        WIDTH
    }

    /// Returns the configured pixel height.
    pub const fn height(self) -> usize {
        HEIGHT
    }
}
