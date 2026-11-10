use schematic_core::{
    BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata, Position, Region,
    RegionId, Size,
};
use schematic_format::{LitematicDocument, NbtTag};
use std::collections::BTreeMap;

fn make_block_state(id: &str, props: &[(&str, &str)]) -> BlockState {
    let p: Vec<BlockProperty> = props
        .iter()
        .map(|&(k, v)| BlockProperty::new(k, v))
        .collect();
    BlockState::new(id, p).unwrap()
}

#[test]
fn test_roundtrip_empty_litematic() {
    let mut root = BTreeMap::new();
    root.insert("MinecraftDataVersion".to_string(), NbtTag::Int(2975));
    root.insert("Version".to_string(), NbtTag::Int(5));
    root.insert("Metadata".to_string(), NbtTag::Compound(BTreeMap::new()));
    root.insert("Regions".to_string(), NbtTag::Compound(BTreeMap::new()));

    let bytes = schematic_format::encode_gzip("Litematic", &NbtTag::Compound(root)).unwrap();
    let litematic = LitematicDocument::parse(&bytes).unwrap();
    assert_eq!(litematic.document.regions().count(), 0);

    let exported = litematic.export(&litematic.document).unwrap();
    let reparsed = LitematicDocument::parse(&exported).unwrap();
    assert_eq!(reparsed.document.regions().count(), 0);
    assert_eq!(reparsed.data_version, 2975);
}

#[test]
fn test_roundtrip_single_region_with_blocks() {
    let rid = RegionId::new("MainRegion");
    let origin = Position::new(10, 20, 30);
    let size = Size::new(3, 3, 3);
    let mut region = Region::new(rid.clone(), origin, size);

    let stone = make_block_state("minecraft:stone", &[]);
    let stairs = make_block_state(
        "minecraft:oak_stairs",
        &[("facing", "north"), ("half", "bottom")],
    );

    let idx_stone = region.palette_mut().intern(stone.clone()).unwrap();
    let idx_stairs = region.palette_mut().intern(stairs.clone()).unwrap();

    region
        .set_block_index(BlockPosition::new(0, 0, 0), Some(idx_stone))
        .unwrap();
    region
        .set_block_index(BlockPosition::new(1, 2, 1), Some(idx_stairs))
        .unwrap();

    let mut doc = Document::new(DocumentMetadata {
        name: Some("TestSingle".to_string()),
        author: Some("Architect".to_string()),
        description: Some("Single region test".to_string()),
        created_at_unix_ms: Some(1700000000000),
        modified_at_unix_ms: Some(1700000005000),
    });
    doc.insert_region(region);

    let default_litematic = LitematicDocument {
        data_version: 3120,
        version: 6,
        document: doc.clone(),
        raw_metadata: BTreeMap::new(),
        raw_root_unknown: BTreeMap::new(),
        raw_regions: BTreeMap::new(),
    };

    let exported = default_litematic.export(&doc).unwrap();
    let parsed = LitematicDocument::parse(&exported).unwrap();

    assert_eq!(parsed.data_version, 3120);
    assert_eq!(parsed.version, 6);
    assert_eq!(
        parsed.document.metadata().name.as_deref(),
        Some("TestSingle")
    );
    assert_eq!(
        parsed.document.metadata().author.as_deref(),
        Some("Architect")
    );

    let loaded_reg = parsed.document.region(&rid).expect("Region must exist");
    assert_eq!(loaded_reg.origin(), origin);
    assert_eq!(loaded_reg.size(), size);

    let b0 = loaded_reg
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    assert_eq!(loaded_reg.palette().get(b0.unwrap()), Some(&stone));

    let b1 = loaded_reg
        .block_index_at(BlockPosition::new(1, 2, 1))
        .unwrap();
    assert_eq!(loaded_reg.palette().get(b1.unwrap()), Some(&stairs));

    // Unset block is air
    let b_empty = loaded_reg
        .block_index_at(BlockPosition::new(2, 2, 2))
        .unwrap();
    assert_eq!(b_empty, None);
}

