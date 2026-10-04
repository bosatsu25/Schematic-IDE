use crate::{surface::SurfaceAnalysis, voxel_face::VoxelFace};
use schematic_core::{DocumentRevision, Position};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SurfaceFeatureKind {
    UnknownBoundary,
    Interior,
    Isolated,
    Tip,
    ThinFeature,
    Corner,
    Edge,
    Face,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SurfaceFeatureDescriptor {
    pub position: Position,
    pub kind: SurfaceFeatureKind,
    pub exposure_vector_x: i8,
    pub exposure_vector_y: i8,
    pub exposure_vector_z: i8,
    pub exposed_axis_count: usize,
    pub opposite_exposure_pair_count: usize,
    pub surface_neighbor_count: usize,
    pub component_complete: bool,
}

impl SurfaceFeatureDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        position: Position,
        kind: SurfaceFeatureKind,
        exposure_vector_x: i8,
        exposure_vector_y: i8,
        exposure_vector_z: i8,
        exposed_axis_count: usize,
        opposite_exposure_pair_count: usize,
        surface_neighbor_count: usize,
        component_complete: bool,
    ) -> Result<Self, String> {
        if !(-1..=1).contains(&exposure_vector_x)
            || !(-1..=1).contains(&exposure_vector_y)
            || !(-1..=1).contains(&exposure_vector_z)
        {
            return Err("Exposure-vector components must be between -1 and 1".to_string());
        }
        if exposed_axis_count > 3 {
            return Err("exposed_axis_count must be between 0 and 3".to_string());
        }
        if opposite_exposure_pair_count > 3 {
            return Err("opposite_exposure_pair_count must be between 0 and 3".to_string());
        }
        if surface_neighbor_count > 6 {
            return Err("surface_neighbor_count must be between 0 and 6".to_string());
        }

        Ok(Self {
            position,
            kind,
            exposure_vector_x,
            exposure_vector_y,
            exposure_vector_z,
            exposed_axis_count,
            opposite_exposure_pair_count,
            surface_neighbor_count,
            component_complete,
        })
    }

    pub fn has_directional_exposure(&self) -> bool {
        self.exposure_vector_x != 0 || self.exposure_vector_y != 0 || self.exposure_vector_z != 0
    }

    pub fn has_complete_context(&self) -> bool {
        self.kind != SurfaceFeatureKind::UnknownBoundary && self.component_complete
    }
}

impl Ord for SurfaceFeatureDescriptor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.position.cmp(&other.position)
    }
}

impl PartialOrd for SurfaceFeatureDescriptor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceFeatureAnalysis {
    revision: DocumentRevision,
    descriptors: Vec<SurfaceFeatureDescriptor>,
}

impl SurfaceFeatureAnalysis {
    pub fn new(
        revision: DocumentRevision,
        mut descriptors: Vec<SurfaceFeatureDescriptor>,
    ) -> Result<Self, String> {
        descriptors.sort();
        for window in descriptors.windows(2) {
            if window[0].position == window[1].position {
                return Err(format!(
                    "Duplicate feature descriptor position: {:?}",
                    window[0].position
                ));
            }
        }
        Ok(Self {
            revision,
            descriptors,
        })
    }

    pub fn empty(revision: DocumentRevision) -> Self {
        Self {
            revision,
            descriptors: Vec::new(),
        }
    }

    pub fn revision(&self) -> DocumentRevision {
        self.revision
    }

    pub fn is_from(&self, surface: &SurfaceAnalysis) -> bool {
        self.revision == surface.revision()
    }

    pub fn descriptors(&self) -> &[SurfaceFeatureDescriptor] {
        &self.descriptors
    }

    pub fn descriptor_at(&self, position: Position) -> Option<&SurfaceFeatureDescriptor> {
        self.descriptors
            .binary_search_by_key(&position, |d| d.position)
            .ok()
            .map(|idx| &self.descriptors[idx])
    }

    pub fn of_kind(&self, kind: SurfaceFeatureKind) -> Vec<&SurfaceFeatureDescriptor> {
        self.descriptors.iter().filter(|d| d.kind == kind).collect()
    }
}

pub struct SurfaceFeatureAnalyzer;

