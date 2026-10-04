use schematic_core::Position;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VoxelFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl VoxelFace {
    pub const ALL: [Self; 6] = [
        Self::Down,
        Self::Up,
        Self::North,
        Self::South,
        Self::West,
        Self::East,
    ];

    pub const fn delta(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }

    pub fn neighbor_of(self, origin: Position) -> Option<Position> {
        let [dx, dy, dz] = self.delta();
        Some(Position::new(
            origin.x.checked_add(dx)?,
            origin.y.checked_add(dy)?,
            origin.z.checked_add(dz)?,
        ))
    }
}
