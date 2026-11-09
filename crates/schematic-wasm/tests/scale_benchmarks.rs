//! Large-structure scale benchmarks for the native `Session` pipeline.
//!
//! The 1M / 5M / 10M runs are `#[ignore]`d so they never slow down or flake
//! the default `cargo test` run. Run them in release mode, one at a time:
//!
//! ```text
//! cargo test --release -p schematic-wasm --test scale_benchmarks -- \
//!     --ignored --nocapture --test-threads=1
//! ```
//!
//! Only release builds enforce the (deliberately generous) regression budgets;
//! debug builds print the numbers and skip the budget asserts. The
//! `smoke_dense_32_cubed` test runs in the default suite with no timing
//! asserts so the code path stays covered. See `docs/benchmarks.md`.

#[path = "support/dense_fixture.rs"]
mod dense_fixture;

use dense_fixture::{DenseFixture, Scale, REGION_NAME, SCALE_10M, SCALE_1M, SCALE_5M, SMOKE_32};
use schematic_analysis::{NonAirPolicy, SurfaceAnalyzer, SurfaceFeatureAnalyzer};
use schematic_core::{OperationTarget, RegionId, Selection};
use schematic_format::LitematicDocument;
use schematic_wasm::{ReplaceRequest, SelectionBounds, Session};
use std::time::Instant;

/// Measured numbers for one scale.
#[derive(Debug)]
struct ScaleReport {
    fixture_bytes: usize,
    load_ms: f64,
    mesh_ms: f64,
    mesh_json_ms: f64,
    mesh_json_bytes: usize,
    view_ms: f64,
    faces_ms: f64,
    faces_bytes: usize,
    face_chunks: usize,
    face_count: usize,
    faces_peak_mb: Option<f64>,
    inspect_ms: f64,
    validate_ms: f64,
    analyze_ms: f64,
    surface_ms: f64,
    edit_ms: f64,
    edit_changed: usize,
    undo_ms: f64,
    redo_ms: f64,
    export_ms: f64,
    export_bytes: usize,
    load_peak_mb: Option<f64>,
    mesh_peak_mb: Option<f64>,
    analyze_peak_mb: Option<f64>,
}

/// Generous regression ceilings (milliseconds), enforced only in release builds.
struct Budget {
    load_ms: f64,
    mesh_ms: f64,
    mesh_json_ms: f64,
    faces_ms: f64,
    analyze_ms: f64,
    edit_ms: f64,
    undo_ms: f64,
    export_ms: f64,
}

fn ms_since(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// Peak resident set size of this process in MiB (Linux only).
fn peak_rss_mb() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kb: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024.0)
}

/// Reset the kernel's peak-RSS counter so the next `peak_rss_mb()` reflects
/// only what happened since (Linux >= 4.0; silently ignored elsewhere). The
/// counter is process-wide, so run with `--test-threads=1`.
fn reset_peak_rss() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

fn time<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = Instant::now();
    let out = f();
    (out, ms_since(start))
}

/// Like [`time`], but also returns the process peak RSS reached during `f`.
fn time_with_peak<T>(f: impl FnOnce() -> T) -> (T, f64, Option<f64>) {
    reset_peak_rss();
    let (out, ms) = time(f);
    (out, ms, peak_rss_mb())
}

fn fmt_mb(mb: Option<f64>) -> String {
    mb.map(|mb| format!("{mb:.0} MiB"))
        .unwrap_or_else(|| "n/a".to_string())
}

/// Parse the `region_faces` stream header: i32 LE `[chunk_count, per chunk:
/// cx, cy, cz, n, then n packed u32 faces]`. Returns `(chunk_count, total_faces)`.
fn decode_face_counts(bytes: &[u8]) -> (usize, usize) {
    let word = |i: usize| i32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    let chunks = word(0) as usize;
    let mut faces = 0usize;
    let mut i = 1;
    for _ in 0..chunks {
        let n = word(i + 3) as usize;
        faces += n;
        i += 4 + n;
    }
    assert_eq!(i * 4, bytes.len(), "face stream length mismatch");
    (chunks, faces)
}

