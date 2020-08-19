//! Text measurement.

/// A trait for measurement of text.
///
/// The client is expected to provide this.
pub trait Measurement {
    /// Report the number of visual lines for a logical line.
    fn n_visual_lines(&self, line_num: usize) -> usize;

    /// Report cursor position for an offset within the logical line.
    ///
    /// The `offset` argument is *relative* to the beginning of the
    /// logical line.
    ///
    /// The return value is a horizontal position and a relative
    /// visual line number.
    fn to_pos(&self, line_num: usize, offset: usize) -> (f64, usize);

    /// Find the closest location in the text corresponding to the
    /// given position.
    ///
    /// The return value is an offset relative to the beginning of the
    /// logical line.
    fn from_pos(&self, line_num: usize, horiz: f64, visual_line: usize) -> usize;
}
