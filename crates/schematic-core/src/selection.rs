use crate::{Bounds, Position};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    bounds: Bounds,
}

impl Selection {
    pub fn from_corners(first: Position, second: Position) -> Self {
        let minimum = Position::new(
            first.x.min(second.x),
            first.y.min(second.y),
            first.z.min(second.z),
        );
        let maximum = [
            first.x.max(second.x) as i64 + 1,
            first.y.max(second.y) as i64 + 1,
            first.z.max(second.z) as i64 + 1,
        ];
        Self {
            bounds: Bounds::new(minimum, maximum),
        }
    }

    pub const fn bounds(self) -> Bounds {
        self.bounds
    }

    pub fn contains(self, position: Position) -> bool {
        self.bounds.contains(position)
    }

    pub fn intersection(self, other: &Self) -> Option<Self> {
        self.bounds
            .intersection(other.bounds)
            .map(|bounds| Self { bounds })
    }
}
