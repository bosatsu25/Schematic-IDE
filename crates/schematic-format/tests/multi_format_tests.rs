use schematic_core::{
    BlockEntityRef, BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata,
    EntityRef, NbtTag, Position, Region, RegionId, Size,
};
use schematic_format::{
    detect_format, LitematicDocument, SchematicFormatType, SpongeSchematic, StructureDocument,
};

fn create_sample_doc() -> Document {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("TestMultiFormat".to_string()),
        author: Some("MultiTester".to_string()),
        ..Default::default()
    });

    let region_id = RegionId::new("Main");
    let origin = Position::new(0, 0, 0);
    let size = Size::new(4, 3, 4);
    let mut region = Region::new(region_id.clone(), origin, size);

    let stone = BlockState::new("minecraft:stone", []).unwrap();
    let oak_stairs = BlockState::new(
        "minecraft:oak_stairs",
        vec![
            BlockProperty::new("facing", "east"),
            BlockProperty::new("half", "bottom"),
        ],
    )
    .unwrap();

    let stone_idx = region.palette_mut().intern(stone).unwrap();
    let stairs_idx = region.palette_mut().intern(oak_stairs).unwrap();

    // Place a floor of stone
    for x in 0..4 {
        for z in 0..4 {
            region
                .set_block_index(BlockPosition::new(x, 0, z), Some(stone_idx))
                .unwrap();
        }
    }

    // Place stairs
    region
        .set_block_index(BlockPosition::new(1, 1, 1), Some(stairs_idx))
        .unwrap();

    doc.insert_region(region);

    // Add block entity
    doc.add_block_entity(BlockEntityRef::new(
        RegionId::new("Main"),
        "minecraft:chest",
        BlockPosition::new(1, 1, 1),
    ));

    // Add entity
    doc.add_entity(EntityRef::new(
        RegionId::new("Main"),
        "minecraft:cow",
        [1.5, 1.0, 1.5],
    ));

    doc
}

#[test]
fn test_sponge_schematic_roundtrip() {
    let doc = create_sample_doc();
    let sponge = SpongeSchematic::from_document(doc.clone(), 2975);

    let bytes = sponge.export(&doc).expect("export sponge schematic");
    assert!(!bytes.is_empty());

    // Format detection
    let detected = detect_format(&bytes).expect("detect sponge");
    assert_eq!(detected, SchematicFormatType::SpongeSchematic);

    // Parse back
    let parsed = SpongeSchematic::parse(&bytes).expect("parse sponge schematic");
    assert_eq!(parsed.data_version, 2975);

    let reg = parsed.document.regions().next().unwrap();
    assert_eq!(reg.size(), Size::new(4, 3, 4));

    // Verify stone floor
    for x in 0..4 {
        for z in 0..4 {
            let idx = reg
                .block_index_at(BlockPosition::new(x, 0, z))
                .unwrap()
                .unwrap();
            let state = reg.palette().get(idx).unwrap();
            assert_eq!(state.id(), "minecraft:stone");
        }
    }

    // Verify stairs
    let stairs_idx = reg
        .block_index_at(BlockPosition::new(1, 1, 1))
        .unwrap()
        .unwrap();
    let stairs_state = reg.palette().get(stairs_idx).unwrap();
    assert_eq!(stairs_state.id(), "minecraft:oak_stairs");
    assert_eq!(stairs_state.property("facing"), Some("east"));

    // Verify block entity & entity
    assert_eq!(parsed.document.block_entities().count(), 1);
    assert_eq!(parsed.document.entities().count(), 1);
}

