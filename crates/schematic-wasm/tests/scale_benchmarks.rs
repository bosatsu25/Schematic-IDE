use schematic_core::{
    BlockPosition, BlockState, Document, DocumentMetadata, Position, Region, RegionId, Size,
};
use schematic_format::LitematicDocument;
use schematic_wasm::{ReplaceRequest, SelectionBounds, Session};
use std::time::Instant;

/// Benchmark result structure capturing latency and throughput
#[derive(Debug)]
pub struct ScaleBenchmarkResult {
    pub scale_name: &'static str,
    pub total_voxels: usize,
    pub non_air_blocks: usize,
    pub export_bytes: usize,
    pub export_duration_ms: f64,
    pub parse_duration_ms: f64,
    pub mesh_duration_ms: f64,
    pub edit_duration_ms: f64,
    pub undo_duration_ms: f64,
    pub redo_duration_ms: f64,
}

fn create_scale_litematic(
    width: u32,
    height: u32,
    length: u32,
    step: u32,
) -> (Vec<u8>, usize, usize) {
    let meta = DocumentMetadata {
        name: Some(format!("ScaleTest_{}x{}x{}", width, height, length)),
        author: Some("BenchBot".to_string()),
        description: Some("Scale performance benchmark test".to_string()),
        ..Default::default()
    };

    let mut doc = Document::new(meta);
    let region_id = RegionId::new("MainRegion");
    let origin = Position::new(0, 0, 0);
    let size = Size::new(width, height, length);
    let mut region = Region::new(region_id.clone(), origin, size);

    let stone = BlockState::new("minecraft:stone", []).unwrap();
    let granite = BlockState::new("minecraft:granite", []).unwrap();
    let stone_idx = region.palette_mut().intern(stone).unwrap();
    let granite_idx = region.palette_mut().intern(granite).unwrap();

    let mut block_count = 0;
    // Populate sparse structure (e.g. floors/pillars/walls every `step` voxels)
    for y in (0..height).step_by(step as usize) {
        for z in (0..length).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let idx = if (x + y + z) % 2 == 0 {
                    stone_idx
                } else {
                    granite_idx
                };
                region
                    .set_block_index(BlockPosition::new(x as i64, y as i64, z as i64), Some(idx))
                    .unwrap();
                block_count += 1;
            }
        }
    }

    doc.insert_region(region);
    let total_voxels = (width * height * length) as usize;

    let start_export = Instant::now();
    let litematic = LitematicDocument::from_document(doc, 2975);
    let bytes = litematic.export(&litematic.document).unwrap();
    let _export_elapsed = start_export.elapsed();

    (bytes, total_voxels, block_count)
}

