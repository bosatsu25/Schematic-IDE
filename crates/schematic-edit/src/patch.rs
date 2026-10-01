use schematic_core::{
    BlockPosition, BlockState, ChunkPosition, Document, PaletteIndex, RegionBlockError, RegionId,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockChange {
    position: BlockPosition,
    before: Option<PaletteIndex>,
    after: Option<PaletteIndex>,
}

impl BlockChange {
    pub(crate) const fn new(
        position: BlockPosition,
        before: Option<PaletteIndex>,
        after: Option<PaletteIndex>,
    ) -> Self {
        Self {
            position,
            before,
            after,
        }
    }

    pub const fn position(self) -> BlockPosition {
        self.position
    }

    pub const fn before(self) -> Option<PaletteIndex> {
        self.before
    }

    pub const fn after(self) -> Option<PaletteIndex> {
        self.after
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Patch {
    region_id: RegionId,
    chunk_position: ChunkPosition,
    changes: Vec<BlockChange>,
}

impl Patch {
    pub fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    pub const fn chunk_position(&self) -> ChunkPosition {
        self.chunk_position
    }

    pub fn changes(&self) -> impl Iterator<Item = &BlockChange> {
        self.changes.iter()
    }

    pub fn changed_block_count(&self) -> usize {
        self.changes.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchSet {
    patches: Vec<Patch>,
    palette_additions: Vec<PaletteAddition>,
}

impl PatchSet {
    pub(crate) fn empty() -> Self {
        Self {
            patches: Vec::new(),
            palette_additions: Vec::new(),
        }
    }

    pub(crate) fn from_changes(
        region_id: RegionId,
        changes: impl IntoIterator<Item = BlockChange>,
        palette_addition: Option<(PaletteIndex, BlockState)>,
    ) -> Self {
        let mut grouped = BTreeMap::<(ChunkPosition, RegionId), Vec<BlockChange>>::new();
        for change in changes {
            let chunk_position = ChunkPosition::from_block_position(change.position);
            grouped
                .entry((chunk_position, region_id.clone()))
                .or_default()
                .push(change);
        }
        let patches = grouped
            .into_iter()
            .map(|((chunk_position, region_id), changes)| Patch {
                region_id,
                chunk_position,
                changes,
            })
            .collect();
        let palette_additions = palette_addition
            .map(|(index, state)| {
                vec![PaletteAddition {
                    region_id,
                    index,
                    state,
                }]
            })
            .unwrap_or_default();
        Self {
            patches,
            palette_additions,
        }
    }

    pub fn patches(&self) -> impl Iterator<Item = &Patch> {
        self.patches.iter()
    }

    pub fn changed_block_count(&self) -> usize {
        self.patches.iter().map(Patch::changed_block_count).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    pub fn apply(&self, document: &mut Document) -> Result<(), PatchError> {
        self.transition(document, Direction::Forward)
    }

    pub fn revert(&self, document: &mut Document) -> Result<(), PatchError> {
        self.transition(document, Direction::Reverse)
    }

    fn transition(&self, document: &mut Document, direction: Direction) -> Result<(), PatchError> {
        self.validate(document, direction)?;

        if direction == Direction::Forward {
            for addition in &self.palette_additions {
                let region = document
                    .region_mut(&addition.region_id)
                    .ok_or_else(|| PatchError::MissingRegion(addition.region_id.clone()))?;
                if region.palette().get(addition.index).is_none() {
                    let inserted = region
                        .palette_mut()
                        .intern(addition.state.clone())
                        .map_err(|_| PatchError::PaletteConflict {
                            region_id: addition.region_id.clone(),
                            index: addition.index,
                        })?;
                    if inserted != addition.index {
                        return Err(PatchError::PaletteConflict {
                            region_id: addition.region_id.clone(),
                            index: addition.index,
                        });
                    }
                }
            }
        }

        for patch in &self.patches {
            for change in &patch.changes {
                let region = document
                    .region_mut(&patch.region_id)
                    .ok_or_else(|| PatchError::MissingRegion(patch.region_id.clone()))?;
                region
                    .set_block_index(
                        change.position,
                        direction.target(change.before, change.after),
                    )
                    .map_err(PatchError::RegionBlock)?;
            }
        }
        Ok(())
    }

    fn validate(&self, document: &Document, direction: Direction) -> Result<(), PatchError> {
        for patch in &self.patches {
            let region = document
                .region(&patch.region_id)
                .ok_or_else(|| PatchError::MissingRegion(patch.region_id.clone()))?;
            for change in &patch.changes {
                if ChunkPosition::from_block_position(change.position) != patch.chunk_position {
                    return Err(PatchError::InvalidChunkGrouping);
                }
                let actual = region
                    .block_index_at(change.position)
                    .map_err(PatchError::RegionBlock)?;
                let expected = direction.expected(change.before, change.after);
                if actual != expected {
                    return Err(PatchError::Conflict {
                        region_id: patch.region_id.clone(),
                        position: change.position,
                        expected,
                        actual,
                    });
                }
            }
        }

        for addition in &self.palette_additions {
            let region = document
                .region(&addition.region_id)
                .ok_or_else(|| PatchError::MissingRegion(addition.region_id.clone()))?;
            match region.palette().get(addition.index) {
                Some(state) if state == &addition.state => {}
                Some(_) => {
                    return Err(PatchError::PaletteConflict {
                        region_id: addition.region_id.clone(),
                        index: addition.index,
                    });
                }
                None if direction == Direction::Forward
                    && region.palette().len() == addition.index.get() as usize
                    && region.palette().index_of(&addition.state).is_none() => {}
                None => {
                    return Err(PatchError::PaletteConflict {
                        region_id: addition.region_id.clone(),
                        index: addition.index,
                    });
                }
            }
        }

        for patch in &self.patches {
            let region = document
                .region(&patch.region_id)
                .ok_or_else(|| PatchError::MissingRegion(patch.region_id.clone()))?;
            for change in &patch.changes {
                if let Some(target) = direction.target(change.before, change.after) {
                    let addition_exists = self.palette_additions.iter().any(|addition| {
                        addition.region_id == patch.region_id && addition.index == target
                    });
                    if region.palette().get(target).is_none() && !addition_exists {
                        return Err(PatchError::UnknownPaletteIndex {
                            region_id: patch.region_id.clone(),
                            index: target,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PaletteAddition {
    region_id: RegionId,
    index: PaletteIndex,
    state: BlockState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Forward,
    Reverse,
}

impl Direction {
    const fn expected(
        self,
        before: Option<PaletteIndex>,
        after: Option<PaletteIndex>,
    ) -> Option<PaletteIndex> {
        match self {
            Self::Forward => before,
            Self::Reverse => after,
        }
    }

    const fn target(
        self,
        before: Option<PaletteIndex>,
        after: Option<PaletteIndex>,
    ) -> Option<PaletteIndex> {
        match self {
            Self::Forward => after,
            Self::Reverse => before,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatchError {
    MissingRegion(RegionId),
    Conflict {
        region_id: RegionId,
        position: BlockPosition,
        expected: Option<PaletteIndex>,
        actual: Option<PaletteIndex>,
    },
    UnknownPaletteIndex {
        region_id: RegionId,
        index: PaletteIndex,
    },
    PaletteConflict {
        region_id: RegionId,
        index: PaletteIndex,
    },
    InvalidChunkGrouping,
    RegionBlock(RegionBlockError),
}

impl Display for PatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRegion(id) => write!(formatter, "region '{}' does not exist", id.as_str()),
            Self::Conflict {
                region_id,
                position,
                expected,
                actual,
            } => write!(
                formatter,
                "patch conflict in region '{}' at ({}, {}, {}): expected {expected:?}, found {actual:?}",
                region_id.as_str(),
                position.x,
                position.y,
                position.z
            ),
            Self::UnknownPaletteIndex { region_id, index } => write!(
                formatter,
                "palette index {} does not exist in region '{}'",
                index.get(),
                region_id.as_str()
            ),
            Self::PaletteConflict { region_id, index } => write!(
                formatter,
                "palette entry {} changed in region '{}'",
                index.get(),
                region_id.as_str()
            ),
            Self::InvalidChunkGrouping => formatter.write_str("patch contains a block in the wrong chunk"),
            Self::RegionBlock(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for PatchError {}

impl From<RegionBlockError> for PatchError {
    fn from(error: RegionBlockError) -> Self {
        Self::RegionBlock(error)
    }
}