#[test]
fn test_roundtrip_multi_region() {
    let mut doc = Document::new(DocumentMetadata::default());

    let r1_id = RegionId::new("RegionA");
    let mut r1 = Region::new(r1_id.clone(), Position::new(0, 0, 0), Size::new(2, 2, 2));
    let stone = make_block_state("minecraft:stone", &[]);
    let s_idx = r1.palette_mut().intern(stone.clone()).unwrap();
    r1.set_block_index(BlockPosition::new(0, 0, 0), Some(s_idx))
        .unwrap();
    doc.insert_region(r1);

    let r2_id = RegionId::new("RegionB");
    let mut r2 = Region::new(
        r2_id.clone(),
        Position::new(100, 50, 100),
        Size::new(4, 4, 4),
    );
    let glass = make_block_state("minecraft:glass", &[]);
    let g_idx = r2.palette_mut().intern(glass.clone()).unwrap();
    r2.set_block_index(BlockPosition::new(3, 3, 3), Some(g_idx))
        .unwrap();
    doc.insert_region(r2);

    let litematic = LitematicDocument {
        data_version: 2975,
        version: 5,
        document: doc.clone(),
        raw_metadata: BTreeMap::new(),
        raw_root_unknown: BTreeMap::new(),
        raw_regions: BTreeMap::new(),
    };

    let exported = litematic.export(&doc).unwrap();
    let parsed = LitematicDocument::parse(&exported).unwrap();

    assert_eq!(parsed.document.regions().count(), 2);
    let loaded_r1 = parsed.document.region(&r1_id).unwrap();
    assert_eq!(loaded_r1.origin(), Position::new(0, 0, 0));
    assert_eq!(
        loaded_r1.palette().get(
            loaded_r1
                .block_index_at(BlockPosition::new(0, 0, 0))
                .unwrap()
                .unwrap()
        ),
        Some(&stone)
    );

    let loaded_r2 = parsed.document.region(&r2_id).unwrap();
    assert_eq!(loaded_r2.origin(), Position::new(100, 50, 100));
    assert_eq!(
        loaded_r2.palette().get(
            loaded_r2
                .block_index_at(BlockPosition::new(3, 3, 3))
                .unwrap()
                .unwrap()
        ),
        Some(&glass)
    );
}

