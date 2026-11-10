use crate::{BlockPosition, NbtTag, RegionId};
use std::collections::BTreeMap;

/// An NBT compound payload.
pub type NbtCompound = BTreeMap<String, NbtTag>;

/// A block entity (chest contents, sign text, spawner data, ...).
///
/// Positions are **region-local** (relative to the owning region's minimum
/// corner), the same coordinate space used by [`crate::Region::block_index_at`].
/// `data` holds the full payload minus the identifier and coordinate keys, so
/// format adapters can re-emit it losslessly in their own layout.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockEntityRef {
    pub region: RegionId,
    pub position: BlockPosition,
    pub kind: String,
    pub data: NbtCompound,
}

impl BlockEntityRef {
    pub fn new(region: RegionId, kind: impl Into<String>, position: BlockPosition) -> Self {
        Self {
            region,
            position,
            kind: kind.into(),
            data: NbtCompound::new(),
        }
    }

    pub fn with_data(mut self, data: NbtCompound) -> Self {
        self.data = data;
        self
    }
}

/// A free-moving entity (mob, armor stand, item frame, minecart, ...).
///
/// `position` is region-local and continuous (block `(0,0,0)` spans
/// `[0,1)` on every axis). `data` holds the full payload minus the identifier
/// and `Pos`; orientation lives in `data["Rotation"]` as in vanilla NBT.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityRef {
    pub region: RegionId,
    pub position: [f64; 3],
    pub kind: String,
    pub data: NbtCompound,
}

impl EntityRef {
    pub fn new(region: RegionId, kind: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            region,
            position,
            kind: kind.into(),
            data: NbtCompound::new(),
        }
    }

    pub fn with_data(mut self, data: NbtCompound) -> Self {
        self.data = data;
        self
    }

    /// The region-local block cell containing this entity.
    pub fn block_position(&self) -> BlockPosition {
        BlockPosition::new(
            self.position[0].floor() as i64,
            self.position[1].floor() as i64,
            self.position[2].floor() as i64,
        )
    }
}
