//! Writes a dense benchmark `.litematic` fixture to disk.
//!
//! ```text
//! cargo run --release -p schematic-wasm --example gen_dense_fixture -- <32|1m|5m|10m> <out.litematic>
//! ```
//!
//! Prints one JSON line describing the fixture (used by
//! `apps/web/scripts/bench-browser.mjs`). The generator is shared with
//! `tests/scale_benchmarks.rs`.

#[path = "../tests/support/dense_fixture.rs"]
mod dense_fixture;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(scale_name), Some(out_path)) = (args.first(), args.get(1)) else {
        eprintln!("usage: gen_dense_fixture <32|1m|5m|10m> <out.litematic>");
        return ExitCode::from(2);
    };
    let Some(scale) = dense_fixture::scale_by_name(scale_name) else {
        eprintln!("unknown scale '{scale_name}' (expected 32, 1m, 5m or 10m)");
        return ExitCode::from(2);
    };

    let fixture = dense_fixture::generate(scale);
    if let Err(e) = std::fs::write(out_path, &fixture.litematic) {
        eprintln!("failed to write {out_path}: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "{{\"scale\":\"{}\",\"dims\":[{},{},{}],\"total_voxels\":{},\"occupied_blocks\":{},\
         \"fill_percent\":{:.2},\"palette_states\":{},\"block_entities\":{},\
         \"file_bytes\":{},\"region\":\"{}\",\"path\":{:?}}}",
        scale.name,
        scale.width,
        scale.height,
        scale.length,
        fixture.total_voxels,
        fixture.occupied_blocks,
        fixture.fill_percent(),
        fixture.palette_states,
        fixture.block_entities,
        fixture.litematic.len(),
        dense_fixture::REGION_NAME,
        out_path,
    );
    ExitCode::SUCCESS
}
