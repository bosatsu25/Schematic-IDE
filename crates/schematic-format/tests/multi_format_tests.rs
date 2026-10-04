use schematic_core::{
    BlockEntityRef, BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata,
    EntityRef, Position, Region, RegionId, Size,
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
        "minecraft:chest",
        Position::new(2, 1, 2),
    ));

    // Add entity
    doc.add_entity(EntityRef::new("minecraft:cow", [1.5, 1.0, 1.5]));

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