#[test]
fn test_structure_nbt_roundtrip() {
    let doc = create_sample_doc();
    let structure = StructureDocument::from_document(doc.clone(), 2975);

    let bytes = structure.export(&doc).expect("export structure nbt");
    assert!(!bytes.is_empty());

    // Format detection
    let detected = detect_format(&bytes).expect("detect structure");
    assert_eq!(detected, SchematicFormatType::StructureNbt);

    // Parse back
    let parsed = StructureDocument::parse(&bytes).expect("parse structure nbt");
    assert_eq!(parsed.data_version, 2975);

    let reg = parsed.document.regions().next().unwrap();
    assert_eq!(reg.size(), Size::new(4, 3, 4));

    // Verify stone floor
    for x in 0..4 {
        for z in 0..4 {
            let idx = reg
                .block_index_at(BlockPosition::new(x, 0, z))
                .unwrap()
                .unwrap();
            let state = reg.palette().get(idx).unwrap();
            assert_eq!(state.id(), "minecraft:stone");
        }
    }

    // Verify stairs
    let stairs_idx = reg
        .block_index_at(BlockPosition::new(1, 1, 1))
        .unwrap()
        .unwrap();
    let stairs_state = reg.palette().get(stairs_idx).unwrap();
    assert_eq!(stairs_state.id(), "minecraft:oak_stairs");
    assert_eq!(stairs_state.property("facing"), Some("east"));

    // Verify block entity & entity
    assert_eq!(parsed.document.block_entities().count(), 1);
    assert_eq!(parsed.document.entities().count(), 1);
}

#[test]
fn test_cross_format_interoperability() {
    let original_doc = create_sample_doc();

    // Export original to Sponge
    let sponge_adapter = SpongeSchematic::from_document(original_doc.clone(), 2975);
    let sponge_bytes = sponge_adapter.export(&original_doc).unwrap();

    // Parse Sponge into new Document
    let from_sponge = SpongeSchematic::parse(&sponge_bytes).unwrap().document;

    // Export from_sponge to Structure NBT
    let struct_adapter = StructureDocument::from_document(from_sponge.clone(), 2975);
    let struct_bytes = struct_adapter.export(&from_sponge).unwrap();

    // Parse Structure into new Document
    let from_struct = StructureDocument::parse(&struct_bytes).unwrap().document;

    // Export from_struct to Litematic
    let litematic_adapter = LitematicDocument::from_document(from_struct.clone(), 2975);
    let litematic_bytes = litematic_adapter.export(&from_struct).unwrap();

    // Detect format of all three
    assert_eq!(
        detect_format(&sponge_bytes).unwrap(),
        SchematicFormatType::SpongeSchematic
    );
    assert_eq!(
        detect_format(&struct_bytes).unwrap(),
        SchematicFormatType::StructureNbt
    );
    assert_eq!(
        detect_format(&litematic_bytes).unwrap(),
        SchematicFormatType::Litematic
    );

    // Parse back final litematic
    let final_doc = LitematicDocument::parse(&litematic_bytes).unwrap().document;
    let final_reg = final_doc.regions().next().unwrap();

    // Verify blocks survived full Sponge -> Structure -> Litematic roundtrip!
    let stairs_idx = final_reg
        .block_index_at(BlockPosition::new(1, 1, 1))
        .unwrap()
        .unwrap();
    let stairs_state = final_reg.palette().get(stairs_idx).unwrap();
    assert_eq!(stairs_state.id(), "minecraft:oak_stairs");
    assert_eq!(stairs_state.property("facing"), Some("east"));
}

// ---------------------------------------------------------------------------
// Block entity / entity payloads survive every format (issue #21) and
// multi-region documents survive single-volume formats (issue #20).
// ---------------------------------------------------------------------------

fn chest_doc() -> Document {
    use schematic_core::NbtCompound;
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Chests".into()),
        author: Some("tester".into()),
        ..Default::default()
    });
    let mut region = Region::new(
        RegionId::new("Main"),
        Position::new(0, 0, 0),
        Size::new(3, 2, 3),
    );
    let chest = region
        .palette_mut()
        .intern(BlockState::new("minecraft:chest", []).unwrap())
        .unwrap();
    region
        .set_block_index(BlockPosition::new(1, 0, 2), Some(chest))
        .unwrap();
    doc.insert_region(region);

    let mut item = NbtCompound::new();
    item.insert("id".into(), NbtTag::String("minecraft:diamond".into()));
    item.insert("Count".into(), NbtTag::Byte(64));
    item.insert("Slot".into(), NbtTag::Byte(3));
    let mut data = NbtCompound::new();
    data.insert(
        "Items".into(),
        NbtTag::List(10, vec![NbtTag::Compound(item)]),
    );
    data.insert("Lock".into(), NbtTag::String("".into()));
    doc.add_block_entity(
        BlockEntityRef::new(
            RegionId::new("Main"),
            "minecraft:chest",
            BlockPosition::new(1, 0, 2),
        )
        .with_data(data),
    );

    let mut ent = NbtCompound::new();
    ent.insert(
        "CustomName".into(),
        NbtTag::String("{\"text\":\"Bob\"}".into()),
    );
    ent.insert(
        "Rotation".into(),
        NbtTag::List(5, vec![NbtTag::Float(90.0), NbtTag::Float(0.0)]),
    );
    doc.add_entity(
        EntityRef::new(RegionId::new("Main"), "minecraft:villager", [0.5, 1.0, 0.5]).with_data(ent),
    );
    doc
}