fn run_pipeline(fixture: &DenseFixture) -> ScaleReport {
    let Scale {
        width,
        height,
        length,
        ..
    } = fixture.scale;
    let mut session = Session::new();

    // 1. Parse `.litematic` bytes into an editable session (what the WASM
    //    `schematic_load_litematic` export does before serializing the summary).
    let (summary, load_ms, load_peak_mb) =
        time_with_peak(|| session.load_litematic(&fixture.litematic).unwrap());
    assert_eq!(summary.regions.len(), 1);
    assert_eq!(summary.regions[0].non_air_blocks, fixture.occupied_blocks);

    // 2. Mesh extraction (flat [x, y, z, palette] array) and 3. its JSON
    //    serialization, exactly as the WASM C-ABI does before handing it to
    //    JavaScript.
    reset_peak_rss();
    let (mesh, mesh_ms) = time(|| session.get_region_mesh(REGION_NAME).unwrap());
    assert_eq!(mesh.blocks.len(), fixture.occupied_blocks * 4);
    let (_, mesh_json_ms) = time(|| session.set_json_response(&mesh));
    let mesh_peak_mb = peak_rss_mb();
    let mesh_json_bytes = session.response_buffer().len();
    drop(mesh);
    session.set_json_response(&());

    // 3b. The binary face-culled mesh the renderer now uses: region metadata
    //     (`region_view`) plus every chunk's exposed faces (`region_faces`).
    reset_peak_rss();
    let (_, view_ms) = time(|| session.region_view(REGION_NAME).unwrap());
    let (faces, faces_ms) = time(|| session.region_faces(REGION_NAME, None).unwrap());
    let faces_peak_mb = peak_rss_mb();
    let (face_chunks, face_count) = decode_face_counts(&faces);
    assert!(face_count > 0);
    let faces_bytes = faces.len();
    drop(faces);

    // 4. Panels the web app computes right after load (inspector, problems,
    //    analysis).
    let (_, inspect_ms) = time(|| session.inspect_document().unwrap());
    let (_, validate_ms) = time(|| session.validate_document().unwrap());
    let (_, analyze_ms, analyze_peak_mb) = time_with_peak(|| session.analyze(None).unwrap());

    // 4b. Attribute the analysis cost: the surface pass on its own.
    let parsed = LitematicDocument::parse(&fixture.litematic).unwrap();
    let region = parsed.document.region(&RegionId::new(REGION_NAME)).unwrap();
    let target = OperationTarget::unconstrained(Selection::from_bounds(region.bounds()));
    let (_, surface_ms) = time(|| {
        let surface =
            SurfaceAnalyzer::analyze(&parsed.document, region.id(), &target, &NonAirPolicy)
                .unwrap();
        SurfaceFeatureAnalyzer::analyze(&surface);
    });
    drop(parsed);

    // 5. Bounded replace (preview + commit) over one octant.
    let (edit_changed, edit_ms) = time(|| {
        let preview = session
            .preview_replace(ReplaceRequest {
                region_id: REGION_NAME.to_string(),
                from_block: "minecraft:stone".to_string(),
                to_block: "minecraft:diorite".to_string(),
                selection: Some(SelectionBounds {
                    min: [0, 0, 0],
                    max: [(width / 2) as i32, (height / 2) as i32, (length / 2) as i32],
                    ..Default::default()
                }),
            })
            .unwrap();
        session.commit_preview().unwrap();
        preview.changed_count
    });
    assert!(edit_changed > 0);

    // 6. Undo / redo.
    let (undo, undo_ms) = time(|| session.undo().unwrap());
    assert!(undo.can_redo);
    let (redo, redo_ms) = time(|| session.redo().unwrap());
    assert!(redo.can_undo);

    // 7. Re-export.
    let (exported, export_ms) = time(|| session.export_litematic().unwrap());
    let reparsed = LitematicDocument::parse(&exported).unwrap();
    let reparsed_blocks: usize = reparsed
        .document
        .regions()
        .flat_map(|r| r.chunks())
        .map(|(_, c)| c.occupied_blocks().count())
        .sum();
    assert_eq!(reparsed_blocks, fixture.occupied_blocks);
    assert_eq!(
        reparsed.document.block_entities().count(),
        fixture.block_entities
    );

    ScaleReport {
        fixture_bytes: fixture.litematic.len(),
        load_ms,
        mesh_ms,
        mesh_json_ms,
        mesh_json_bytes,
        view_ms,
        faces_ms,
        faces_bytes,
        face_chunks,
        face_count,
        faces_peak_mb,
        inspect_ms,
        validate_ms,
        analyze_ms,
        surface_ms,
        edit_ms,
        edit_changed,
        undo_ms,
        redo_ms,
        export_ms,
        export_bytes: exported.len(),
        load_peak_mb,
        mesh_peak_mb,
        analyze_peak_mb,
    }
}

