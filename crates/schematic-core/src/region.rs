use crate::{
    BlockPosition, Bounds, Chunk, ChunkIndexError, ChunkPosition, Palette, PaletteIndex, Position,
    Size,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegionId(String);

impl RegionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    id: RegionId,
    origin: Position,
    size: Size,
    palette: Palette,
    chunks: BTreeMap<ChunkPosition, Chunk>,
}

impl Region {
    pub fn new(id: RegionId, origin: Position, size: Size) -> Self {
        Self {
            id,
            origin,
            size,
            palette: Palette::new(),
            chunks: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> &RegionId {
        &self.id
    }

    pub const fn origin(&self) -> Position {
        self.origin
    }

    pub const fn size(&self) -> Size {
        self.size
    }

    pub fn bounds(&self) -> Bounds {
        Bounds::new(
            self.origin,
            [
                self.origin.x as i64 + self.size.x as i64,
                self.origin.y as i64 + self.size.y as i64,
                self.origin.z as i64 + self.size.z as i64,
            ],
        )
    }

    pub const fn is_empty(&self) -> bool {
        self.size.is_empty()
    }

    pub fn contains(&self, world_position: Position) -> bool {
        self.bounds().contains(world_position)
    }

    pub fn local_to_world(&self, local_position: BlockPosition) -> Option<Position> {
        if local_position.x < 0
            || local_position.y < 0
            || local_position.z < 0
            || local_position.x >= self.size.x as i64
            || local_position.y >= self.size.y as i64
            || local_position.z >= self.size.z as i64
        {
            return None;
        }

        Some(Position::new(
            i32::try_from(self.origin.x as i64 + local_position.x).ok()?,
            i32::try_from(self.origin.y as i64 + local_position.y).ok()?,
            i32::try_from(self.origin.z as i64 + local_position.z).ok()?,
        ))
    }

    pub fn world_to_local(&self, world_position: Position) -> Option<BlockPosition> {
        if !self.contains(world_position) {
            return None;
        }
        Some(BlockPosition::new(
            world_position.x as i64 - self.origin.x as i64,
            world_position.y as i64 - self.origin.y as i64,
            world_position.z as i64 - self.origin.z as i64,
        ))
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn palette_mut(&mut self) -> &mut Palette {
        &mut self.palette
    }

    pub fn chunk(&self, position: ChunkPosition) -> Option<&Chunk> {
        self.chunks.get(&position)
    }

    pub fn chunk_mut(&mut self, position: ChunkPosition) -> &mut Chunk {
        self.chunks.entry(position).or_default()
    }

    pub fn block_index_at(
        &self,
        local_position: BlockPosition,
    ) -> Result<Option<PaletteIndex>, RegionBlockError> {
        let (chunk_position, index) = self.chunk_coordinates(local_position)?;
        match self.chunks.get(&chunk_position) {
            Some(chunk) => chunk
                .get(index)
                .map_err(RegionBlockError::InvalidChunkIndex),
            None => Ok(None),
        }
    }

    pub fn set_block_index(
        &mut self,
        local_position: BlockPosition,
        palette_index: Option<PaletteIndex>,
    ) -> Result<Option<PaletteIndex>, RegionBlockError> {
        if let Some(index) = palette_index {
            if self.palette.get(index).is_none() {
                return Err(RegionBlockError::UnknownPaletteIndex(index));
            }
        }
        let (chunk_position, index) = self.chunk_coordinates(local_position)?;
        if palette_index.is_none() && !self.chunks.contains_key(&chunk_position) {
            return Ok(None);
        }
        let (previous, remove_chunk) = {
            let chunk = self.chunks.entry(chunk_position).or_default();
            let previous = chunk
                .set(index, palette_index)
                .map_err(RegionBlockError::InvalidChunkIndex)?;
            (previous, chunk.is_empty())
        };
        if remove_chunk {
            self.chunks.remove(&chunk_position);
        }
        Ok(previous)
    }

    pub fn chunks(&self) -> impl Iterator<Item = (ChunkPosition, &Chunk)> {
        self.chunks
            .iter()
            .map(|(position, chunk)| (*position, chunk))
    }

    fn chunk_coordinates(
        &self,
        local_position: BlockPosition,
    ) -> Result<(ChunkPosition, usize), RegionBlockError> {
        if local_position.x < 0
            || local_position.y < 0
            || local_position.z < 0
            || local_position.x >= self.size.x as i64
            || local_position.y >= self.size.y as i64
            || local_position.z >= self.size.z as i64
        {
            return Err(RegionBlockError::OutsideRegion);
        }
        let chunk_position = ChunkPosition::from_block_position(local_position);
        let index = chunk_position
            .index_for_block(local_position)
            .ok_or(RegionBlockError::OutsideRegion)?;
        Ok((chunk_position, index))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionBlockError {
    OutsideRegion,
    UnknownPaletteIndex(PaletteIndex),
    InvalidChunkIndex(ChunkIndexError),
}

impl Display for RegionBlockError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutsideRegion => formatter.write_str("block position is outside the region"),
            Self::UnknownPaletteIndex(index) => {
                write!(formatter, "palette index {} does not exist", index.get())
            }
            Self::InvalidChunkIndex(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for RegionBlockError {}
