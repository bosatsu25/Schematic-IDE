use crate::{
    command::{planned_palette_index, EditError},
    patch::{BlockChange, PatchSet},
};
use schematic_analysis::{
    NonAirPolicy, OccupancyPolicy, SurfaceAnalysis, SurfaceFeatureAnalysis, SurfaceFeatureKind,
};
use schematic_core::{BlockState, Document, Position, RegionId};
use std::collections::{BTreeSet, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IslandCleanupRequest {
    pub max_component_size: usize,
    pub replacement_state: BlockState,
    pub protected_feature_kinds: BTreeSet<SurfaceFeatureKind>,
}

impl IslandCleanupRequest {
    pub fn new(
        max_component_size: usize,
        replacement_state: BlockState,
        protected_feature_kinds: impl IntoIterator<Item = SurfaceFeatureKind>,
    ) -> Result<Self, CleanupError> {
        if max_component_size == 0 {
            return Err(CleanupError::InvalidComponentSize);
        }
        Ok(Self {
            max_component_size,
            replacement_state,
            protected_feature_kinds: protected_feature_kinds.into_iter().collect(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupError {
    InvalidComponentSize,
    StaleSurfaceAnalysis,
    StaleFeatureAnalysis,
    MissingRegion(RegionId),
    MissingFeatureEvidence(Position),
    Edit(EditError),
}

impl Display for CleanupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidComponentSize => {
                formatter.write_str("max_component_size must be positive")
            }
            Self::StaleSurfaceAnalysis => {
                formatter.write_str("surface analysis revision does not match document")
            }
            Self::StaleFeatureAnalysis => {
                formatter.write_str("feature analysis revision does not match surface analysis")
            }
            Self::MissingRegion(id) => write!(formatter, "region '{}' does not exist", id.as_str()),
            Self::MissingFeatureEvidence(pos) => {
                write!(
                    formatter,
                    "missing feature evidence for analyzed cell at {pos:?}"
                )
            }
            Self::Edit(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CleanupError {}

impl From<EditError> for CleanupError {
    fn from(error: EditError) -> Self {
        Self::Edit(error)
    }
}

pub struct DisconnectedIslandCleanupPlanner;

impl DisconnectedIslandCleanupPlanner {
    pub fn plan(
        document: &Document,
        region_id: &RegionId,
        surface: &SurfaceAnalysis,
        features: &SurfaceFeatureAnalysis,
        request: &IslandCleanupRequest,
    ) -> Result<PatchSet, CleanupError> {
        if !surface.is_from(document.revision()) {
            return Err(CleanupError::StaleSurfaceAnalysis);
        }
        if !features.is_from(surface) {
            return Err(CleanupError::StaleFeatureAnalysis);
        }

        let region = document
            .region(region_id)
            .ok_or_else(|| CleanupError::MissingRegion(region_id.clone()))?;

        let mut eligible_roots = HashSet::new();
        for component in surface.small_island_candidates(request.max_component_size) {
            eligible_roots.insert(component.root);
        }

        // Complete the preservation pass before generating changes:
        // one protected cell vetoes the entire component.
        for cell in surface.cells() {
            if !eligible_roots.contains(&cell.component_root) {
                continue;
            }
            let descriptor = features
                .descriptor_at(cell.position)
                .ok_or(CleanupError::MissingFeatureEvidence(cell.position))?;
            if cell.has_unknown_boundary()
                || !descriptor.has_complete_context()
                || request.protected_feature_kinds.contains(&descriptor.kind)
            {
                eligible_roots.remove(&cell.component_root);
            }
        }

        let mut changes = Vec::new();
        for cell in surface.cells() {
            if eligible_roots.contains(&cell.component_root)
                && cell.state != request.replacement_state
            {
                let local_pos = region
                    .world_to_local(cell.position)
                    .ok_or(CleanupError::Edit(EditError::InvalidCoordinate))?;
                let old_index = region
                    .block_index_at(local_pos)
                    .map_err(|e| CleanupError::Edit(EditError::RegionBlock(e)))?;
                changes.push((local_pos, old_index));
            }
        }

        if changes.is_empty() {
            return Ok(PatchSet::empty());
        }

        let is_replacement_air = !NonAirPolicy.is_occupied(&request.replacement_state);
        if is_replacement_air {
            let block_changes = changes
                .into_iter()
                .map(|(pos, before)| BlockChange::new(pos, before, None));
            Ok(PatchSet::from_changes(
                region_id.clone(),
                block_changes,
                None,
            ))
        } else {
            let (new_index, addition) = planned_palette_index(region, &request.replacement_state)
                .map_err(CleanupError::Edit)?;
            let block_changes = changes
                .into_iter()
                .map(|(pos, before)| BlockChange::new(pos, before, Some(new_index)));
            Ok(PatchSet::from_changes(
                region_id.clone(),
                block_changes,
                addition,
            ))
        }
    }
}