fn assert_payloads_equal(a: &Document, b: &Document) {
    let ba: Vec<_> = a.block_entities().collect();
    let bb: Vec<_> = b.block_entities().collect();
    assert_eq!(ba.len(), bb.len());
    for (x, y) in ba.iter().zip(&bb) {
        assert_eq!(x.kind, y.kind);
        assert_eq!(x.position, y.position);
        assert_eq!(x.data, y.data, "block entity payload changed");
    }
    let ea: Vec<_> = a.entities().collect();
    let eb: Vec<_> = b.entities().collect();
    assert_eq!(ea.len(), eb.len());
    for (x, y) in ea.iter().zip(&eb) {
        assert_eq!(x.kind, y.kind);
        assert_eq!(x.position, y.position);
        assert_eq!(x.data, y.data, "entity payload changed");
    }
}

#[test]
fn block_entity_and_entity_payloads_roundtrip_through_every_format() {
    let doc = chest_doc();

    let lit = LitematicDocument::from_document(doc.clone(), 3465);
    let parsed = LitematicDocument::parse(&lit.export(&doc).unwrap()).unwrap();
    assert_payloads_equal(&doc, &parsed.document);

    let sponge = SpongeSchematic::from_document(doc.clone(), 3465);
    let parsed = SpongeSchematic::parse(&sponge.export(&doc).unwrap()).unwrap();
    assert_eq!(parsed.version, 2);
    assert_eq!(parsed.document.metadata().author.as_deref(), Some("tester"));
    assert_payloads_equal(&doc, &parsed.document);

    let mut sponge3 = SpongeSchematic::from_document(doc.clone(), 3465);
    sponge3.version = 3;
    let bytes = sponge3.export(&doc).unwrap();
    assert_eq!(
        detect_format(&bytes).unwrap(),
        SchematicFormatType::SpongeSchematic
    );
    let parsed = SpongeSchematic::parse(&bytes).unwrap();
    assert_eq!(parsed.version, 3);
    assert_payloads_equal(&doc, &parsed.document);

    let structure = StructureDocument::from_document(doc.clone(), 3465);
    let parsed = StructureDocument::parse(&structure.export(&doc).unwrap()).unwrap();
    assert_payloads_equal(&doc, &parsed.document);
}

#[test]
fn chest_contents_survive_full_cross_format_chain() {
    let doc = chest_doc();
    let s = SpongeSchematic::parse(
        &SpongeSchematic::from_document(doc.clone(), 3465)
            .export(&doc)
            .unwrap(),
    )
    .unwrap()
    .document;
    let t = StructureDocument::parse(
        &StructureDocument::from_document(s.clone(), 3465)
            .export(&s)
            .unwrap(),
    )
    .unwrap()
    .document;
    let l = LitematicDocument::parse(
        &LitematicDocument::from_document(t.clone(), 3465)
            .export(&t)
            .unwrap(),
    )
    .unwrap()
    .document;
    assert_payloads_equal(&doc, &l);
}