fn print_report(fixture: &DenseFixture, r: &ScaleReport) {
    let s = fixture.scale;
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    println!(
        "\n--- [DENSE SCALE BENCHMARK: {name}, {profile}] ---\n\
         \x20 Dimensions:        {w} x {h} x {l} = {vox} voxels\n\
         \x20 Occupied blocks:   {occ} ({fill:.1}% fill)\n\
         \x20 Palette states:    {pal} (+ air)\n\
         \x20 Block entities:    {be}\n\
         \x20 Fixture build:     {build:.1} ms   fixture export: {fexp:.1} ms\n\
         \x20 Fixture size:      {fsize:.1} KiB\n\
         \x20 Load (parse):      {load:.1} ms\n\
         \x20 Mesh extraction:   {mesh:.1} ms\n\
         \x20 Mesh JSON:         {mjson:.1} ms -> {mbytes:.1} MiB\n\
         \x20 region_view:       {view:.1} ms\n\
         \x20 region_faces:      {faces:.1} ms -> {fbytes:.1} MiB, {fcount} faces in {fchunks} chunks \
         ({fpb:.2} faces/block, peak RSS {rfaces})\n\
         \x20 Inspect / Validate: {insp:.1} / {val:.1} ms\n\
         \x20 Analyze:           {ana:.1} ms (surface + feature pass alone: {surf:.1} ms)\n\
         \x20 Replace+commit:    {edit:.1} ms ({changed} blocks changed)\n\
         \x20 Undo / Redo:       {undo:.1} ms / {redo:.1} ms\n\
         \x20 Re-export:         {exp:.1} ms -> {esize:.1} KiB\n\
         \x20 Peak RSS during load / mesh+JSON / analyze: {rload} / {rmesh} / {rana}",
        name = s.name,
        w = s.width,
        h = s.height,
        l = s.length,
        vox = fixture.total_voxels,
        occ = fixture.occupied_blocks,
        fill = fixture.fill_percent(),
        pal = fixture.palette_states,
        be = fixture.block_entities,
        build = fixture.build_ms,
        fexp = fixture.export_ms,
        fsize = r.fixture_bytes as f64 / 1024.0,
        load = r.load_ms,
        mesh = r.mesh_ms,
        mjson = r.mesh_json_ms,
        mbytes = r.mesh_json_bytes as f64 / (1024.0 * 1024.0),
        view = r.view_ms,
        faces = r.faces_ms,
        fbytes = r.faces_bytes as f64 / (1024.0 * 1024.0),
        fcount = r.face_count,
        fchunks = r.face_chunks,
        fpb = r.face_count as f64 / fixture.occupied_blocks as f64,
        rfaces = fmt_mb(r.faces_peak_mb),
        insp = r.inspect_ms,
        val = r.validate_ms,
        ana = r.analyze_ms,
        edit = r.edit_ms,
        changed = r.edit_changed,
        undo = r.undo_ms,
        redo = r.redo_ms,
        exp = r.export_ms,
        esize = r.export_bytes as f64 / 1024.0,
        surf = r.surface_ms,
        rload = fmt_mb(r.load_peak_mb),
        rmesh = fmt_mb(r.mesh_peak_mb),
        rana = fmt_mb(r.analyze_peak_mb),
    );
}

