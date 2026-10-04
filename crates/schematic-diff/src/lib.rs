use schematic_core::{BlockState, Document, Position, Region, RegionId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
    Unchanged,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiffBlockState {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}

impl From<&BlockState> for DiffBlockState {
    fn from(state: &BlockState) -> Self {
        let mut properties = BTreeMap::new();
        for prop in state.properties() {
            properties.insert(prop.name().to_string(), prop.value().to_string());
        }
        Self {
            name: state.id().to_string(),
            properties,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockDiff {
    pub region_id: String,
    pub position: [i32; 3], // world position
    pub local_position: [i32; 3],
    pub kind: DiffKind,
    pub before: Option<DiffBlockState>,
    pub after: Option<DiffBlockState>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityDiff {
    pub position: [i32; 3],
    pub kind: DiffKind,
    pub before_type: Option<String>,
    pub after_type: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegionDiffSummary {
    pub region_id: String,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub unchanged_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentDiff {
    pub summaries: Vec<RegionDiffSummary>,
    pub total_added: usize,
    pub total_removed: usize,
    pub total_modified: usize,
    pub total_unchanged: usize,
    pub block_diffs: Vec<BlockDiff>,
    pub entity_diffs: Vec<EntityDiff>,
}

pub fn diff_regions(
    region_id: &RegionId,
    before: Option<&Region>,
    after: Option<&Region>,
    include_unchanged: bool,
) -> (RegionDiffSummary, Vec<BlockDiff>) {
    let mut summary = RegionDiffSummary {
        region_id: region_id.as_str().to_string(),
        added_count: 0,
        removed_count: 0,
        modified_count: 0,
        unchanged_count: 0,
    };
    let mut diffs = Vec::new();

    match (before, after) {
        (None, None) => (summary, diffs),
        (None, Some(after_reg)) => {
            for (chunk_pos, chunk) in after_reg.chunks() {
                for (idx, pal_idx) in chunk.occupied_blocks() {
                    if let Some(local_pos) = chunk_pos.block_position_for_index(idx) {
                        let state = after_reg.palette().get(pal_idx).map(DiffBlockState::from);
                        let world_pos = after_reg
                            .local_to_world(local_pos)
                            .unwrap_or(Position::new(0, 0, 0));
                        summary.added_count += 1;
                        diffs.push(BlockDiff {
                            region_id: region_id.as_str().to_string(),
                            position: [world_pos.x, world_pos.y, world_pos.z],
                            local_position: [
                                local_pos.x as i32,
                                local_pos.y as i32,
                                local_pos.z as i32,
                            ],
                            kind: DiffKind::Added,
                            before: None,
                            after: state,
                        });
                    }
                }
            }
            (summary, diffs)
        }
        (Some(before_reg), None) => {
            for (chunk_pos, chunk) in before_reg.chunks() {
                for (idx, pal_idx) in chunk.occupied_blocks() {
                    if let Some(local_pos) = chunk_pos.block_position_for_index(idx) {
                        let state = before_reg.palette().get(pal_idx).map(DiffBlockState::from);
                        let world_pos = before_reg
                            .local_to_world(local_pos)
                            .unwrap_or(Position::new(0, 0, 0));
                        summary.removed_count += 1;
                        diffs.push(BlockDiff {
                            region_id: region_id.as_str().to_string(),
                            position: [world_pos.x, world_pos.y, world_pos.z],
                            local_position: [
                                local_pos.x as i32,
                                local_pos.y as i32,
                                local_pos.z as i32,
                            ],
                            kind: DiffKind::Removed,
                            before: state,
                            after: None,
                        });
                    }
                }
            }
            (summary, diffs)
        }
        (Some(before_reg), Some(after_reg)) => {
            let mut all_chunk_positions = BTreeSet::new();
            for (chunk_pos, _) in before_reg.chunks() {
                all_chunk_positions.insert(chunk_pos);
            }
            for (chunk_pos, _) in after_reg.chunks() {
                all_chunk_positions.insert(chunk_pos);
            }

            for chunk_pos in all_chunk_positions {
                let chunk_before = before_reg.chunk(chunk_pos);
                let chunk_after = after_reg.chunk(chunk_pos);

                let mut chunk_indices = BTreeSet::new();
                if let Some(c) = chunk_before {
                    for (idx, _) in c.occupied_blocks() {
                        chunk_indices.insert(idx);
                    }
                }
                if let Some(c) = chunk_after {
                    for (idx, _) in c.occupied_blocks() {
                        chunk_indices.insert(idx);
                    }
                }

                for idx in chunk_indices {
                    let Some(local_pos) = chunk_pos.block_position_for_index(idx) else {
                        continue;
                    };
                    let state_before = chunk_before
                        .and_then(|c| c.get(idx).ok().flatten())
                        .and_then(|pi| before_reg.palette().get(pi));
                    let state_after = chunk_after
                        .and_then(|c| c.get(idx).ok().flatten())
                        .and_then(|pi| after_reg.palette().get(pi));

                    let world_pos = after_reg
                        .local_to_world(local_pos)
                        .or_else(|| before_reg.local_to_world(local_pos))
                        .unwrap_or(Position::new(0, 0, 0));

                    let diff_item = match (state_before, state_after) {
                        (Some(b), None) => {
                            summary.removed_count += 1;
                            Some(BlockDiff {
                                region_id: region_id.as_str().to_string(),
                                position: [world_pos.x, world_pos.y, world_pos.z],
                                local_position: [
                                    local_pos.x as i32,
                                    local_pos.y as i32,
                                    local_pos.z as i32,
                                ],
                                kind: DiffKind::Removed,
                                before: Some(DiffBlockState::from(b)),
                                after: None,
                            })
                        }
                        (None, Some(a)) => {
                            summary.added_count += 1;
                            Some(BlockDiff {
                                region_id: region_id.as_str().to_string(),
                                position: [world_pos.x, world_pos.y, world_pos.z],
                                local_position: [
                                    local_pos.x as i32,
                                    local_pos.y as i32,
                                    local_pos.z as i32,
                                ],
                                kind: DiffKind::Added,
                                before: None,
                                after: Some(DiffBlockState::from(a)),
                            })
                        }
                        (Some(b), Some(a)) => {
                            if b == a {
                                summary.unchanged_count += 1;
                                if include_unchanged {
                                    Some(BlockDiff {
                                        region_id: region_id.as_str().to_string(),
                                        position: [world_pos.x, world_pos.y, world_pos.z],
                                        local_position: [
                                            local_pos.x as i32,
                                            local_pos.y as i32,
                                            local_pos.z as i32,
                                        ],
                                        kind: DiffKind::Unchanged,
                                        before: Some(DiffBlockState::from(b)),
                                        after: Some(DiffBlockState::from(a)),
                                    })
                                } else {
                                    None
                                }
                            } else {
                                summary.modified_count += 1;
                                Some(BlockDiff {
                                    region_id: region_id.as_str().to_string(),
                                    position: [world_pos.x, world_pos.y, world_pos.z],
                                    local_position: [
                                        local_pos.x as i32,
                                        local_pos.y as i32,
                                        local_pos.z as i32,
                                    ],
                                    kind: DiffKind::Modified,
                                    before: Some(DiffBlockState::from(b)),
                                    after: Some(DiffBlockState::from(a)),
                                })
                            }
                        }
                        (None, None) => None,
                    };

                    if let Some(diff) = diff_item {
                        diffs.push(diff);
                    }
                }
            }
            (summary, diffs)
        }
    }
}

pub fn diff_documents(
    before: &Document,
    after: &Document,
    include_unchanged: bool,
) -> DocumentDiff {
    let mut all_region_ids = BTreeSet::new();
    for reg in before.regions() {
        all_region_ids.insert(reg.id().clone());
    }
    for reg in after.regions() {
        all_region_ids.insert(reg.id().clone());
    }

    let mut summaries = Vec::new();
    let mut block_diffs = Vec::new();
    let mut total_added = 0;
    let mut total_removed = 0;
    let mut total_modified = 0;
    let mut total_unchanged = 0;

    for reg_id in all_region_ids {
        let before_reg = before.region(&reg_id);
        let after_reg = after.region(&reg_id);
        let (summary, diffs) = diff_regions(&reg_id, before_reg, after_reg, include_unchanged);

        total_added += summary.added_count;
        total_removed += summary.removed_count;
        total_modified += summary.modified_count;
        total_unchanged += summary.unchanged_count;

        summaries.push(summary);
        block_diffs.extend(diffs);
    }

    // Entity & BlockEntity diffs
    let mut entity_diffs = Vec::new();
    let mut be_before: BTreeMap<Position, String> = BTreeMap::new();
    for be in before.block_entities() {
        be_before.insert(be.position, be.kind.clone());
    }
    let mut be_after: BTreeMap<Position, String> = BTreeMap::new();
    for be in after.block_entities() {
        be_after.insert(be.position, be.kind.clone());
    }

    let mut all_be_pos = BTreeSet::new();
    for pos in be_before.keys() {
        all_be_pos.insert(*pos);
    }
    for pos in be_after.keys() {
        all_be_pos.insert(*pos);
    }

    for pos in all_be_pos {
        let b = be_before.get(&pos).cloned();
        let a = be_after.get(&pos).cloned();
        match (b, a) {
            (Some(kind), None) => {
                entity_diffs.push(EntityDiff {
                    position: [pos.x, pos.y, pos.z],
                    kind: DiffKind::Removed,
                    before_type: Some(kind),
                    after_type: None,
                });
            }
            (None, Some(kind)) => {
                entity_diffs.push(EntityDiff {
                    position: [pos.x, pos.y, pos.z],
                    kind: DiffKind::Added,
                    before_type: None,
                    after_type: Some(kind),
                });
            }
            (Some(kb), Some(ka)) => {
                if kb != ka {
                    entity_diffs.push(EntityDiff {
                        position: [pos.x, pos.y, pos.z],
                        kind: DiffKind::Modified,
                        before_type: Some(kb),
                        after_type: Some(ka),
                    });
                } else if include_unchanged {
                    entity_diffs.push(EntityDiff {
                        position: [pos.x, pos.y, pos.z],
                        kind: DiffKind::Unchanged,
                        before_type: Some(kb),
                        after_type: Some(ka),
                    });
                }
            }
            (None, None) => {}
        }
    }

    DocumentDiff {
        summaries,
        total_added,
        total_removed,
        total_modified,
        total_unchanged,
        block_diffs,
        entity_diffs,
    }
}