fn two_region_doc() -> Document {
    let mut doc = Document::new(DocumentMetadata::default());
    let stone = BlockState::new("minecraft:stone", []).unwrap();
    let chest = BlockState::new("minecraft:chest", []).unwrap();
    // Region A at origin (0,0,0) 2x1x2 full of stone, chest at (1,0,1).
    let mut a = Region::new(
        RegionId::new("A"),
        Position::new(0, 0, 0),
        Size::new(2, 1, 2),
    );
    let sa = a.palette_mut().intern(stone.clone()).unwrap();
    let ca = a.palette_mut().intern(chest.clone()).unwrap();
    for x in 0..2 {
        for z in 0..2 {
            a.set_block_index(BlockPosition::new(x, 0, z), Some(sa))
                .unwrap();
        }
    }
    a.set_block_index(BlockPosition::new(1, 0, 1), Some(ca))
        .unwrap();
    // Region B at origin (5,2,-3) 1x1x1 chest.
    let mut b = Region::new(
        RegionId::new("B"),
        Position::new(5, 2, -3),
        Size::new(1, 1, 1),
    );
    let cb = b.palette_mut().intern(chest).unwrap();
    b.set_block_index(BlockPosition::new(0, 0, 0), Some(cb))
        .unwrap();
    doc.insert_region(a);
    doc.insert_region(b);
    doc.add_block_entity(BlockEntityRef::new(
        RegionId::new("A"),
        "minecraft:chest",
        BlockPosition::new(1, 0, 1),
    ));
    doc.add_block_entity(BlockEntityRef::new(
        RegionId::new("B"),
        "minecraft:chest",
        BlockPosition::new(0, 0, 0),
    ));
    doc.add_entity(EntityRef::new(
        RegionId::new("B"),
        "minecraft:pig",
        [0.5, 0.0, 0.5],
    ));
    doc
}

fn non_air_world_blocks(doc: &Document) -> std::collections::BTreeMap<Position, String> {
    let mut out = std::collections::BTreeMap::new();
    for region in doc.regions() {
        for (cp, chunk) in region.chunks() {
            for (i, pi) in chunk.occupied_blocks() {
                let local = cp.block_position_for_index(i).unwrap();
                let world = region.local_to_world(local).unwrap();
                out.insert(world, region.palette().get(pi).unwrap().id().to_string());
            }
        }
    }
    out
}

fn world_block_entities(doc: &Document) -> Vec<Position> {
    doc.block_entities()
        .map(|be| {
            doc.region(&be.region)
                .unwrap()
                .local_to_world(be.position)
                .unwrap()
        })
        .collect()
}

#[test]
fn multi_region_documents_survive_single_volume_formats() {
    let doc = two_region_doc();
    let expected_blocks = non_air_world_blocks(&doc);
    let mut expected_bes = world_block_entities(&doc);
    expected_bes.sort();
    assert_eq!(expected_blocks.len(), 5);

    let sponge = SpongeSchematic::from_document(doc.clone(), 3465);
    let from_sponge = SpongeSchematic::parse(&sponge.export(&doc).unwrap())
        .unwrap()
        .document;
    // Structure NBT has no world origin; compare relative to the merged min corner.
    let structure = StructureDocument::from_document(doc.clone(), 3465);
    let from_structure = StructureDocument::parse(&structure.export(&doc).unwrap())
        .unwrap()
        .document;

    assert_eq!(non_air_world_blocks(&from_sponge), expected_blocks);
    let mut bes = world_block_entities(&from_sponge);
    bes.sort();
    assert_eq!(bes, expected_bes);
    assert_eq!(
        from_sponge.entities().next().unwrap().position,
        [5.5, 2.0, 0.5]
    );

    let shift = |p: &Position| Position::new(p.x, p.y, p.z + 3);
    let shifted: std::collections::BTreeMap<_, _> = expected_blocks
        .iter()
        .map(|(p, s)| (shift(p), s.clone()))
        .collect();
    assert_eq!(non_air_world_blocks(&from_structure), shifted);
    assert_eq!(from_structure.block_entities().count(), 2);
    assert_eq!(from_structure.entities().count(), 1);
}

#[test]
fn merge_regions_counts_overlap_conflicts() {
    let mut doc = Document::new(DocumentMetadata::default());
    for (name, block) in [("A", "minecraft:stone"), ("B", "minecraft:dirt")] {
        let mut r = Region::new(
            RegionId::new(name),
            Position::new(0, 0, 0),
            Size::new(1, 1, 2),
        );
        let i = r
            .palette_mut()
            .intern(BlockState::new(block, []).unwrap())
            .unwrap();
        r.set_block_index(BlockPosition::new(0, 0, 0), Some(i))
            .unwrap();
        doc.insert_region(r);
    }
    let merged = schematic_format::merge_regions(&doc).unwrap();
    assert_eq!(merged.overlap_conflicts, 1);
    assert_eq!(
        merged.merged_region_names,
        vec!["A".to_string(), "B".to_string()]
    );
    assert_eq!(merged.document.regions().count(), 1);
}
