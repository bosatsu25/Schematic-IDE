use crate::{occupancy::OccupancyPolicy, voxel_face::VoxelFace};
use schematic_core::{
    BlockState, ChunkPosition, Document, DocumentRevision, OperationTarget, Position, RegionId,
};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceCell {
    pub position: Position,
    pub state: BlockState,
    pub exposed_faces: Vec<VoxelFace>,
    pub unknown_faces: Vec<VoxelFace>,
    pub occupied_neighbor_count: usize,
    pub component_root: Position,
}

impl SurfaceCell {
    pub fn is_interior(&self) -> bool {
        self.occupied_neighbor_count == 6
    }

    pub fn is_surface(&self) -> bool {
        !self.exposed_faces.is_empty()
    }

    pub fn is_isolated(&self) -> bool {
        self.occupied_neighbor_count == 0 && self.unknown_faces.is_empty()
    }

    pub fn is_weakly_supported(&self) -> bool {
        self.occupied_neighbor_count <= 1 && self.unknown_faces.is_empty()
    }

    pub fn has_unknown_boundary(&self) -> bool {
        !self.unknown_faces.is_empty()
    }

    pub fn known_empty_neighbor_count(&self) -> usize {
        self.exposed_faces.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SurfaceComponent {
    pub root: Position,
    pub size: usize,
    pub complete: bool,
}

impl SurfaceComponent {
    pub fn is_small_island_candidate(&self, max_size: usize) -> bool {
        self.complete && self.size <= max_size
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceAnalysis {
    revision: DocumentRevision,
    cells: Vec<SurfaceCell>,
    components: Vec<SurfaceComponent>,
}

impl SurfaceAnalysis {
    pub fn new(
        revision: DocumentRevision,
        mut cells: Vec<SurfaceCell>,
        mut components: Vec<SurfaceComponent>,
    ) -> Self {
        cells.sort_by_key(|c| c.position);
        components.sort();
        Self {
            revision,
            cells,
            components,
        }
    }

    pub fn empty(revision: DocumentRevision) -> Self {
        Self {
            revision,
            cells: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }

    pub fn is_from(&self, revision: DocumentRevision) -> bool {
        self.revision == revision
    }

    pub fn cells(&self) -> &[SurfaceCell] {
        &self.cells
    }

    pub fn components(&self) -> &[SurfaceComponent] {
        &self.components
    }

    pub fn cell_at(&self, position: Position) -> Option<&SurfaceCell> {
        self.cells
            .binary_search_by_key(&position, |c| c.position)
            .ok()
            .map(|idx| &self.cells[idx])
    }

    pub fn component_at(&self, root: Position) -> Option<&SurfaceComponent> {
        self.components
            .binary_search_by_key(&root, |c| c.root)
            .ok()
            .map(|idx| &self.components[idx])
    }

    pub fn small_island_candidates(&self, max_size: usize) -> Vec<&SurfaceComponent> {
        self.components
            .iter()
            .filter(|c| c.is_small_island_candidate(max_size))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisError {
    MissingRegion(RegionId),
    InvalidCoordinate,
}

impl Display for AnalysisError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRegion(id) => write!(formatter, "region '{}' does not exist", id.as_str()),
            Self::InvalidCoordinate => {
                formatter.write_str("coordinate conversion failed or exceeded bounds")
            }
        }
    }
}

impl Error for AnalysisError {}

pub struct SurfaceAnalyzer;

impl SurfaceAnalyzer {
    pub fn analyze(
        document: &Document,
        region_id: &RegionId,
        target: &OperationTarget,
        occupancy: &impl OccupancyPolicy,
    ) -> Result<SurfaceAnalysis, AnalysisError> {
        let region = document
            .region(region_id)
            .ok_or_else(|| AnalysisError::MissingRegion(region_id.clone()))?;

        let target_regions = target.world_regions();
        if target_regions.is_empty() {
            return Ok(SurfaceAnalysis::empty(document.revision()));
        }

        // Collect all occupied blocks inside the target world regions
        let mut occupied_inside_target = BTreeMap::new();

        for (chunk_pos, chunk) in region.chunks() {
            if chunk.is_empty() {
                continue;
            }
            for (idx, palette_idx) in chunk.occupied_blocks() {
                let Some(local_pos) = chunk_pos.block_position_for_index(idx) else {
                    continue;
                };
                let Some(world_pos) = region.local_to_world(local_pos) else {
                    continue;
                };
                if !target_regions.iter().any(|r| r.contains(world_pos)) {
                    continue;
                }
                let Some(state) = region.palette().get(palette_idx) else {
                    continue;
                };
                if occupancy.is_occupied(state) {
                    occupied_inside_target.insert(world_pos, (local_pos, state.clone()));
                }
            }
        }

        if occupied_inside_target.is_empty() {
            return Ok(SurfaceAnalysis::empty(document.revision()));
        }

        // Helper to query block status at a world position:
        // returns Some(Some(state)) if occupied, Some(None) if known empty/air, None if unknown/unloaded
        let query_world_block = |pos: Position| -> Option<Option<BlockState>> {
            let local_pos = region.world_to_local(pos)?;
            let chunk_pos = ChunkPosition::from_block_position(local_pos);
            let _chunk = region.chunk(chunk_pos)?;
            let block_index = region.block_index_at(local_pos).ok()?;
            match block_index {
                Some(palette_idx) => {
                    let state = region.palette().get(palette_idx)?;
                    if occupancy.is_occupied(state) {
                        Some(Some(state.clone()))
                    } else {
                        Some(None)
                    }
                }
                None => Some(None), // Chunk is present and stored None -> known empty
            }
        };

        // Build 6-connected components
        let mut visited = HashSet::new();
        let mut root_by_position = BTreeMap::new();
        let mut components = Vec::new();

        for &root in occupied_inside_target.keys() {
            if !visited.insert(root) {
                continue;
            }

            let mut queue = VecDeque::new();
            let mut members = Vec::new();
            let mut complete = true;

            queue.push_back(root);

            while let Some(current) = queue.pop_front() {
                members.push(current);

                for face in VoxelFace::ALL {
                    let Some(neighbor_pos) = face.neighbor_of(current) else {
                        complete = false;
                        continue;
                    };

                    match query_world_block(neighbor_pos) {
                        None => {
                            // Missing data / outside region
                            complete = false;
                        }
                        Some(None) => {
                            // Known empty
                        }
                        Some(Some(_neighbor_state)) => {
                            // Occupied neighbor
                            let in_target = target_regions.iter().any(|r| r.contains(neighbor_pos));
                            if !in_target {
                                complete = false;
                            } else if occupied_inside_target.contains_key(&neighbor_pos)
                                && visited.insert(neighbor_pos)
                            {
                                queue.push_back(neighbor_pos);
                            }
                        }
                    }
                }
            }

            for &member in &members {
                root_by_position.insert(member, root);
            }
            components.push(SurfaceComponent {
                root,
                size: members.len(),
                complete,
            });
        }

        // Build SurfaceCells
        let mut cells = Vec::with_capacity(occupied_inside_target.len());

        for (&world_pos, &(_local_pos, ref state)) in &occupied_inside_target {
            let mut exposed_faces = Vec::new();
            let mut unknown_faces = Vec::new();
            let mut occupied_neighbor_count = 0;

            for face in VoxelFace::ALL {
                let Some(neighbor_pos) = face.neighbor_of(world_pos) else {
                    unknown_faces.push(face);
                    continue;
                };

                match query_world_block(neighbor_pos) {
                    None => {
                        unknown_faces.push(face);
                    }
                    Some(None) => {
                        exposed_faces.push(face);
                    }
                    Some(Some(_)) => {
                        occupied_neighbor_count += 1;
                    }
                }
            }

            let component_root = root_by_position
                .get(&world_pos)
                .copied()
                .unwrap_or(world_pos);

            cells.push(SurfaceCell {
                position: world_pos,
                state: state.clone(),
                exposed_faces,
                unknown_faces,
                occupied_neighbor_count,
                component_root,
            });
        }

        Ok(SurfaceAnalysis::new(document.revision(), cells, components))
    }
}