fn run_scale_benchmark(
    scale_name: &'static str,
    width: u32,
    height: u32,
    length: u32,
    step: u32,
) -> ScaleBenchmarkResult {
    let (bytes, total_voxels, non_air_blocks) = create_scale_litematic(width, height, length, step);

    // 1. Benchmark Parsing into WASM Session
    let mut session = Session::new();
    let start_parse = Instant::now();
    session.load_litematic(&bytes).unwrap();
    let parse_duration_ms = start_parse.elapsed().as_secs_f64() * 1000.0;

    // 2. Benchmark Mesh extraction
    let start_mesh = Instant::now();
    let mesh = session.get_region_mesh("MainRegion").unwrap();
    let mesh_duration_ms = start_mesh.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(mesh.blocks.len(), non_air_blocks * 4);

    // 3. Benchmark Bounded Edit (Replace)
    let start_edit = Instant::now();
    let preview = session
        .preview_replace(ReplaceRequest {
            region_id: "MainRegion".to_string(),
            from_block: "minecraft:stone".to_string(),
            to_block: "minecraft:diorite".to_string(),
            selection: Some(SelectionBounds {
                min: [0, 0, 0],
                max: [(width / 2) as i32, (height / 2) as i32, (length / 2) as i32],
            }),
        })
        .unwrap();
    assert!(preview.changed_count > 0);
    session.commit_preview().unwrap();
    let edit_duration_ms = start_edit.elapsed().as_secs_f64() * 1000.0;

    // 4. Benchmark Undo
    let start_undo = Instant::now();
    let hist_undo = session.undo().unwrap();
    assert!(hist_undo.can_redo);
    let undo_duration_ms = start_undo.elapsed().as_secs_f64() * 1000.0;

    // 5. Benchmark Redo
    let start_redo = Instant::now();
    let hist_redo = session.redo().unwrap();
    assert!(hist_redo.can_undo);
    let redo_duration_ms = start_redo.elapsed().as_secs_f64() * 1000.0;

    // 6. Benchmark Export
    let start_export = Instant::now();
    let exported = session.export_litematic().unwrap();
    let export_duration_ms = start_export.elapsed().as_secs_f64() * 1000.0;
    assert!(!exported.is_empty());

    let result = ScaleBenchmarkResult {
        scale_name,
        total_voxels,
        non_air_blocks,
        export_bytes: exported.len(),
        export_duration_ms,
        parse_duration_ms,
        mesh_duration_ms,
        edit_duration_ms,
        undo_duration_ms,
        redo_duration_ms,
    };

    println!(
        "\n--- [SCALE BENCHMARK: {}] ---\n  Total Volume: {} voxels ({} x {} x {})\n  Non-Air Blocks: {}\n  Export File Size: {} KB\n  Parse Latency: {:.2} ms\n  Mesh Extraction: {:.2} ms\n  Edit (Replace+Commit): {:.2} ms\n  Undo Latency: {:.2} ms\n  Redo Latency: {:.2} ms\n  Export Latency: {:.2} ms",
        result.scale_name,
        result.total_voxels,
        width,
        height,
        length,
        result.non_air_blocks,
        result.export_bytes / 1024,
        result.parse_duration_ms,
        result.mesh_duration_ms,
        result.edit_duration_ms,
        result.undo_duration_ms,
        result.redo_duration_ms,
        result.export_duration_ms,
    );

    result
}

#[test]
fn test_scale_benchmark_1m_blocks() {
    // 1M blocks scale: 100 x 100 x 100 = 1,000,000 voxels, step = 3 (~37,000 non-air blocks)
    let res = run_scale_benchmark("1M Blocks", 100, 100, 100, 3);
    assert_eq!(res.total_voxels, 1_000_000);
    // Performance budgets
    assert!(
        res.parse_duration_ms < 500.0,
        "1M parse took too long: {:.2}ms",
        res.parse_duration_ms
    );
    assert!(
        res.mesh_duration_ms < 250.0,
        "1M mesh took too long: {:.2}ms",
        res.mesh_duration_ms
    );
    assert!(
        res.edit_duration_ms < 200.0,
        "1M edit took too long: {:.2}ms",
        res.edit_duration_ms
    );
    assert!(
        res.undo_duration_ms < 100.0,
        "1M undo took too long: {:.2}ms",
        res.undo_duration_ms
    );
}

#[test]
fn test_scale_benchmark_5m_blocks() {
    // 5M blocks scale: 200 x 125 x 200 = 5,000,000 voxels, step = 4 (~67,000 non-air blocks)
    let res = run_scale_benchmark("5M Blocks", 200, 125, 200, 4);
    assert_eq!(res.total_voxels, 5_000_000);
    // Performance budgets
    assert!(
        res.parse_duration_ms < 2000.0,
        "5M parse took too long: {:.2}ms",
        res.parse_duration_ms
    );
    assert!(
        res.mesh_duration_ms < 1000.0,
        "5M mesh took too long: {:.2}ms",
        res.mesh_duration_ms
    );
    assert!(
        res.edit_duration_ms < 500.0,
        "5M edit took too long: {:.2}ms",
        res.edit_duration_ms
    );
}

#[test]
fn test_scale_benchmark_10m_blocks() {
    // 10M blocks scale: 250 x 160 x 250 = 10,000,000 voxels, step = 5 (~133,000 non-air blocks)
    let res = run_scale_benchmark("10M Blocks", 250, 160, 250, 5);
    assert_eq!(res.total_voxels, 10_000_000);
    // Performance budgets
    assert!(
        res.parse_duration_ms < 4000.0,
        "10M parse took too long: {:.2}ms",
        res.parse_duration_ms
    );
    assert!(
        res.mesh_duration_ms < 2000.0,
        "10M mesh took too long: {:.2}ms",
        res.mesh_duration_ms
    );
    assert!(
        res.edit_duration_ms < 1000.0,
        "10M edit took too long: {:.2}ms",
        res.edit_duration_ms
    );
}