fn check_budget(name: &str, r: &ScaleReport, b: &Budget) {
    if cfg!(debug_assertions) {
        println!("  (debug build: regression budgets not enforced; use --release)");
        return;
    }
    let checks = [
        ("load", r.load_ms, b.load_ms),
        ("mesh", r.mesh_ms, b.mesh_ms),
        ("mesh JSON", r.mesh_json_ms, b.mesh_json_ms),
        ("region_faces", r.faces_ms, b.faces_ms),
        ("analyze", r.analyze_ms, b.analyze_ms),
        ("replace+commit", r.edit_ms, b.edit_ms),
        ("undo", r.undo_ms, b.undo_ms),
        ("export", r.export_ms, b.export_ms),
    ];
    for (stage, measured, ceiling) in checks {
        assert!(
            measured < ceiling,
            "{name} {stage} regressed: {measured:.1} ms >= budget {ceiling:.1} ms"
        );
    }
}

fn run_scale(scale: Scale, budget: &Budget) {
    let fixture = dense_fixture::generate(scale);
    assert_eq!(fixture.total_voxels, scale.total_voxels());
    assert!(
        fixture.fill_percent() >= 30.0,
        "fixture must be dense, got {:.1}%",
        fixture.fill_percent()
    );
    let report = run_pipeline(&fixture);
    print_report(&fixture, &report);
    check_budget(scale.name, &report, budget);
}

/// Default-suite smoke test: exercises the full dense pipeline on a 32^3
/// volume with no timing assertions.
#[test]
fn smoke_dense_32_cubed() {
    let fixture = dense_fixture::generate(SMOKE_32);
    assert_eq!(fixture.total_voxels, 32 * 32 * 32);
    assert!(fixture.fill_percent() > 50.0 && fixture.fill_percent() < 60.0);
    assert!(fixture.palette_states >= 30);
    assert!(fixture.block_entities > 0);
    let report = run_pipeline(&fixture);
    assert!(report.mesh_json_bytes > 0);
    assert!(report.export_bytes > 0);
}

// Budgets are roughly 5-10x the release numbers measured on a 4-vCPU Linux
// container (see docs/benchmarks.md); they catch order-of-magnitude
// regressions, not noise. The analysis budget is wide because the current
// surface analysis is known to be slow on dense volumes (see the doc).

#[test]
#[ignore = "scale benchmark: run with --release -- --ignored --nocapture --test-threads=1"]
fn scale_01m_dense() {
    run_scale(
        SCALE_1M,
        &Budget {
            load_ms: 1_000.0,
            mesh_ms: 1_500.0,
            mesh_json_ms: 500.0,
            faces_ms: 1_500.0,
            analyze_ms: 30_000.0,
            edit_ms: 500.0,
            undo_ms: 250.0,
            export_ms: 1_500.0,
        },
    );
}

#[test]
#[ignore = "scale benchmark: run with --release -- --ignored --nocapture --test-threads=1"]
fn scale_05m_dense() {
    run_scale(
        SCALE_5M,
        &Budget {
            load_ms: 3_000.0,
            mesh_ms: 9_000.0,
            mesh_json_ms: 1_500.0,
            faces_ms: 6_000.0,
            analyze_ms: 150_000.0,
            edit_ms: 1_000.0,
            undo_ms: 250.0,
            export_ms: 6_000.0,
        },
    );
}

#[test]
#[ignore = "scale benchmark: run with --release -- --ignored --nocapture --test-threads=1"]
fn scale_10m_dense() {
    run_scale(
        SCALE_10M,
        &Budget {
            load_ms: 6_000.0,
            mesh_ms: 20_000.0,
            mesh_json_ms: 3_000.0,
            faces_ms: 15_000.0,
            analyze_ms: 300_000.0,
            edit_ms: 2_000.0,
            undo_ms: 250.0,
            export_ms: 15_000.0,
        },
    );
}
