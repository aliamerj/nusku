pub const Symbol = struct {
    /// Index of the section the symbol lives in.
    section: usize,
    /// Offset of the symbol inside that section.
    value: usize,
    size: usize,
};
