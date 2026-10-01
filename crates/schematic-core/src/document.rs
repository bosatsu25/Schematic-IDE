use crate::{BlockEntityRef, DocumentMetadata, EntityRef, Region, RegionId};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    metadata: DocumentMetadata,
    regions: BTreeMap<RegionId, Region>,
    entities: Vec<EntityRef>,
    block_entities: Vec<BlockEntityRef>,
}

impl Document {
    pub fn new(metadata: DocumentMetadata) -> Self {
        Self {
            metadata,
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

    pub fn insert_region(&mut self, region: Region) -> Option<Region> {
        self.regions.insert(region.id().clone(), region)
    }

    pub fn region(&self, id: &RegionId) -> Option<&Region> {
        self.regions.get(id)
    }

    pub fn region_mut(&mut self, id: &RegionId) -> Option<&mut Region> {
        self.regions.get_mut(id)
    }

    pub fn regions(&self) -> impl Iterator<Item = &Region> {
        self.regions.values()
    }

    pub fn add_entity(&mut self, entity: EntityRef) {
        self.entities.push(entity);
    }

    pub fn entities(&self) -> impl Iterator<Item = &EntityRef> {
        self.entities.iter()
    }

    pub fn add_block_entity(&mut self, block_entity: BlockEntityRef) {
        self.block_entities.push(block_entity);
    }

    pub fn block_entities(&self) -> impl Iterator<Item = &BlockEntityRef> {
        self.block_entities.iter()
    }
}
