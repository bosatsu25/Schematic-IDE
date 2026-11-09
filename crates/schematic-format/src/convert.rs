//! Cross-format conversion helpers: flattening multi-region documents for
//! single-volume formats and reporting what a conversion cannot represent.

use crate::litematic::FormatError;
use schematic_core::{BlockEntityRef, Document, EntityRef, Position, Region, RegionId, Size};

/// Result of flattening a document into a single region.
#[derive(Clone, Debug)]
pub struct MergedDocument {
    pub document: Document,
    /// Names of the source regions when more than one was merged.
    pub merged_region_names: Vec<String>,
    /// Non-air blocks of an earlier region overwritten by a later region
    /// holding a *different* block at the same world position.
    pub overlap_conflicts: usize,
}

/// Flattens every region of `doc` into one region spanning their union
/// bounding box. Regions are applied in id order; on overlap the later region
/// wins and the conflict is counted. Block entities and entities are re-based
/// to the merged region. Single-region documents are returned unchanged.
pub fn merge_regions(doc: &Document) -> Result<MergedDocument, FormatError> {
    let regions: Vec<&Region> = doc.regions().collect();
    if regions.len() <= 1 {
        return Ok(MergedDocument {
            document: doc.clone(),
            merged_region_names: Vec::new(),
            overlap_conflicts: 0,
        });
    }

    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for region in &regions {
        let o = region.origin();
        let s = region.size();
        let lo = [o.x as i64, o.y as i64, o.z as i64];
        let hi = [lo[0] + s.x as i64, lo[1] + s.y as i64, lo[2] + s.z as i64];
        for i in 0..3 {
            min[i] = min[i].min(lo[i]);
            max[i] = max[i].max(hi[i]);
        }
    }
    let to_u32 = |v: i64| {
        u32::try_from(v)
            .map_err(|_| FormatError::InvalidStructure("merged region is too large".to_string()))
    };
    let to_i32 = |v: i64| {
        i32::try_from(v).map_err(|_| {
            FormatError::InvalidStructure("merged region origin out of range".to_string())
        })
    };
    let origin = Position::new(to_i32(min[0])?, to_i32(min[1])?, to_i32(min[2])?);
    let size = Size::new(
        to_u32(max[0] - min[0])?,
        to_u32(max[1] - min[1])?,
        to_u32(max[2] - min[2])?,
    );
    let merged_id = RegionId::new("Main");
    let mut merged = Region::new(merged_id.clone(), origin, size);
    let mut overlap_conflicts = 0usize;

    for region in &regions {
        for (chunk_pos, chunk) in region.chunks() {
            for (index, palette_index) in chunk.occupied_blocks() {
                let Some(local) = chunk_pos.block_position_for_index(index) else {
                    continue;
                };
                let Some(world) = region.local_to_world(local) else {
                    continue;
                };
                let Some(state) = region.palette().get(palette_index) else {
                    continue;
                };
                let Some(target) = merged.world_to_local(world) else {
                    continue;
                };
                let new_index = merged
                    .palette_mut()
                    .intern(state.clone())
                    .map_err(|e| FormatError::InvalidStructure(format!("palette error: {e}")))?;
                if let Ok(Some(existing)) = merged.block_index_at(target) {
                    if existing != new_index {
                        overlap_conflicts += 1;
                    }
                }
                merged
                    .set_block_index(target, Some(new_index))
                    .map_err(|e| FormatError::InvalidStructure(format!("merge error: {e}")))?;
            }
        }
    }

    let mut document = Document::new(doc.metadata().clone());
    for be in doc.block_entities() {
        let Some(region) = doc.region(&be.region) else {
            continue;
        };
        let Some(world) = region.local_to_world(be.position) else {
            continue;
        };
        if let Some(local) = merged.world_to_local(world) {
            document.add_block_entity(
                BlockEntityRef::new(merged_id.clone(), be.kind.clone(), local)
                    .with_data(be.data.clone()),
            );
        }
    }
    for entity in doc.entities() {
        let Some(region) = doc.region(&entity.region) else {
            continue;
        };
        let ro = region.origin();
        let shift = [
            (ro.x - origin.x) as f64,
            (ro.y - origin.y) as f64,
            (ro.z - origin.z) as f64,
        ];
        document.add_entity(
            EntityRef::new(
                merged_id.clone(),
                entity.kind.clone(),
                [
                    entity.position[0] + shift[0],
                    entity.position[1] + shift[1],
                    entity.position[2] + shift[2],
                ],
            )
            .with_data(entity.data.clone()),
        );
    }
    document.insert_region(merged);

    Ok(MergedDocument {
        document,
        merged_region_names: regions
            .iter()
            .map(|r| r.id().as_str().to_string())
            .collect(),
        overlap_conflicts,
    })
}