#[test]
fn test_negative_size_encoding_normalization_and_roundtrip() {
    // Construct a raw Litematic NBT with negative sizes: Pos = (10, 20, 30), Size = (-4, -5, -6)
    let mut root = BTreeMap::new();
    root.insert("MinecraftDataVersion".to_string(), NbtTag::Int(2975));
    root.insert("Version".to_string(), NbtTag::Int(5));
    root.insert("Metadata".to_string(), NbtTag::Compound(BTreeMap::new()));

    let mut pos_map = BTreeMap::new();
    pos_map.insert("x".to_string(), NbtTag::Int(10));
    pos_map.insert("y".to_string(), NbtTag::Int(20));
    pos_map.insert("z".to_string(), NbtTag::Int(30));

    let mut size_map = BTreeMap::new();
    size_map.insert("x".to_string(), NbtTag::Int(-4));
    size_map.insert("y".to_string(), NbtTag::Int(-5));
    size_map.insert("z".to_string(), NbtTag::Int(-6));

    let mut palette_list = Vec::new();
    let mut stone_entry = BTreeMap::new();
    stone_entry.insert(
        "Name".to_string(),
        NbtTag::String("minecraft:stone".to_string()),
    );
    palette_list.push(NbtTag::Compound(stone_entry));

    // 4 * 5 * 6 = 120 blocks. Palette len 1 -> bits = 2.
    let total_blocks = 120;
    let packed = schematic_format::bit_packing::pack_block_states(&vec![0; total_blocks], 2);

    let mut reg_map = BTreeMap::new();
    reg_map.insert("Position".to_string(), NbtTag::Compound(pos_map));
    reg_map.insert("Size".to_string(), NbtTag::Compound(size_map));
    reg_map.insert(
        "BlockStatePalette".to_string(),
        NbtTag::List(10, palette_list),
    );
    reg_map.insert("BlockStates".to_string(), NbtTag::LongArray(packed));

    let mut regions = BTreeMap::new();
    regions.insert("NegRegion".to_string(), NbtTag::Compound(reg_map));
    root.insert("Regions".to_string(), NbtTag::Compound(regions));

    let bytes = schematic_format::encode_gzip("Litematic", &NbtTag::Compound(root)).unwrap();
    let parsed = LitematicDocument::parse(&bytes).unwrap();

    let reg = parsed.document.region(&RegionId::new("NegRegion")).unwrap();
    // Normalization check:
    // min_x = 10 + (-4) + 1 = 7
    // min_y = 20 + (-5) + 1 = 16
    // min_z = 30 + (-6) + 1 = 25
    assert_eq!(reg.origin(), Position::new(7, 16, 25));
    assert_eq!(reg.size(), Size::new(4, 5, 6));

    // Re-export should preserve the original negative-size encoding if unchanged
    let exported = parsed.export(&parsed.document).unwrap();
    let reparsed = LitematicDocument::parse(&exported).unwrap();
    let raw = reparsed
        .raw_regions
        .get(&RegionId::new("NegRegion"))
        .unwrap();
    assert_eq!(raw.original_pos, [10, 20, 30]);
    assert_eq!(raw.original_size, [-4, -5, -6]);
}

