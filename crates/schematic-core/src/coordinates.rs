use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn coordinates(self) -> [i32; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlockPosition {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl BlockPosition {
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    pub const fn coordinates(self) -> [i64; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkPosition {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl ChunkPosition {
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    pub fn from_block_position(position: BlockPosition) -> Self {
        Self::new(
            position.x.div_euclid(CHUNK_EDGE as i64),
            position.y.div_euclid(CHUNK_EDGE as i64),
            position.z.div_euclid(CHUNK_EDGE as i64),
        )
    }

    pub fn index_for_block(self, position: BlockPosition) -> Option<usize> {
        if Self::from_block_position(position) != self {
            return None;
        }
        let x = position.x.rem_euclid(CHUNK_EDGE as i64) as usize;
        let y = position.y.rem_euclid(CHUNK_EDGE as i64) as usize;
        let z = position.z.rem_euclid(CHUNK_EDGE as i64) as usize;
        Some(Chunk::linear_index(x, y, z))
    }

    pub fn block_position_for_index(self, index: usize) -> Option<BlockPosition> {
        if index >= CHUNK_VOLUME {
            return None;
        }
        let (x, y, z) = Chunk::coordinates_for_index(index);
        let edge = CHUNK_EDGE as i64;
        Some(BlockPosition::new(
            self.x.checked_mul(edge)?.checked_add(x as i64)?,
            self.y.checked_mul(edge)?.checked_add(y as i64)?,
            self.z.checked_mul(edge)?.checked_add(z as i64)?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Size {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Size {
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    pub const fn is_empty(self) -> bool {
        self.x == 0 || self.y == 0 || self.z == 0
    }

    pub const fn dimensions(self) -> [u32; 3] {
        [self.x, self.y, self.z]
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bounds {
    min: Position,
    max_exclusive: [i64; 3],
}

impl Bounds {
    pub const fn new(min: Position, max_exclusive: [i64; 3]) -> Self {
        Self { min, max_exclusive }
    }

    pub const fn min(self) -> Position {
        self.min
    }

    pub const fn max_exclusive(self) -> [i64; 3] {
        self.max_exclusive
    }

    pub fn axis(self, axis: usize) -> Option<Range<i64>> {
        let minimum = *self.min.coordinates().get(axis)? as i64;
        Some(minimum..self.max_exclusive[axis])
    }

    pub const fn is_empty(self) -> bool {
        self.max_exclusive[0] <= self.min.x as i64
            || self.max_exclusive[1] <= self.min.y as i64
            || self.max_exclusive[2] <= self.min.z as i64
    }

    pub fn size(self) -> Size {
        if self.is_empty() {
            Size::new(0, 0, 0)
        } else {
            Size::new(
                (self.max_exclusive[0] - self.min.x as i64) as u32,
                (self.max_exclusive[1] - self.min.y as i64) as u32,
                (self.max_exclusive[2] - self.min.z as i64) as u32,
            )
        }
    }

    pub fn from_inclusive(first: Position, second: Position) -> Self {
        let min = Position::new(
            first.x.min(second.x),
            first.y.min(second.y),
            first.z.min(second.z),
        );
        let max_exclusive = [
            first.x.max(second.x) as i64 + 1,
            first.y.max(second.y) as i64 + 1,
            first.z.max(second.z) as i64 + 1,
        ];
        Self { min, max_exclusive }
    }

    pub fn between(first: Position, second: Position) -> Option<Self> {
        let min = Position::new(
            first.x.min(second.x),
            first.y.min(second.y),
            first.z.min(second.z),
        );
        let max_exclusive = [
            first.x.max(second.x) as i64,
            first.y.max(second.y) as i64,
            first.z.max(second.z) as i64,
        ];
        let bounds = Self::new(min, max_exclusive);
        (!bounds.is_empty()).then_some(bounds)
    }

    pub fn contains(self, position: Position) -> bool {
        !self.is_empty()
            && position.x >= self.min.x
            && (position.x as i64) < self.max_exclusive[0]
            && position.y >= self.min.y
            && (position.y as i64) < self.max_exclusive[1]
            && position.z >= self.min.z
            && (position.z as i64) < self.max_exclusive[2]
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let left_min = self.min.coordinates();
        let right_min = other.min.coordinates();
        let minimum = Position::new(
            left_min[0].max(right_min[0]),
            left_min[1].max(right_min[1]),
            left_min[2].max(right_min[2]),
        );
        let maximum = [
            self.max_exclusive[0].min(other.max_exclusive[0]),
            self.max_exclusive[1].min(other.max_exclusive[1]),
            self.max_exclusive[2].min(other.max_exclusive[2]),
        ];
        let bounds = Self::new(minimum, maximum);
        (!bounds.is_empty()).then_some(bounds)
    }
}

use crate::{Chunk, CHUNK_EDGE, CHUNK_VOLUME};
