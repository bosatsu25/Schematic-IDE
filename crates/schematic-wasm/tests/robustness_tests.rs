//! A corrupt file that still parses must not crash any later stage either:
//! in the browser a panic kills the WASM engine and loses the session.

use schematic_core::{
    BlockEntityRef, BlockPosition, BlockState, Document, DocumentMetadata, EntityRef, NbtTag,
    Position, Region, RegionId, Size,
};
use schematic_format::{
    decode_gzip_or_raw, write_nbt, LitematicDocument, SpongeSchematic, StructureDocument,
};
use schematic_wasm::Session;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn sample_files() -> Vec<(&'static str, Vec<u8>)> {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Robust".into()),
        ..Default::default()
    });
    for (name, origin) in [
        ("A", Position::new(0, 0, 0)),
        ("B", Position::new(9, 0, -3)),
    ] {
        let id = RegionId::new(name);
        let mut region = Region::new(id.clone(), origin, Size::new(5, 4, 3));
        for (i, block) in ["minecraft:stone", "minecraft:chest", "minecraft:oak_stairs"]
            .iter()
            .enumerate()
        {
            let idx = region
                .palette_mut()
                .intern(BlockState::new(*block, []).unwrap())
                .unwrap();
            for x in 0..5 {
                region
                    .set_block_index(BlockPosition::new(x, i as i64, x % 3), Some(idx))
                    .unwrap();
            }
        }
        doc.insert_region(region);
        doc.add_block_entity(BlockEntityRef::new(
            id.clone(),
            "minecraft:chest",
            BlockPosition::new(0, 1, 0),
        ));
        doc.add_entity(EntityRef::new(id, "minecraft:pig", [1.5, 3.0, 1.5]));
    }
    let raw = |bytes: Vec<u8>| {
        let (_, tag): (String, NbtTag) = decode_gzip_or_raw(&bytes).unwrap();
        let mut out = Vec::new();
        write_nbt(&mut out, "", &tag).unwrap();
        out
    };
    vec![
        (
            "litematic",
            raw(LitematicDocument::from_document(doc.clone(), 3465)
                .export(&doc)
                .unwrap()),
        ),
        (
            "sponge",
            raw(SpongeSchematic::from_document(doc.clone(), 3465)
                .export(&doc)
                .unwrap()),
        ),
        (
            "structure",
            raw(StructureDocument::from_document(doc.clone(), 3465)
                .export(&doc)
                .unwrap()),
        ),
    ]
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Exercises everything the app does after opening a file.
fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let out = f();
    let ms = start.elapsed().as_millis();
    if ms > 200 && std::env::var("ROBUSTNESS_TIMING").is_ok() {
        eprintln!("slow stage {label}: {ms} ms");
    }
    out
}

fn drive(bytes: &[u8]) {
    let mut session = Session::new();
    let Ok(summary) = timed("load", || session.load_schematic(bytes)) else {
        return;
    };
    let _ = timed("inspect_document()", || session.inspect_document());
    let _ = timed("validate_document()", || session.validate_document());
    let _ = timed("list_entities()", || session.list_entities());
    let _ = timed("diff_with_source()", || session.diff_with_source());
    let mut volume = 0u64;
    for region in &summary.regions {
        volume += region.size.iter().map(|&s| s as u64).product::<u64>();
        let _ = timed("region_view", || session.region_view(&region.name));
        let _ = timed("region_faces", || session.region_faces(&region.name, None));
        let _ = timed("get_region_mesh", || session.get_region_mesh(&region.name));
        let _ = timed("analyze", || session.analyze(Some(&region.name)));
    }
    let _ = timed("preview_fix(None)", || session.preview_fix(None));
    let _ = session.cancel_preview();
    for target in ["litematic", "sponge", "structure"] {
        let _ = timed("export_report", || session.export_report(target));
    }
    // Dense exports of a huge (but sparse and valid) volume are slow, not
    // unsafe; keep the test fast.
    if volume <= 1 << 16 {
        let _ = timed("export_litematic()", || session.export_litematic());
        let _ = timed("export_sponge()", || session.export_sponge());
        let _ = timed("export_structure()", || session.export_structure());
    }
}

#[test]
fn corrupt_files_never_crash_the_session() {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for (name, bytes) in sample_files() {
        for round in 0..120 {
            let mut mutated = bytes.clone();
            for _ in 0..1 + rng.next() % 3 {
                let at = (rng.next() as usize) % mutated.len();
                mutated[at] = match rng.next() % 5 {
                    0 => 0x00,
                    1 => 0xFF,
                    2 => 0x7F,
                    3 => 0x80,
                    _ => rng.next() as u8,
                };
            }
            let result = catch_unwind(AssertUnwindSafe(|| drive(&mutated)));
            assert!(
                result.is_ok(),
                "session panicked on {name} mutation round {round}"
            );
        }
    }
}
