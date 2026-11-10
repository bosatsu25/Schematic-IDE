//! Corrupt or hostile files must produce an error (or a best-effort
//! document), never a panic, a hang or a giant allocation: in the browser a
//! panic kills the WASM engine.

use schematic_core::{
    BlockEntityRef, BlockPosition, BlockState, Document, DocumentMetadata, NbtTag, Position,
    Region, RegionId, Size,
};
use schematic_format::{
    detect_format, encode_gzip, write_nbt, LitematicDocument, SpongeSchematic, StructureDocument,
};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

fn compound(pairs: Vec<(&str, NbtTag)>) -> NbtTag {
    NbtTag::Compound(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn xyz(x: i32, y: i32, z: i32) -> NbtTag {
    compound(vec![
        ("x", NbtTag::Int(x)),
        ("y", NbtTag::Int(y)),
        ("z", NbtTag::Int(z)),
    ])
}

fn raw(tag: &NbtTag) -> Vec<u8> {
    let mut out = Vec::new();
    write_nbt(&mut out, "", tag).unwrap();
    out
}

/// Runs every parser on `bytes`; none may panic or take long.
fn parse_all(label: &str, bytes: &[u8]) {
    let start = Instant::now();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = detect_format(bytes);
        let _ = LitematicDocument::parse(bytes);
        let _ = SpongeSchematic::parse(bytes);
        let _ = StructureDocument::parse(bytes);
    }));
    assert!(result.is_ok(), "parser panicked on {label}");
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "parser took {:?} on {label}",
        start.elapsed()
    );
}

fn sample_document() -> Document {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Robust".into()),
        ..Default::default()
    });
    let id = RegionId::new("Main");
    let mut region = Region::new(id.clone(), Position::new(0, 0, 0), Size::new(5, 4, 3));
    for (i, name) in ["minecraft:stone", "minecraft:chest", "minecraft:oak_stairs"]
        .iter()
        .enumerate()
    {
        let idx = region
            .palette_mut()
            .intern(BlockState::new(*name, []).unwrap())
            .unwrap();
        for x in 0..5 {
            region
                .set_block_index(BlockPosition::new(x, i as i64, x % 3), Some(idx))
                .unwrap();
        }
    }
    doc.insert_region(region);
    doc.add_block_entity(BlockEntityRef::new(
        id,
        "minecraft:chest",
        BlockPosition::new(0, 1, 0),
    ));
    doc
}

/// Uncompressed NBT of the sample in each format (mutating compressed bytes
/// would only ever exercise the gzip checksum).
fn sample_files() -> Vec<(&'static str, Vec<u8>)> {
    let doc = sample_document();
    let gz = [
        (
            "litematic",
            LitematicDocument::from_document(doc.clone(), 3465)
                .export(&doc)
                .unwrap(),
        ),
        (
            "sponge",
            SpongeSchematic::from_document(doc.clone(), 3465)
                .export(&doc)
                .unwrap(),
        ),
        (
            "structure",
            StructureDocument::from_document(doc.clone(), 3465)
                .export(&doc)
                .unwrap(),
        ),
    ];
    gz.into_iter()
        .map(|(name, bytes)| {
            let (_, tag) = schematic_format::decode_gzip_or_raw(&bytes).unwrap();
            (name, raw(&tag))
        })
        .collect()
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

#[test]
fn mutated_files_never_panic() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for (name, bytes) in sample_files() {
        // Every truncation point.
        for cut in 0..bytes.len() {
            parse_all(&format!("{name} truncated at {cut}"), &bytes[..cut]);
        }
        // Random byte corruption, including the interesting values.
        for round in 0..3000 {
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
            parse_all(&format!("{name} mutation round {round}"), &mutated);
        }
    }
}

