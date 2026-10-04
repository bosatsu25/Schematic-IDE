use schematic_analysis::{
    ConnectedProtrusionAnalyzer, NonAirPolicy, ProtrusionAnalysisRequest, ProtrusionTermination,
    SurfaceAnalysis, SurfaceAnalyzer, SurfaceCell, SurfaceComponent, SurfaceFeatureAnalyzer,
    SurfaceFeatureKind, VoxelFace,
};
use schematic_core::{
    BlockState, Document, DocumentMetadata, OperationTarget, Position, Region, RegionId, Selection,
    Size,
};

fn create_test_doc(
    region_id: &RegionId,
    min_chunk: (i32, i32, i32),
    max_chunk: (i32, i32, i32),
) -> Document {
    let mut doc = Document::new(DocumentMetadata::default());
    let origin = Position::new(min_chunk.0 * 16, min_chunk.1 * 16, min_chunk.2 * 16);
    let size = Size::new(
        ((max_chunk.0 - min_chunk.0 + 1) * 16) as u32,
        ((max_chunk.1 - min_chunk.1 + 1) * 16) as u32,
        ((max_chunk.2 - min_chunk.2 + 1) * 16) as u32,
    );
    let region = Region::new(region_id.clone(), origin, size);
    doc.insert_region(region);
    doc
}

fn set_block(doc: &mut Document, region_id: &RegionId, x: i32, y: i32, z: i32, state: BlockState) {
    let region = doc.region_mut(region_id).unwrap();
    let world_pos = Position::new(x, y, z);
    let local_pos = region.world_to_local(world_pos).expect("Inside region");
    let idx = region.palette_mut().intern(state).unwrap();
    region.set_block_index(local_pos, Some(idx)).unwrap();
}

fn make_target(
    min_x: i32,
    min_y: i32,
    min_z: i32,
    max_x: i32,
    max_y: i32,
    max_z: i32,
) -> OperationTarget {
    let sel = Selection::from_corners(
        Position::new(min_x, min_y, min_z),
        Position::new(max_x, max_y, max_z),
    );
    OperationTarget::unconstrained(sel)
}

fn stone() -> BlockState {
    BlockState::new("minecraft:stone", []).unwrap()
}

#[test]
fn test_surface_classification_interior_and_exposed_faces() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // 3x3x3 cube of stone from (5,5,5) to (7,7,7) inside recorded chunk (0,0,0)
    for x in 5..=7 {
        for y in 5..=7 {
            for z in 5..=7 {
                set_block(&mut doc, &rid, x, y, z, stone());
            }
        }
    }

    let target = make_target(5, 5, 5, 7, 7, 7);
    let analysis = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();

    let center = analysis.cell_at(Position::new(6, 6, 6)).unwrap();
    assert!(center.is_interior());
    assert!(!center.is_surface());
    assert_eq!(center.occupied_neighbor_count, 6);

    let corner = analysis.cell_at(Position::new(5, 5, 5)).unwrap();
    assert_eq!(
        corner.exposed_faces,
        vec![VoxelFace::Down, VoxelFace::North, VoxelFace::West]
    );
    assert!(corner.is_surface());
    assert!(!corner.has_unknown_boundary());

    let comp = &analysis.components()[0];
    assert_eq!(comp.size, 27);
    assert!(comp.complete);
}

#[test]
fn test_missing_neighbor_is_unknown() {
    let rid = RegionId::new("test");
    // Region that only covers chunk (0,0,0) [0..15, 0..15, 0..15]
    // A block at (0,0,0) has neighbors at x=-1, y=-1, z=-1 which are outside loaded chunks
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));
    set_block(&mut doc, &rid, 0, 0, 0, stone());

    let target = make_target(0, 0, 0, 0, 0, 0);
    let analysis = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();

    let cell = analysis.cell_at(Position::new(0, 0, 0)).unwrap();
    assert!(cell.exposed_faces.contains(&VoxelFace::Up));
    assert!(cell.exposed_faces.contains(&VoxelFace::East));
    assert!(cell.exposed_faces.contains(&VoxelFace::South));
    assert!(cell.unknown_faces.contains(&VoxelFace::Down));
    assert!(cell.unknown_faces.contains(&VoxelFace::West));
    assert!(cell.unknown_faces.contains(&VoxelFace::North));
    assert!(cell.has_unknown_boundary());
    assert!(!analysis.components()[0].complete);
}

#[test]
fn test_surface_features_isolated_and_tip() {
    let root = Position::new(0, 0, 0);
    let isolated_cell = SurfaceCell {
        position: root,
        state: stone(),
        exposed_faces: VoxelFace::ALL.to_vec(),
        unknown_faces: Vec::new(),
        occupied_neighbor_count: 0,
        component_root: root,
    };
    let comp = SurfaceComponent {
        root,
        size: 1,
        complete: true,
    };

    let surface = SurfaceAnalysis::new(
        schematic_core::DocumentRevision::default(),
        vec![isolated_cell],
        vec![comp],
    );
    let features = SurfaceFeatureAnalyzer::analyze(&surface);
    let desc = features.descriptor_at(root).unwrap();
    assert_eq!(desc.kind, SurfaceFeatureKind::Isolated);
    assert_eq!(desc.exposed_axis_count, 3);
    assert_eq!(desc.opposite_exposure_pair_count, 3);
    assert!(!desc.has_directional_exposure());
}

