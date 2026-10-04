use schematic_core::{
    BlockPosition, BlockState, Document, DocumentMetadata, Position, Region, RegionId, Size,
};
use schematic_format::LitematicDocument;
use schematic_wasm::{CleanupRequest, ReplaceRequest, SelectionBounds, Session};

fn create_sample_litematic() -> Vec<u8> {
    let meta = DocumentMetadata {
        name: Some("WasmTestSchematic".to_string()),
        author: Some("TestBot".to_string()),
        description: Some("Integration test for wasm session".to_string()),
        ..Default::default()
    };

    let mut doc = Document::new(meta);
    let region_id = RegionId::new("MainRegion");
    let origin = Position::new(0, 0, 0);
    let size = Size::new(16, 16, 16);
    let mut region = Region::new(region_id.clone(), origin, size);

    let stone = BlockState::new("minecraft:stone", []).unwrap();
    let granite = BlockState::new("minecraft:granite", []).unwrap();
    let stone_idx = region.palette_mut().intern(stone).unwrap();
    let granite_idx = region.palette_mut().intern(granite).unwrap();

    // Place 10 stone blocks in a row
    for x in 0..10 {
        region
            .set_block_index(BlockPosition::new(x, 1, 1), Some(stone_idx))
            .unwrap();
    }
    // Place 1 granite block
    region
        .set_block_index(BlockPosition::new(12, 1, 1), Some(granite_idx))
        .unwrap();

    // Place an isolated 1-block bump for cleanup test
    region
        .set_block_index(BlockPosition::new(5, 5, 5), Some(stone_idx))
        .unwrap();

    doc.insert_region(region);
    let litematic = LitematicDocument::from_document(doc, 2975);
    litematic.export(&litematic.document).unwrap()
}

#[test]
fn test_export_fixture_file() {
    let bytes = create_sample_litematic();
    let dest_dir = std::path::Path::new("../../apps/web/public");
    if dest_dir.exists() {
        std::fs::write(dest_dir.join("sample.litematic"), &bytes).unwrap();
    }
}

#[test]
fn test_wasm_session_full_lifecycle() {
    let raw_bytes = create_sample_litematic();
    let mut session = Session::new();

    // 1. Load document
    let summary = session.load_litematic(&raw_bytes).expect("load success");
    assert_eq!(summary.name, "WasmTestSchematic");
    assert_eq!(summary.author, "TestBot");
    assert_eq!(summary.regions.len(), 1);
    assert_eq!(summary.regions[0].name, "MainRegion");
    assert_eq!(summary.regions[0].non_air_blocks, 12);

    let status = session.get_status();
    assert!(status.loaded);
    assert!(!status.has_preview);
    assert!(!status.can_undo);
    assert!(!status.is_dirty);

    // 2. Query mesh data
    let mesh = session.get_region_mesh("MainRegion").expect("get mesh");
    assert_eq!(mesh.region_id, "MainRegion");
    assert_eq!(mesh.origin, [0, 0, 0]);
    assert_eq!(mesh.size, [16, 16, 16]);
    // 12 blocks * 4 ints (x, y, z, pal_idx) = 48 ints
    assert_eq!(mesh.blocks.len(), 48);
    assert!(mesh.preview_diff.is_none());

    // 3. Preview replace stone -> diorite
    let replace_req = ReplaceRequest {
        region_id: "MainRegion".to_string(),
        selection: Some(SelectionBounds {
            min: [0, 0, 0],
            max: [10, 5, 5],
        }),
        from_block: "minecraft:stone".to_string(),
        to_block: "minecraft:diorite".to_string(),
    };
    let prev_summary = session
        .preview_replace(replace_req)
        .expect("preview replace");
    // In box [0..10, 0..5, 0..5]: positions (0..10, 1, 1) are stone = 10 blocks. (5, 5, 5) is stone = 1 block. Total = 11.
    assert_eq!(prev_summary.changed_count, 11);
    assert!(prev_summary.can_commit);

    let status = session.get_status();
    assert!(status.has_preview);

    // Check mesh with active preview
    let mesh_preview = session
        .get_region_mesh("MainRegion")
        .expect("mesh with preview");
    assert!(mesh_preview.preview_diff.is_some());
    let diff = mesh_preview.preview_diff.unwrap();
    assert_eq!(diff.modified_positions.len(), 11);

    // 4. Commit replace
    let hist = session.commit_preview().expect("commit");
    assert!(hist.can_undo);
    assert!(!hist.can_redo);
    assert!(!hist.has_preview);
    assert!(hist.is_dirty);

    // 5. Test Undo
    let hist_undo = session.undo().expect("undo");
    assert!(!hist_undo.can_undo);
    assert!(hist_undo.can_redo);
    assert!(!hist_undo.is_dirty);

    // 6. Test Redo
    let hist_redo = session.redo().expect("redo");
    assert!(hist_redo.can_undo);
    assert!(!hist_redo.can_redo);
    assert!(hist_redo.is_dirty);

    // 7. Preview Cleanup (remove small islands <= 2 blocks)
    // Note: granite at (12, 1, 1) is isolated with size 1!
    let cleanup_req = CleanupRequest {
        region_id: "MainRegion".to_string(),
        max_size: 2,
        replacement_block: Some("minecraft:air".to_string()),
        protected_kinds: vec![],
    };
    let clean_prev = session
        .preview_cleanup(cleanup_req)
        .expect("preview cleanup");
    assert!(clean_prev.changed_count >= 1);

    // Commit cleanup
    let hist_clean = session.commit_preview().expect("commit cleanup");
    assert!(hist_clean.can_undo);

    // 8. Export Litematic
    let exported_bytes = session.export_litematic().expect("export litematic");
    assert!(!exported_bytes.is_empty());

    // 9. Reload exported file into new session
    let mut session2 = Session::new();
    let summary2 = session2
        .load_litematic(&exported_bytes)
        .expect("reload exported");
    assert_eq!(summary2.name, "WasmTestSchematic");
    assert_eq!(summary2.author, "TestBot");
    assert_eq!(summary2.description, "Integration test for wasm session");
    assert_eq!(summary2.regions.len(), 1);
    // Verified roundtrip!
}

#[test]
fn test_inspect_and_validate_session() {
    let raw_bytes = create_sample_litematic();
    let mut session = Session::new();
    session.load_litematic(&raw_bytes).expect("load litematic");

    // 1. Inspect block at (0, 1, 1) -> minecraft:stone
    let req = schematic_wasm::BlockInspectionRequest {
        region_id: "MainRegion".to_string(),
        x: 0,
        y: 1,
        z: 1,
    };
    let inspection = session
        .inspect_block(&req)
        .expect("inspect block")
        .expect("block exists");
    assert_eq!(inspection.block_id, "minecraft:stone");
    assert_eq!(inspection.local_position, [0, 1, 1]);
    assert_eq!(inspection.world_position, [0, 1, 1]);

    // 2. Inspect document
    let doc_inspect = session.inspect_document().expect("inspect document");
    assert_eq!(
        doc_inspect.metadata.name,
        Some("WasmTestSchematic".to_string())
    );
    assert_eq!(doc_inspect.regions.len(), 1);
    assert_eq!(doc_inspect.regions[0].name, "MainRegion");

    // 3. Validate document
    let diagnostics = session.validate_document().expect("validate document");
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == schematic_wasm::DiagnosticSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "Expected no errors in sample document");
}