impl SurfaceFeatureAnalyzer {
    pub fn analyze(surface_analysis: &SurfaceAnalysis) -> SurfaceFeatureAnalysis {
        if surface_analysis.cells().is_empty() {
            return SurfaceFeatureAnalysis::empty(surface_analysis.revision());
        }

        let mut cells_by_position = HashMap::with_capacity(surface_analysis.cells().len());
        for cell in surface_analysis.cells() {
            cells_by_position.insert(cell.position, cell);
        }

        let mut descriptors = Vec::with_capacity(surface_analysis.cells().len());

        for cell in surface_analysis.cells() {
            let component = surface_analysis
                .component_at(cell.component_root)
                .expect("Missing component for surface cell");

            let exposed_axis_count = Self::exposed_axis_count(&cell.exposed_faces);
            let opposite_pair_count = Self::opposite_exposure_pair_count(&cell.exposed_faces);
            let (vx, vy, vz) = Self::exposure_vector(&cell.exposed_faces);
            let surface_neighbor_count =
                Self::surface_neighbor_count(&cells_by_position, cell.position);

            let kind = Self::classify(cell, exposed_axis_count, opposite_pair_count);

            descriptors.push(
                SurfaceFeatureDescriptor::new(
                    cell.position,
                    kind,
                    vx,
                    vy,
                    vz,
                    exposed_axis_count,
                    opposite_pair_count,
                    surface_neighbor_count,
                    component.complete,
                )
                .expect("Valid descriptor attributes"),
            );
        }

        SurfaceFeatureAnalysis::new(surface_analysis.revision(), descriptors)
            .expect("Valid feature analysis")
    }

    fn classify(
        cell: &crate::surface::SurfaceCell,
        exposed_axis_count: usize,
        opposite_pair_count: usize,
    ) -> SurfaceFeatureKind {
        if cell.has_unknown_boundary() {
            return SurfaceFeatureKind::UnknownBoundary;
        }
        if cell.is_interior() {
            return SurfaceFeatureKind::Interior;
        }
        if cell.is_isolated() {
            return SurfaceFeatureKind::Isolated;
        }
        if cell.occupied_neighbor_count == 1 {
            return SurfaceFeatureKind::Tip;
        }
        if opposite_pair_count > 0 {
            return SurfaceFeatureKind::ThinFeature;
        }
        match exposed_axis_count {
            3 => SurfaceFeatureKind::Corner,
            2 => SurfaceFeatureKind::Edge,
            1 => SurfaceFeatureKind::Face,
            0 => SurfaceFeatureKind::Interior,
            other => panic!("Unexpected exposed-axis count: {other}"),
        }
    }

    fn exposed_axis_count(exposed_faces: &[VoxelFace]) -> usize {
        let x =
            exposed_faces.contains(&VoxelFace::West) || exposed_faces.contains(&VoxelFace::East);
        let y = exposed_faces.contains(&VoxelFace::Down) || exposed_faces.contains(&VoxelFace::Up);
        let z =
            exposed_faces.contains(&VoxelFace::North) || exposed_faces.contains(&VoxelFace::South);
        usize::from(x) + usize::from(y) + usize::from(z)
    }

    fn opposite_exposure_pair_count(exposed_faces: &[VoxelFace]) -> usize {
        let mut pairs = 0;
        if exposed_faces.contains(&VoxelFace::West) && exposed_faces.contains(&VoxelFace::East) {
            pairs += 1;
        }
        if exposed_faces.contains(&VoxelFace::Down) && exposed_faces.contains(&VoxelFace::Up) {
            pairs += 1;
        }
        if exposed_faces.contains(&VoxelFace::North) && exposed_faces.contains(&VoxelFace::South) {
            pairs += 1;
        }
        pairs
    }

    fn exposure_vector(exposed_faces: &[VoxelFace]) -> (i8, i8, i8) {
        let mut x: i8 = 0;
        let mut y: i8 = 0;
        let mut z: i8 = 0;
        for face in exposed_faces {
            match face {
                VoxelFace::Down => y -= 1,
                VoxelFace::Up => y += 1,
                VoxelFace::North => z -= 1,
                VoxelFace::South => z += 1,
                VoxelFace::West => x -= 1,
                VoxelFace::East => x += 1,
            }
        }
        (x, y, z)
    }

    fn surface_neighbor_count(
        cells_by_position: &HashMap<Position, &crate::surface::SurfaceCell>,
        position: Position,
    ) -> usize {
        let mut count = 0;
        for face in VoxelFace::ALL {
            if let Some(neighbor) = face.neighbor_of(position) {
                if let Some(neighbor_cell) = cells_by_position.get(&neighbor) {
                    if neighbor_cell.is_surface() {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}