#[test]
fn hostile_dimensions_and_lengths_are_rejected_or_bounded() {
    let palette = NbtTag::List(
        10,
        vec![
            compound(vec![("Name", NbtTag::String("minecraft:air".into()))]),
            compound(vec![("Name", NbtTag::String("minecraft:stone".into()))]),
        ],
    );
    let litematic = |pos: NbtTag, size: NbtTag| {
        compound(vec![
            ("MinecraftDataVersion", NbtTag::Int(3465)),
            ("Version", NbtTag::Int(6)),
            ("Metadata", compound(vec![])),
            (
                "Regions",
                compound(vec![(
                    "R",
                    compound(vec![
                        ("Position", pos),
                        ("Size", size),
                        ("BlockStatePalette", palette.clone()),
                        ("BlockStates", NbtTag::LongArray(vec![-1, -1])),
                    ]),
                )]),
            ),
        ])
    };
    for (label, pos, size) in [
        ("huge", xyz(0, 0, 0), xyz(i32::MAX, i32::MAX, i32::MAX)),
        ("min", xyz(0, 0, 0), xyz(i32::MIN, i32::MIN, i32::MIN)),
        (
            "overflowing corner",
            xyz(i32::MIN, 0, i32::MAX),
            xyz(-5, 4, 9),
        ),
        ("zero", xyz(0, 0, 0), xyz(0, 0, 0)),
    ] {
        parse_all(
            &format!("litematic {label} size"),
            &raw(&litematic(pos, size)),
        );
    }
    // A plausible but lying size is an error, not a region full of air.
    let lying = LitematicDocument::parse(&raw(&litematic(xyz(0, 0, 0), xyz(1000, 1000, 1000))));
    assert!(
        lying.is_err(),
        "BlockStates too short for the declared size"
    );
    // The honest size for 2 longs at 2 bits (64 blocks) parses.
    let honest = LitematicDocument::parse(&raw(&litematic(xyz(0, 0, 0), xyz(4, 4, -4)))).unwrap();
    let blocks: usize = honest
        .document
        .regions()
        .flat_map(|r| r.chunks().map(|(_, c)| c.occupied_blocks().count()))
        .sum();
    assert_eq!(blocks, 0, "index 3 is outside the 2-entry palette");

    let sponge = |w: i16, h: i16, l: i16, pal_id: i32| {
        compound(vec![
            ("Version", NbtTag::Int(2)),
            ("DataVersion", NbtTag::Int(3465)),
            ("Width", NbtTag::Short(w)),
            ("Height", NbtTag::Short(h)),
            ("Length", NbtTag::Short(l)),
            (
                "Palette",
                compound(vec![
                    ("minecraft:air", NbtTag::Int(0)),
                    ("minecraft:stone", NbtTag::Int(pal_id)),
                ]),
            ),
            (
                "BlockData",
                NbtTag::ByteArray(vec![1, 1, 1, 0xFF, 0xFF, 0xFF]),
            ),
        ])
    };
    for (label, tag) in [
        ("zero width", sponge(0, 4, 4, 1)),
        ("zero everything", sponge(0, 0, 0, 1)),
        ("max dims", sponge(-1, -1, -1, 1)),
        ("huge palette id", sponge(2, 2, 2, i32::MAX)),
        ("negative palette id", sponge(2, 2, 2, i32::MIN)),
    ] {
        parse_all(&format!("sponge {label}"), &raw(&tag));
    }

    let structure = compound(vec![
        ("DataVersion", NbtTag::Int(3465)),
        ("size", NbtTag::List(3, vec![NbtTag::Int(i32::MAX); 3])),
        ("palette", palette.clone()),
        (
            "blocks",
            NbtTag::List(
                10,
                vec![compound(vec![
                    (
                        "pos",
                        NbtTag::List(
                            3,
                            vec![NbtTag::Int(i32::MAX), NbtTag::Int(-7), NbtTag::Int(0)],
                        ),
                    ),
                    ("state", NbtTag::Int(i32::MIN)),
                ])],
            ),
        ),
    ]);
    parse_all("structure huge size", &raw(&structure));
}

#[test]
fn hostile_nbt_encodings_are_errors() {
    // 200k nested lists: must hit the depth limit, not the stack limit.
    let mut deep = vec![9u8, 0, 0];
    for _ in 0..200_000 {
        deep.extend_from_slice(&[9, 0, 0, 0, 1]);
    }
    parse_all("deep nesting", &deep);
    assert!(schematic_format::decode_gzip_or_raw(&deep).is_err());

    // Lengths that promise gigabytes on a short stream.
    for (label, tail) in [
        (
            "byte array",
            vec![7u8, 0, 0, 0x7F, 0xFF, 0xFF, 0xFF, 1, 2, 3],
        ),
        (
            "int array",
            vec![11u8, 0, 0, 0x7F, 0xFF, 0xFF, 0xFF, 1, 2, 3],
        ),
        (
            "long array",
            vec![12u8, 0, 0, 0x7F, 0xFF, 0xFF, 0xFF, 1, 2, 3],
        ),
        ("list of end", vec![9u8, 0, 0, 0, 0x7F, 0xFF, 0xFF, 0xFF]),
        (
            "list of compounds",
            vec![9u8, 0, 0, 10, 0x7F, 0xFF, 0xFF, 0xFF],
        ),
    ] {
        parse_all(label, &tail);
        assert!(
            schematic_format::decode_gzip_or_raw(&tail).is_err(),
            "{label}"
        );
    }

    // A gzip bomb: 64 MiB of zeros compresses to a few KiB and is refused
    // by a 1 MiB limit without being fully inflated.
    let mut bomb_tag = BTreeMap::new();
    bomb_tag.insert("b".to_string(), NbtTag::ByteArray(vec![0u8; 64 << 20]));
    let bomb = encode_gzip("", &NbtTag::Compound(bomb_tag)).unwrap();
    assert!(bomb.len() < 1 << 20);
    assert!(schematic_format::nbt::decode_with_limit(&bomb, 1 << 20).is_err());
    assert!(schematic_format::nbt::decode_with_limit(&bomb, 128 << 20).is_ok());
}
