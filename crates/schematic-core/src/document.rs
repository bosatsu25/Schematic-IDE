use crate::{BlockEntityRef, DocumentMetadata, EntityRef, Region, RegionId};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentRevision(pub u64);

impl Default for DocumentRevision {
    fn default() -> Self {
        Self(1)
    }
}

impl DocumentRevision {
    pub const fn new(revision: u64) -> Self {
        Self(revision)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    metadata: DocumentMetadata,
    revision: DocumentRevision,
    regions: BTreeMap<RegionId, Region>,
    entities: Vec<EntityRef>,
    block_entities: Vec<BlockEntityRef>,
}

impl Document {
    pub fn new(metadata: DocumentMetadata) -> Self {
        Self {
            metadata,
            revision: DocumentRevision::new(1),
            regions: BTreeMap::new(),
            entities: Vec::new(),
            block_entities: Vec::new(),
        }
    }

    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }

    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }

    pub fn increment_revision(&mut self) -> DocumentRevision {
        self.revision = self.revision.next();
        self.revision
    }

    pub fn insert_region(&mut self, region: Region) -> Option<Region> {
        self.increment_revision();
        self.regions.insert(region.id().clone(), region)
    }

    pub fn region(&self, id: &RegionId) -> Option<&Region> {
        self.regions.get(id)
    }

    pub fn region_mut(&mut self, id: &RegionId) -> Option<&mut Region> {
        self.increment_revision();
        self.regions.get_mut(id)
    }

    pub fn regions(&self) -> impl Iterator<Item = &Region> {
        self.regions.values()
    }

    pub fn add_entity(&mut self, entity: EntityRef) {
        self.increment_revision();
        self.entities.push(entity);
    }

    pub fn entities(&self) -> impl Iterator<Item = &EntityRef> {
        self.entities.iter()
    }

    pub fn add_block_entity(&mut self, block_entity: BlockEntityRef) {
        self.increment_revision();
        self.block_entities.push(block_entity);
    }

    pub fn block_entities(&self) -> impl Iterator<Item = &BlockEntityRef> {
        self.block_entities.iter()
    }

    pub fn is_same_content(&self, other: &Self) -> bool {
        if self.regions.len() != other.regions.len() {
            return false;
        }
        for (id, r1) in &self.regions {
            let Some(r2) = other.regions.get(id) else {
                return false;
            };
            if r1.bounds() != r2.bounds() {
                return false;
            }
            let mut chunks = std::collections::BTreeSet::new();
            for (pos, _) in r1.chunks() {
                chunks.insert(pos);
            }
            for (pos, _) in r2.chunks() {
                chunks.insert(pos);
            }
            for pos in chunks {
                let c1 = r1.chunk(pos);
                let c2 = r2.chunk(pos);
                for idx in 0..crate::CHUNK_VOLUME {
                    let s1 = c1
                        .and_then(|c| c.get(idx).ok().flatten())
                        .and_then(|i| r1.palette().get(i));
                    let s2 = c2
                        .and_then(|c| c.get(idx).ok().flatten())
                        .and_then(|i| r2.palette().get(i));
                    if s1 != s2 {
                        return false;
                    }
                }
            }
        }
        true
    }
}
