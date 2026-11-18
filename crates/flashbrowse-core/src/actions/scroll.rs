//! Synthetic wheel/scroll events.

/// Direction for `scroll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    /// Parse `"up"|"down"|"left"|"right"` (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        if t.eq_ignore_ascii_case("up") {
            Some(Self::Up)
        } else if t.eq_ignore_ascii_case("down") {
            Some(Self::Down)
        } else if t.eq_ignore_ascii_case("left") {
            Some(Self::Left)
        } else if t.eq_ignore_ascii_case("right") {
            Some(Self::Right)
        } else {
            None
        }
    }

    /// `(delta_x, delta_y)` for the given pixel `amount`.
    pub fn delta(&self, amount: f64) -> (f64, f64) {
        match self {
            Self::Down => (0.0, amount),
            Self::Up => (0.0, -amount),
            Self::Right => (amount, 0.0),
            Self::Left => (-amount, 0.0),
        }
    }
}
