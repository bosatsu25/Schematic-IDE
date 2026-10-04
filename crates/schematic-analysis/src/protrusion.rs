use crate::{
    features::{SurfaceFeatureAnalysis, SurfaceFeatureDescriptor},
    surface::{SurfaceAnalysis, SurfaceCell, SurfaceComponent},
    voxel_face::VoxelFace,
};
use schematic_core::{DocumentRevision, Position};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtrusionTermination {
    JunctionReached,
    OtherTipReached,
    TraceLimitReached,
    IncompleteContext,
    CoordinateOverflow,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProtrusionContextIssue {
    IncompleteComponent,
    UnknownNeighbor,
    OccupiedNeighborOutsideTarget,
    CoordinateOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtrusionAnalysisRequest {
    pub max_trace_length: usize,
}

impl ProtrusionAnalysisRequest {
    pub fn new(max_trace_length: usize) -> Result<Self, String> {
        if max_trace_length == 0 {
            return Err("max_trace_length must be positive".to_string());
        }
        Ok(Self { max_trace_length })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtrusionEvidence {
    pub component: SurfaceComponent,
    pub path: Vec<SurfaceFeatureDescriptor>,
    pub attachment: Option<SurfaceFeatureDescriptor>,
    pub attachment_support_faces: Vec<VoxelFace>,
    pub termination: ProtrusionTermination,
    pub context_issues: Vec<ProtrusionContextIssue>,
}

impl ProtrusionEvidence {
    pub fn new(
        component: SurfaceComponent,
        path: Vec<SurfaceFeatureDescriptor>,
        attachment: Option<SurfaceFeatureDescriptor>,
        attachment_support_faces: Vec<VoxelFace>,
        termination: ProtrusionTermination,
        mut context_issues: Vec<ProtrusionContextIssue>,
    ) -> Result<Self, String> {
        if path.is_empty() {
            return Err("A trace must include its seed".to_string());
        }
        context_issues.sort();
        context_issues.dedup();
        Ok(Self {
            component,
            path,
            attachment,
            attachment_support_faces,
            termination,
            context_issues,
        })
    }

    pub fn tip_position(&self) -> Position {
        self.path[0].position
    }

    pub fn path_positions(&self) -> Vec<Position> {
        self.path.iter().map(|d| d.position).collect()
    }

    pub fn observed_path_length(&self) -> usize {
        self.path.len()
    }

    pub fn exact_path_length(&self) -> Option<usize> {
        if self.component.complete
            && self.context_issues.is_empty()
            && (self.termination == ProtrusionTermination::JunctionReached
                || self.termination == ProtrusionTermination::OtherTipReached)
        {
            Some(self.path.len())
        } else {
            None
        }
    }

    pub fn attachment_direction(&self) -> Option<VoxelFace> {
        self.attachment.as_ref().map(|att| {
            Self::face_direction(self.path.last().unwrap().position, att.position)
                .expect("Attachment must be neighbor of last path element")
        })
    }

    pub fn step_directions(&self) -> Vec<VoxelFace> {
        let mut steps = Vec::new();
        for i in 1..self.path.len() {
            let step = Self::face_direction(self.path[i - 1].position, self.path[i].position)
                .expect("Trace steps must be face connected");
            steps.push(step);
        }
        if let Some(att_dir) = self.attachment_direction() {
            steps.push(att_dir);
        }
        steps
    }

    pub fn direction_change_count(&self) -> usize {
        let steps = self.step_directions();
        let mut changes = 0;
        for i in 1..steps.len() {
            if steps[i] != steps[i - 1] {
                changes += 1;
            }
        }
        changes
    }

    fn face_direction(from: Position, to: Position) -> Option<VoxelFace> {
        VoxelFace::ALL
            .into_iter()
            .find(|&face| face.neighbor_of(from) == Some(to))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectedProtrusionAnalysis {
    revision: DocumentRevision,
    request: ProtrusionAnalysisRequest,
    evidence: Vec<ProtrusionEvidence>,
}

impl ConnectedProtrusionAnalysis {
    pub fn new(
        revision: DocumentRevision,
        request: ProtrusionAnalysisRequest,
        evidence: Vec<ProtrusionEvidence>,
    ) -> Self {
        Self {
            revision,
            request,
            evidence,
        }
    }

    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }

    pub fn request(&self) -> ProtrusionAnalysisRequest {
        self.request
    }

    pub fn evidence(&self) -> &[ProtrusionEvidence] {
        &self.evidence
    }

    pub fn is_from(&self, surface: &SurfaceAnalysis, features: &SurfaceFeatureAnalysis) -> bool {
        self.revision == surface.revision() && self.revision == features.revision()
    }
}

pub struct ConnectedProtrusionAnalyzer;

impl ConnectedProtrusionAnalyzer {
    pub fn analyze(
        surface: &SurfaceAnalysis,
        features: &SurfaceFeatureAnalysis,
        request: ProtrusionAnalysisRequest,
    ) -> Result<ConnectedProtrusionAnalysis, String> {
        if surface.revision() != features.revision() {
            return Err("Feature analysis revision does not match surface revision".to_string());
        }

        let mut cells = HashMap::with_capacity(surface.cells().len());
        for cell in surface.cells() {
            cells.insert(cell.position, cell);
        }

        let mut descriptors = HashMap::with_capacity(features.descriptors().len());
        for descriptor in features.descriptors() {
            descriptors.insert(descriptor.position, descriptor);
        }

        let mut components = HashMap::with_capacity(surface.components().len());
        for comp in surface.components() {
            components.insert(comp.root, comp);
        }

        let mut evidence = Vec::new();

        for seed in surface.cells() {
            if seed.occupied_neighbor_count != 1 || seed.has_unknown_boundary() {
                continue;
            }
            let component = components
                .get(&seed.component_root)
                .expect("Component must exist for seed");

            let trace_result = Self::trace(
                seed,
                &cells,
                &descriptors,
                component,
                request.max_trace_length,
            );

            if trace_result.termination == ProtrusionTermination::OtherTipReached {
                if let Some(last_pos) = trace_result.path.last().map(|d| d.position) {
                    if last_pos < seed.position {
                        continue;
                    }
                }
            }

            evidence.push(trace_result);
        }

        Ok(ConnectedProtrusionAnalysis::new(
            surface.revision(),
            request,
            evidence,
        ))
    }

    fn trace(
        seed: &SurfaceCell,
        cells: &HashMap<Position, &SurfaceCell>,
        descriptors: &HashMap<Position, &SurfaceFeatureDescriptor>,
        component: &SurfaceComponent,
        limit: usize,
    ) -> ProtrusionEvidence {
        let mut path = Vec::new();
        let mut issues = BTreeSet::new();

        if !component.complete {
            issues.insert(ProtrusionContextIssue::IncompleteComponent);
        }

        let mut previous: Option<Position> = None;
        let mut current = seed;

        for visited in 1.. {
            let descriptor = **descriptors
                .get(&current.position)
                .expect("Descriptor must exist for cell");

            let mut occupied_faces = Vec::new();
            let mut next: Option<Position> = None;
            let mut overflow = false;
            let mut missing = false;

            for face in VoxelFace::ALL {
                let neighbor = face.neighbor_of(current.position);
                match neighbor {
                    None => {
                        overflow = true;
                        issues.insert(ProtrusionContextIssue::CoordinateOverflow);
                    }
                    Some(n_pos) => {
                        if current.unknown_faces.contains(&face) {
                            issues.insert(ProtrusionContextIssue::UnknownNeighbor);
                        } else if !current.exposed_faces.contains(&face) {
                            occupied_faces.push(face);
                            if !cells.contains_key(&n_pos) {
                                missing = true;
                                issues
                                    .insert(ProtrusionContextIssue::OccupiedNeighborOutsideTarget);
                            }
                            if previous != Some(n_pos) {
                                next = Some(n_pos);
                            }
                        }
                    }
                }
            }

            if overflow || current.has_unknown_boundary() {
                path.push(descriptor);
                let term = if overflow {
                    ProtrusionTermination::CoordinateOverflow
                } else {
                    ProtrusionTermination::IncompleteContext
                };
                return Self::build_result(*component, path, None, Vec::new(), term, issues);
            }

            if current.occupied_neighbor_count >= 3 {
                let incoming = previous;
                occupied_faces.retain(|face| face.neighbor_of(descriptor.position) != incoming);
                return Self::build_result(
                    *component,
                    path,
                    Some(descriptor),
                    occupied_faces,
                    ProtrusionTermination::JunctionReached,
                    issues,
                );
            }

            path.push(descriptor);

            if missing {
                return Self::build_result(
                    *component,
                    path,
                    None,
                    Vec::new(),
                    ProtrusionTermination::IncompleteContext,
                    issues,
                );
            }

            if previous.is_some() && current.occupied_neighbor_count == 1 {
                return Self::build_result(
                    *component,
                    path,
                    None,
                    Vec::new(),
                    ProtrusionTermination::OtherTipReached,
                    issues,
                );
            }

            if visited == limit {
                return Self::build_result(
                    *component,
                    path,
                    None,
                    Vec::new(),
                    ProtrusionTermination::TraceLimitReached,
                    issues,
                );
            }

            previous = Some(current.position);
            current = cells
                .get(&next.expect("Path continuation must have next block"))
                .expect("Next cell must be present in map");
        }

        unreachable!()
    }

    fn build_result(
        component: SurfaceComponent,
        path: Vec<SurfaceFeatureDescriptor>,
        attachment: Option<SurfaceFeatureDescriptor>,
        support: Vec<VoxelFace>,
        mut termination: ProtrusionTermination,
        issues: BTreeSet<ProtrusionContextIssue>,
    ) -> ProtrusionEvidence {
        if !issues.is_empty() && termination != ProtrusionTermination::CoordinateOverflow {
            termination = ProtrusionTermination::IncompleteContext;
        }

        ProtrusionEvidence::new(
            component,
            path,
            attachment,
            support,
            termination,
            issues.into_iter().collect(),
        )
        .expect("Valid protrusion evidence")
    }
}