#[test]
fn test_block_entities_and_entities_preservation() {
    let mut root = BTreeMap::new();
    root.insert("MinecraftDataVersion".to_string(), NbtTag::Int(2975));
    root.insert("Version".to_string(), NbtTag::Int(5));
    root.insert("Metadata".to_string(), NbtTag::Compound(BTreeMap::new()));

    let mut reg_map = BTreeMap::new();
    let mut pos_map = BTreeMap::new();
    pos_map.insert("x".to_string(), NbtTag::Int(0));
    pos_map.insert("y".to_string(), NbtTag::Int(0));
    pos_map.insert("z".to_string(), NbtTag::Int(0));
    reg_map.insert("Position".to_string(), NbtTag::Compound(pos_map));

    let mut size_map = BTreeMap::new();
    size_map.insert("x".to_string(), NbtTag::Int(2));
    size_map.insert("y".to_string(), NbtTag::Int(2));
    size_map.insert("z".to_string(), NbtTag::Int(2));
    reg_map.insert("Size".to_string(), NbtTag::Compound(size_map));

    let mut palette_list = Vec::new();
    let mut air_entry = BTreeMap::new();
    air_entry.insert(
        "Name".to_string(),
        NbtTag::String("minecraft:air".to_string()),
    );
    palette_list.push(NbtTag::Compound(air_entry));
    reg_map.insert(
        "BlockStatePalette".to_string(),
        NbtTag::List(10, palette_list),
    );
    reg_map.insert("BlockStates".to_string(), NbtTag::LongArray(vec![0; 8]));

    // Block Entity (Chest with Items)
    let mut chest = BTreeMap::new();
    chest.insert(
        "id".to_string(),
        NbtTag::String("minecraft:chest".to_string()),
    );
    chest.insert("x".to_string(), NbtTag::Int(1));
    chest.insert("y".to_string(), NbtTag::Int(1));
    chest.insert("z".to_string(), NbtTag::Int(1));
    chest.insert(
        "CustomName".to_string(),
        NbtTag::String("{\"text\":\"Treasure\"}".to_string()),
    );
    let be_list = vec![NbtTag::Compound(chest)];
    reg_map.insert("TileEntities".to_string(), NbtTag::List(10, be_list));

    // Entity (Item Frame)
    let mut frame = BTreeMap::new();
    frame.insert(
        "id".to_string(),
        NbtTag::String("minecraft:item_frame".to_string()),
    );
    frame.insert(
        "Pos".to_string(),
        NbtTag::List(
            6,
            vec![
                NbtTag::Double(1.5),
                NbtTag::Double(2.0),
                NbtTag::Double(1.5),
            ],
        ),
    );
    let ent_list = vec![NbtTag::Compound(frame)];
    reg_map.insert("Entities".to_string(), NbtTag::List(10, ent_list));

    let mut regions = BTreeMap::new();
    regions.insert("EntityRegion".to_string(), NbtTag::Compound(reg_map));
    root.insert("Regions".to_string(), NbtTag::Compound(regions));

    let bytes = schematic_format::encode_gzip("Litematic", &NbtTag::Compound(root)).unwrap();
    let parsed = LitematicDocument::parse(&bytes).unwrap();

    let bes: Vec<_> = parsed.document.block_entities().collect();
    assert_eq!(bes.len(), 1);
    assert_eq!(bes[0].kind, "minecraft:chest");
    assert_eq!(bes[0].position, BlockPosition::new(1, 1, 1));

    let ents: Vec<_> = parsed.document.entities().collect();
    assert_eq!(ents.len(), 1);
    assert_eq!(ents[0].kind, "minecraft:item_frame");
    assert_eq!(ents[0].position, [1.5, 2.0, 1.5]);

    // Re-export and verify full NBT payload preserved (including CustomName)
    let exported = parsed.export(&parsed.document).unwrap();
    let reparsed = LitematicDocument::parse(&exported).unwrap();
    let bes: Vec<_> = reparsed.document.block_entities().collect();
    assert_eq!(bes.len(), 1);
    assert_eq!(
        bes[0].data.get("CustomName").unwrap().as_string(),
        Some("{\"text\":\"Treasure\"}")
    );
    let ents: Vec<_> = reparsed.document.entities().collect();
    assert_eq!(ents.len(), 1);
    assert_eq!(ents[0].position, [1.5, 2.0, 1.5]);
}

#[test]
fn test_unknown_nbt_fields_preservation() {
    let mut root = BTreeMap::new();
    root.insert("MinecraftDataVersion".to_string(), NbtTag::Int(2975));
    root.insert("Version".to_string(), NbtTag::Int(5));
    root.insert(
        "CustomTopLevelTag".to_string(),
        NbtTag::String("PreserveMe".to_string()),
    );

    let mut meta = BTreeMap::new();
    meta.insert(
        "Name".to_string(),
        NbtTag::String("UnknownTest".to_string()),
    );
    meta.insert("CustomMetaTag".to_string(), NbtTag::Int(42));
    root.insert("Metadata".to_string(), NbtTag::Compound(meta));

    let mut reg = BTreeMap::new();
    let mut pos = BTreeMap::new();
    pos.insert("x".to_string(), NbtTag::Int(0));
    pos.insert("y".to_string(), NbtTag::Int(0));
    pos.insert("z".to_string(), NbtTag::Int(0));
    reg.insert("Position".to_string(), NbtTag::Compound(pos));

    let mut sz = BTreeMap::new();
    sz.insert("x".to_string(), NbtTag::Int(1));
    sz.insert("y".to_string(), NbtTag::Int(1));
    sz.insert("z".to_string(), NbtTag::Int(1));
    reg.insert("Size".to_string(), NbtTag::Compound(sz));

    let mut pal = Vec::new();
    let mut air = BTreeMap::new();
    air.insert(
        "Name".to_string(),
        NbtTag::String("minecraft:air".to_string()),
    );
    pal.push(NbtTag::Compound(air));
    reg.insert("BlockStatePalette".to_string(), NbtTag::List(10, pal));
    reg.insert("BlockStates".to_string(), NbtTag::LongArray(vec![0]));
    reg.insert(
        "CustomRegionTag".to_string(),
        NbtTag::ByteArray(vec![1, 2, 3, 4]),
    );

    let mut regions = BTreeMap::new();
    regions.insert("Reg1".to_string(), NbtTag::Compound(reg));
    root.insert("Regions".to_string(), NbtTag::Compound(regions));

    let bytes = schematic_format::encode_gzip("Litematic", &NbtTag::Compound(root)).unwrap();
    let parsed = LitematicDocument::parse(&bytes).unwrap();

    let exported = parsed.export(&parsed.document).unwrap();
    let reparsed = LitematicDocument::parse(&exported).unwrap();

    assert_eq!(
        reparsed
            .raw_root_unknown
            .get("CustomTopLevelTag")
            .unwrap()
            .as_string(),
        Some("PreserveMe")
    );
    assert_eq!(
        reparsed.raw_metadata.get("CustomMetaTag").unwrap().as_i32(),
        Some(42)
    );
    let reg_raw = reparsed.raw_regions.get(&RegionId::new("Reg1")).unwrap();
    assert_eq!(
        reg_raw.unknown_tags.get("CustomRegionTag"),
        Some(&NbtTag::ByteArray(vec![1, 2, 3, 4]))
    );
}

