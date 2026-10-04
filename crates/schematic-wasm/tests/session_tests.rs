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

#[test]
fn test_structural_editing_session() {
    let raw_bytes = create_sample_litematic();
    let mut session = Session::new();
    session.load_litematic(&raw_bytes).expect("load litematic");

    // 1. Fill 2x1x2 with oak_planks
    let fill_req = schematic_wasm::FillRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [2, 2, 2],
            max: [3, 2, 3],
        },
        block: "minecraft:oak_planks".to_string(),
    };
    let fill_res = session.preview_fill(fill_req).expect("preview fill");
    assert_eq!(fill_res.changed_count, 4);
    session.commit_preview().expect("commit fill");

    // 2. Copy the 2x1x2
    let copy_req = schematic_wasm::CopyRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [2, 2, 2],
            max: [3, 2, 3],
        },
    };
    let count = session.copy_selection(copy_req).expect("copy");
    assert_eq!(count, 4);

    // 3. Paste at (6, 2, 6)
    let paste_req = schematic_wasm::PasteRequest {
        region_id: "MainRegion".to_string(),
        target: [6, 2, 6],
    };
    let paste_res = session.preview_paste(paste_req).expect("preview paste");
    assert_eq!(paste_res.changed_count, 4);
    session.commit_preview().expect("commit paste");

    // 4. Move pasted blocks by (0, 1, 0)
    let move_req = schematic_wasm::MoveRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [6, 2, 6],
            max: [7, 2, 7],
        },
        delta: [0, 1, 0],
    };
    let move_res = session.preview_move(move_req).expect("preview move");
    assert_eq!(move_res.changed_count, 8); // 4 cleared, 4 set
    session.commit_preview().expect("commit move");

    // 5. Rotate the moved blocks: fill a 2x1x1 bar at [6, 3, 6]..[7, 3, 6] within a 2x1x2 bounding box
    let rotate_req = schematic_wasm::RotateRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [6, 3, 6],
            max: [7, 3, 7],
        },
        angle_deg: 90,
    };
    // Prior to rotation, only z=6 had blocks, z=7 was empty.
    // Clear z=7 and keep only [6, 3, 6] and [7, 3, 6]:
    let fill_single = schematic_wasm::FillRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [6, 3, 7],
            max: [7, 3, 7],
        },
        block: "minecraft:air".to_string(),
    };
    session.preview_fill(fill_single).expect("clear row");
    session.commit_preview().expect("commit clear");

    let rotate_res = session.preview_rotate(rotate_req).expect("preview rotate");
    assert!(rotate_res.can_commit);
    session.commit_preview().expect("commit rotate");

    // 6. Mirror along X
    let mirror_req = schematic_wasm::MirrorRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [6, 3, 6],
            max: [7, 3, 7],
        },
        axis: "x".to_string(),
    };
    let mirror_res = session.preview_mirror(mirror_req).expect("preview mirror");
    assert!(mirror_res.can_commit);
    session.commit_preview().expect("commit mirror");

    // 7. Undo mirror and rotate
    session.undo().expect("undo mirror");
    session.undo().expect("undo rotate");
}

#[test]
fn test_diff_with_source_in_session() {
    let mut session = Session::new();
    let bytes = create_sample_litematic();
    session.load_litematic(&bytes).expect("load litematic");

    // Before edits: diff with source is empty
    let diff_initial = session.diff_with_source().expect("diff with source");
    assert_eq!(diff_initial.total_added, 0);
    assert_eq!(diff_initial.total_removed, 0);
    assert_eq!(diff_initial.total_modified, 0);

    // Perform an edit: modify [5, 5, 5] (which was stone) to diamond_block
    let fill_mod = schematic_wasm::FillRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [5, 5, 5],
            max: [5, 5, 5],
        },
        block: "minecraft:diamond_block".to_string(),
    };
    session.preview_fill(fill_mod).expect("preview fill");
    session.commit_preview().expect("commit fill");

    // Perform an edit: add [5, 6, 5] (which was air) as gold_block
    let fill_add = schematic_wasm::FillRequest {
        region_id: "MainRegion".to_string(),
        selection: SelectionBounds {
            min: [5, 6, 5],
            max: [5, 6, 5],
        },
        block: "minecraft:gold_block".to_string(),
    };
    session.preview_fill(fill_add).expect("preview fill add");
    session.commit_preview().expect("commit fill add");

    // After commit: diff with source has 1 added and 1 modified block
    let diff_edited = session
        .diff_with_source()
        .expect("diff with source after edit");
    assert_eq!(diff_edited.total_modified, 1);
    assert_eq!(diff_edited.total_added, 1);
    assert_eq!(diff_edited.block_diffs.len(), 2);
}

#[test]
fn test_multi_format_session() {
    let mut session = Session::new();
    let litematic_bytes = create_sample_litematic();
    let summary1 = session
        .load_schematic(&litematic_bytes)
        .expect("load litematic");
    assert_eq!(summary1.regions.len(), 1);
    assert_eq!(summary1.regions[0].non_air_blocks, 12);

    // Export to Sponge .schem
    let sponge_bytes = session.export_sponge().expect("export sponge");

    // Load Sponge .schem into new session
    let mut session2 = Session::new();
    let summary2 = session2.load_schematic(&sponge_bytes).expect("load sponge");
    assert_eq!(summary2.regions.len(), 1);
    assert_eq!(summary2.regions[0].non_air_blocks, 12);

    // Export to Structure .nbt
    let structure_bytes = session2.export_structure().expect("export structure");

    // Load Structure .nbt into new session
    let mut session3 = Session::new();
    let summary3 = session3
        .load_schematic(&structure_bytes)
        .expect("load structure");
    assert_eq!(summary3.regions.len(), 1);
    assert_eq!(summary3.regions[0].non_air_blocks, 12);

    // Diff session3 with sponge_bytes (both have region 'Main')
    let diff = session3
        .diff_with_bytes(&sponge_bytes)
        .expect("diff with sponge");
    assert_eq!(diff.total_added, 0);
    assert_eq!(diff.total_removed, 0);
    assert_eq!(diff.total_modified, 0);
}

#[test]
fn test_analyze_document_in_session() {
    let mut session = Session::new();
    let bytes = create_sample_litematic();
    session.load_litematic(&bytes).expect("load litematic");

    let report = session.analyze(None).expect("analyze document");
    assert_eq!(report.statistics.non_air_blocks, 12);
    assert_eq!(report.statistics.region_count, 1);
    assert_eq!(report.statistics.dimensions, [16, 16, 16]);
    assert!(report.statistics.total_volume >= 4096);
    assert!(report.statistics.surface_cell_count > 0);
    assert!(report.statistics.island_count > 0);

    // Verify materials
    assert_eq!(report.materials.len(), 2); // stone and granite
    let stone = report
        .materials
        .iter()
        .find(|m| m.id == "minecraft:stone")
        .unwrap();
    assert_eq!(stone.count, 11); // 10 in row + 1 bump
    assert_eq!(stone.stacks_64, 0);
    assert_eq!(stone.remainder, 11);
    assert!((stone.percentage - (11.0 / 12.0 * 100.0)).abs() < 0.1);

    let granite = report
        .materials
        .iter()
        .find(|m| m.id == "minecraft:granite")
        .unwrap();
    assert_eq!(granite.count, 1);
    assert_eq!(granite.stacks_64, 0);
    assert_eq!(granite.remainder, 1);
    assert!((granite.percentage - (1.0 / 12.0 * 100.0)).abs() < 0.1);

    // Verify features breakdown
    assert!(!report.statistics.feature_counts.is_empty());
}
