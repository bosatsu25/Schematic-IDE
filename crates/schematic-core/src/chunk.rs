use crate::{BlockPosition, PaletteIndex};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const CHUNK_EDGE: usize = 16;
pub const CHUNK_VOLUME: usize = CHUNK_EDGE * CHUNK_EDGE * CHUNK_EDGE;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chunk {
    blocks: Vec<Option<PaletteIndex>>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            blocks: vec![None; CHUNK_VOLUME],
        }
    }
}

impl Chunk {
    pub fn get(&self, index: usize) -> Result<Option<PaletteIndex>, ChunkIndexError> {
        self.blocks
            .get(index)
            .copied()
            .ok_or(ChunkIndexError(index))
    }

    pub fn set(
        &mut self,
        index: usize,
        palette_index: Option<PaletteIndex>,
    ) -> Result<Option<PaletteIndex>, ChunkIndexError> {
        let block = self.blocks.get_mut(index).ok_or(ChunkIndexError(index))?;
        Ok(std::mem::replace(block, palette_index))
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.iter().all(Option::is_none)
    }

    pub fn occupied_blocks(&self) -> impl Iterator<Item = (usize, PaletteIndex)> + '_ {
        self.blocks
            .iter()
            .enumerate()
            .filter_map(|(index, state)| state.map(|state| (index, state)))
    }

    pub fn index_for(position: BlockPosition) -> Option<usize> {
        let edge = CHUNK_EDGE as i64;
        if !(0..edge).contains(&position.x)
            || !(0..edge).contains(&position.y)
            || !(0..edge).contains(&position.z)
        {
            return None;
        }
        Some(Self::linear_index(
            position.x as usize,
            position.y as usize,
            position.z as usize,
        ))
    }

    pub(crate) fn linear_index(x: usize, y: usize, z: usize) -> usize {
        x + CHUNK_EDGE * z + CHUNK_EDGE * CHUNK_EDGE * y
    }

    pub(crate) fn coordinates_for_index(index: usize) -> (usize, usize, usize) {
        let x = index % CHUNK_EDGE;
        let z = (index / CHUNK_EDGE) % CHUNK_EDGE;
        let y = index / (CHUNK_EDGE * CHUNK_EDGE);
        (x, y, z)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkIndexError(pub usize);

impl Display for ChunkIndexError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "chunk index {} is outside 0..{CHUNK_VOLUME}",
            self.0
        )
    }
}

impl Error for ChunkIndexError {}
