use schematic_analysis::{
    NonAirPolicy, SurfaceAnalyzer, SurfaceFeatureAnalyzer, SurfaceFeatureKind,
};
use schematic_core::{
    BlockPosition, BlockState, Document, DocumentMetadata, OperationTarget, Position, Region,
    RegionId, Selection, Size,
};
use schematic_edit::{
    CleanupError, DisconnectedIslandCleanupPlanner, EditWorkspace, IslandCleanupRequest,
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

fn air() -> BlockState {
    BlockState::new("minecraft:air", []).unwrap()
}

#[test]
fn test_removes_unprotected_small_island() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // Isolated stone block at (5, 5, 5)
    set_block(&mut doc, &rid, 5, 5, 5, stone());

    let target = make_target(4, 4, 4, 6, 6, 6);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    let request = IslandCleanupRequest::new(1, air(), []).unwrap();
    let patch =
        DisconnectedIslandCleanupPlanner::plan(&doc, &rid, &surface, &features, &request).unwrap();

    assert_eq!(patch.changed_block_count(), 1);
    let change = patch.change_at(&rid, BlockPosition::new(5, 5, 5)).unwrap();
    assert!(change.before().is_some());
    assert_eq!(change.after(), None);
}

#[test]
fn test_preserves_component_larger_than_threshold() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // Island of size 2 at (5, 5, 5) and (5, 5, 6)
    set_block(&mut doc, &rid, 5, 5, 5, stone());
    set_block(&mut doc, &rid, 5, 5, 6, stone());

    let target = make_target(4, 4, 4, 6, 6, 7);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    // Max component size is 1, so size 2 should be preserved
    let request = IslandCleanupRequest::new(1, air(), []).unwrap();
    let patch =
        DisconnectedIslandCleanupPlanner::plan(&doc, &rid, &surface, &features, &request).unwrap();

    assert!(patch.is_empty());
}

#[test]
fn test_vetoes_entire_component_if_any_cell_is_protected() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // Island of size 2: (5, 5, 5) and (5, 5, 6). Both cells have 1 neighbor, so both are TIPs!
    set_block(&mut doc, &rid, 5, 5, 5, stone());
    set_block(&mut doc, &rid, 5, 5, 6, stone());

    let target = make_target(4, 4, 4, 6, 6, 7);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    // Max size 2 (eligible by size), but TIP is protected -> entire component preserved!
    let request = IslandCleanupRequest::new(2, air(), [SurfaceFeatureKind::Tip]).unwrap();
    let patch =
        DisconnectedIslandCleanupPlanner::plan(&doc, &rid, &surface, &features, &request).unwrap();

    assert!(patch.is_empty());
}

#[test]
fn test_preserves_incomplete_or_unknown_boundary_island() {
    let rid = RegionId::new("test");
    // Only chunk (0,0,0) loaded
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));

    // Block at (0, 0, 0) has unknown boundary because neighbors x=-1, y=-1, z=-1 are outside
    set_block(&mut doc, &rid, 0, 0, 0, stone());

    let target = make_target(0, 0, 0, 0, 0, 0);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    let request = IslandCleanupRequest::new(1, air(), []).unwrap();
    let patch =
        DisconnectedIslandCleanupPlanner::plan(&doc, &rid, &surface, &features, &request).unwrap();

    // Unknown boundary must veto cleanup!
    assert!(patch.is_empty());
}

#[test]
fn test_rejects_stale_surface_analysis() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));
    set_block(&mut doc, &rid, 5, 5, 5, stone());

    let target = make_target(4, 4, 4, 6, 6, 6);
    let surface = SurfaceAnalyzer::analyze(&doc, &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    // Mutate document to increment revision
    set_block(&mut doc, &rid, 6, 6, 6, stone());

    let request = IslandCleanupRequest::new(1, air(), []).unwrap();
    let err = DisconnectedIslandCleanupPlanner::plan(&doc, &rid, &surface, &features, &request)
        .unwrap_err();

    assert_eq!(err, CleanupError::StaleSurfaceAnalysis);
}

#[test]
fn test_workspace_preview_and_commit_cleanup_flow() {
    let rid = RegionId::new("test");
    let mut doc = create_test_doc(&rid, (0, 0, 0), (0, 0, 0));
    set_block(&mut doc, &rid, 5, 5, 5, stone());

    let mut ws = EditWorkspace::start(doc);
    assert!(!ws.is_dirty());

    let target = make_target(4, 4, 4, 6, 6, 6);
    let surface = SurfaceAnalyzer::analyze(ws.committed(), &rid, &target, &NonAirPolicy).unwrap();
    let features = SurfaceFeatureAnalyzer::analyze(&surface);

    let request = IslandCleanupRequest::new(1, air(), []).unwrap();
    ws.preview_island_cleanup(&rid, &surface, &features, &request)
        .unwrap();

    assert!(ws.has_preview());
    // Preview shows block is removed (None)
    assert_eq!(
        ws.preview_block_at(&rid, BlockPosition::new(5, 5, 5))
            .unwrap(),
        None
    );

    // Commit preview
    ws.commit_preview().unwrap();
    assert!(!ws.has_preview());
    assert!(ws.can_undo());
    assert!(ws.is_dirty());

    // After commit, block is indeed gone in committed document
    assert_eq!(
        ws.committed()
            .region(&rid)
            .unwrap()
            .block_index_at(BlockPosition::new(5, 5, 5))
            .unwrap(),
        None
    );

    // Undo
    ws.undo().unwrap();
    assert!(ws.can_redo());
    assert!(!ws.is_dirty());
    let restored_idx = ws
        .committed()
        .region(&rid)
        .unwrap()
        .block_index_at(BlockPosition::new(5, 5, 5))
        .unwrap();
    assert!(restored_idx.is_some());

    // Redo
    ws.redo().unwrap();
    assert!(ws.can_undo());
    assert!(ws.is_dirty());
    assert_eq!(
        ws.committed()
            .region(&rid)
            .unwrap()
            .block_index_at(BlockPosition::new(5, 5, 5))
            .unwrap(),
        None
    );
}