#[test]
fn test_one_block_bump_protrusion_evidence() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // 3x3 platform at y=5, bump at (6, 6, 6)
    for x in 5..=7 {
        for z in 5..=7 {
            set_block(&mut doc, &rid, x, 5, z, stone());
        }
    }
    set_block(&mut doc, &rid, 6, 6, 6, stone());

    let target = make_target(5, 5, 5, 7, 6, 7);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    let tip_desc = features.descriptor_at(Position::new(6, 6, 6)).unwrap();
    assert_eq!(tip_desc.kind, SurfaceFeatureKind::Tip);

    let protrusion_req = ProtrusionAnalysisRequest::new(2).unwrap();
    let protrusion_analysis =
        ConnectedProtrusionAnalyzer::analyze(&surface, &features, protrusion_req).unwrap();

    assert_eq!(protrusion_analysis.evidence().len(), 1);
    let ev = &protrusion_analysis.evidence()[0];
    assert_eq!(ev.termination, ProtrusionTermination::JunctionReached);
    assert_eq!(ev.path_positions(), vec![Position::new(6, 6, 6)]);
    assert_eq!(ev.exact_path_length(), Some(1));
    assert_eq!(
        ev.attachment.as_ref().unwrap().position,
        Position::new(6, 5, 6)
    );
    assert_eq!(ev.attachment_direction(), Some(VoxelFace::Down));
    assert_eq!(
        ev.attachment_support_faces,
        vec![
            VoxelFace::North,
            VoxelFace::South,
            VoxelFace::West,
            VoxelFace::East,
        ],
    );
}

#[test]
fn test_bent_decoration_retains_unique_path_and_direction_changes() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // Base at y=5 from x=5..=7, z=5..=7
    for x in 5..=7 {
        for z in 5..=7 {
            set_block(&mut doc, &rid, x, 5, z, stone());
        }
    }
    // Bump of height 2 at (6, 6, 6) and (6, 7, 6)
    set_block(&mut doc, &rid, 6, 6, 6, stone());
    set_block(&mut doc, &rid, 6, 7, 6, stone());
    // Bend at y=7 to x=7 and x=8: (7, 7, 6) and (8, 7, 6)
    set_block(&mut doc, &rid, 7, 7, 6, stone());
    set_block(&mut doc, &rid, 8, 7, 6, stone());

    let target = make_target(5, 5, 5, 8, 7, 7);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    let protrusion_req = ProtrusionAnalysisRequest::new(5).unwrap();
    let protrusion_analysis =
        ConnectedProtrusionAnalyzer::analyze(&surface, &features, protrusion_req).unwrap();

    assert_eq!(protrusion_analysis.evidence().len(), 1);
    let ev = &protrusion_analysis.evidence()[0];
    assert_eq!(ev.termination, ProtrusionTermination::JunctionReached);
    assert_eq!(
        ev.path_positions(),
        vec![
            Position::new(8, 7, 6),
            Position::new(7, 7, 6),
            Position::new(6, 7, 6),
            Position::new(6, 6, 6),
        ]
    );
    assert_eq!(
        ev.step_directions(),
        vec![
            VoxelFace::West,
            VoxelFace::West,
            VoxelFace::Down,
            VoxelFace::Down,
        ]
    );
    assert_eq!(ev.direction_change_count(), 1);
    assert_eq!(ev.exact_path_length(), Some(4));
}

#[test]
fn test_thin_branch_junction_preserves_multiple_tips() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // Center at (6, 6, 6) with 3 branches: (-1, 0, 0), (1, 0, 0), (0, 1, 0)
    set_block(&mut doc, &rid, 6, 6, 6, stone());
    set_block(&mut doc, &rid, 5, 6, 6, stone());
    set_block(&mut doc, &rid, 7, 6, 6, stone());
    set_block(&mut doc, &rid, 6, 7, 6, stone());

    let target = make_target(4, 5, 5, 8, 8, 7);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    let protrusion_req = ProtrusionAnalysisRequest::new(2).unwrap();
    let protrusion_analysis =
        ConnectedProtrusionAnalyzer::analyze(&surface, &features, protrusion_req).unwrap();

    let tips: Vec<Position> = protrusion_analysis
        .evidence()
        .iter()
        .map(|e| e.tip_position())
        .collect();
    assert_eq!(
        tips,
        vec![
            Position::new(5, 6, 6),
            Position::new(6, 7, 6),
            Position::new(7, 6, 6),
        ]
    );
    for trace in protrusion_analysis.evidence() {
        assert_eq!(trace.termination, ProtrusionTermination::JunctionReached);
        assert_eq!(trace.component.size, 4);
        assert_eq!(
            trace.attachment.as_ref().unwrap().kind,
            SurfaceFeatureKind::ThinFeature
        );
        assert_eq!(trace.attachment_support_faces.len(), 2);
        assert_eq!(trace.observed_path_length(), 1);
    }
}