#[test]
fn test_modern_minecraft_version() {
    let mut root = BTreeMap::new();
    // 3953 = Minecraft 1.20.5 / 1.21
    root.insert("MinecraftDataVersion".to_string(), NbtTag::Int(3953));
    root.insert("Version".to_string(), NbtTag::Int(6));
    root.insert("Metadata".to_string(), NbtTag::Compound(BTreeMap::new()));
    root.insert("Regions".to_string(), NbtTag::Compound(BTreeMap::new()));

    let bytes = schematic_format::encode_gzip("Litematic", &NbtTag::Compound(root)).unwrap();
    let parsed = LitematicDocument::parse(&bytes).unwrap();
    assert_eq!(parsed.data_version, 3953);
    assert_eq!(parsed.version, 6);

    let exported = parsed.export(&parsed.document).unwrap();
    let reparsed = LitematicDocument::parse(&exported).unwrap();
    assert_eq!(reparsed.data_version, 3953);
}

/// Issue #37: with a negative `Size`, the block container and TileEntities
/// are indexed from the region's minimum corner (never mirrored), and entity
/// `Pos` is relative to `Position` (the primary corner).
#[test]
fn negative_size_regions_are_not_mirrored() {
    use schematic_core::BlockPosition;

    let compound = |pairs: Vec<(&str, NbtTag)>| {
        NbtTag::Compound(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    };
    let xyz = |x: i32, y: i32, z: i32| {
        compound(vec![
            ("x", NbtTag::Int(x)),
            ("y", NbtTag::Int(y)),
            ("z", NbtTag::Int(z)),
        ])
    };
    let block = |name: &str| compound(vec![("Name", NbtTag::String(name.to_string()))]);

    // Position (10, 20, 30), Size (-4, -5, -6): occupies x 7..=10, y 16..=20, z 25..=30.
    // Container index 0 is the minimum corner (7, 16, 25) and holds gold.
    // The last index is the primary corner (10, 20, 30) and holds a chest.
    let total = 4 * 5 * 6;
    let mut indices = vec![0usize; total];
    indices[0] = 1;
    indices[total - 1] = 2;
    let packed = schematic_format::bit_packing::pack_block_states(&indices, 2);

    let chest = compound(vec![
        ("id", NbtTag::String("minecraft:chest".into())),
        ("x", NbtTag::Int(3)),
        ("y", NbtTag::Int(4)),
        ("z", NbtTag::Int(5)),
        ("Lock", NbtTag::String("key".into())),
    ]);
    // World (7.5, 16.0, 29.5) relative to Position (10, 20, 30).
    let pig = compound(vec![
        ("id", NbtTag::String("minecraft:pig".into())),
        (
            "Pos",
            NbtTag::List(
                6,
                vec![
                    NbtTag::Double(-2.5),
                    NbtTag::Double(-4.0),
                    NbtTag::Double(-0.5),
                ],
            ),
        ),
    ]);

    let region = compound(vec![
        ("Position", xyz(10, 20, 30)),
        ("Size", xyz(-4, -5, -6)),
        (
            "BlockStatePalette",
            NbtTag::List(
                10,
                vec![
                    block("minecraft:air"),
                    block("minecraft:gold_block"),
                    block("minecraft:chest"),
                ],
            ),
        ),
        ("BlockStates", NbtTag::LongArray(packed)),
        ("TileEntities", NbtTag::List(10, vec![chest])),
        ("Entities", NbtTag::List(10, vec![pig])),
    ]);
    let root = compound(vec![
        ("MinecraftDataVersion", NbtTag::Int(3465)),
        ("Version", NbtTag::Int(6)),
        ("Metadata", compound(vec![])),
        ("Regions", compound(vec![("Neg", region)])),
    ]);
    let bytes = schematic_format::encode_gzip("", &root).unwrap();
    let parsed = LitematicDocument::parse(&bytes).unwrap();
    let id = RegionId::new("Neg");
    let reg = parsed.document.region(&id).unwrap();
    assert_eq!(reg.origin(), Position::new(7, 16, 25));

    let name_at = |p: BlockPosition| {
        reg.block_index_at(p)
            .unwrap()
            .and_then(|i| reg.palette().get(i))
            .map(|s| s.id().to_string())
    };
    assert_eq!(
        name_at(BlockPosition::new(0, 0, 0)).as_deref(),
        Some("minecraft:gold_block"),
        "container index 0 is the minimum corner"
    );
    assert_eq!(
        name_at(BlockPosition::new(3, 4, 5)).as_deref(),
        Some("minecraft:chest"),
        "last container index is the primary corner"
    );

    let be = parsed
        .document
        .block_entity_at(&id, BlockPosition::new(3, 4, 5))
        .expect("tile entity sits on its chest");
    assert_eq!(be.data.get("Lock").unwrap().as_string(), Some("key"));

    let pig = parsed.document.entities().next().unwrap();
    assert_eq!(
        pig.position,
        [0.5, 0.0, 4.5],
        "region-local from the minimum corner"
    );

    // Re-export keeps the original encoding byte-for-value.
    let reparsed = LitematicDocument::parse(&parsed.export(&parsed.document).unwrap()).unwrap();
    let raw = reparsed.raw_regions.get(&id).unwrap();
    assert_eq!(raw.original_size, [-4, -5, -6]);
    assert!(parsed.document.is_same_content(&reparsed.document));
    let exported_root =
        schematic_format::decode_gzip_or_raw(&parsed.export(&parsed.document).unwrap())
            .unwrap()
            .1;
    let out_region = exported_root.as_compound().unwrap()["Regions"]
        .as_compound()
        .unwrap()["Neg"]
        .as_compound()
        .unwrap()
        .clone();
    let out_pos = out_region["Entities"].as_list().unwrap()[0]
        .as_compound()
        .unwrap()["Pos"]
        .as_list()
        .unwrap()
        .iter()
        .map(|t| t.as_f64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(out_pos, vec![-2.5, -4.0, -0.5]);
    let out_te = out_region["TileEntities"].as_list().unwrap()[0]
        .as_compound()
        .unwrap()
        .clone();
    assert_eq!(
        (
            out_te["x"].as_i32(),
            out_te["y"].as_i32(),
            out_te["z"].as_i32()
        ),
        (Some(3), Some(4), Some(5))
    );
}
