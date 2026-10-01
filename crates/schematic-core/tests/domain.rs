use proptest::prelude::*;
use schematic_core::{
    BlockPosition, BlockProperty, BlockState, Bounds, Chunk, ChunkPosition, Document,
    DocumentMetadata, EntityRef, Palette, PaletteIndex, Position, Region, RegionId, Selection,
    Size,
};

fn state(properties: &[(&str, &str)]) -> BlockState {
    BlockState::new(
        "minecraft:oak_stairs",
        properties
            .iter()
            .map(|(name, value)| BlockProperty::new(*name, *value)),
    )
    .expect("test state is valid")
}

#[test]
fn positions_keep_positive_and_negative_coordinates() {
    assert_eq!(Position::new(12, 34, 56).x, 12);
    assert_eq!(Position::new(-12, -34, -56).z, -56);
}

#[test]
fn region_bounds_are_origin_inclusive_and_size_exclusive() {
    let region = Region::new(
        RegionId::new("main"),
        Position::new(-3, 4, 9),
        Size::new(2, 3, 4),
    );

    assert_eq!(
        region.bounds(),
        Bounds::new(Position::new(-3, 4, 9), [-1, 7, 13])
    );
    assert!(region.contains(Position::new(-2, 6, 12)));
    assert!(!region.contains(Position::new(-1, 6, 12)));
    assert!(!region.contains(Position::new(-4, 6, 12)));
}

#[test]
fn zero_sized_region_is_empty_and_contains_no_positions() {
    let region = Region::new(
        RegionId::new("empty"),
        Position::new(0, 0, 0),
        Size::new(0, 1, 1),
    );

    assert!(region.is_empty());
    assert!(!region.contains(Position::new(0, 0, 0)));
}

#[test]
fn region_converts_between_world_and_local_coordinates() {
    let region = Region::new(
        RegionId::new("translated"),
        Position::new(-10, 20, -30),
        Size::new(4, 5, 6),
    );
    let local = BlockPosition::new(3, 4, 5);

    assert_eq!(
        region.local_to_world(local),
        Some(Position::new(-7, 24, -25))
    );
    assert_eq!(
        region.world_to_local(Position::new(-7, 24, -25)),
        Some(local)
    );
    assert_eq!(region.world_to_local(Position::new(-11, 24, -25)), None);
}

#[test]
fn region_coordinate_conversion_rejects_overflow() {
    let region = Region::new(
        RegionId::new("edge"),
        Position::new(i32::MAX, 0, 0),
        Size::new(2, 1, 1),
    );

    assert_eq!(region.local_to_world(BlockPosition::new(1, 0, 0)), None);
}

#[test]
fn block_positions_map_to_internal_chunks_at_positive_and_negative_boundaries() {
    assert_eq!(
        ChunkPosition::from_block_position(BlockPosition::new(15, 15, 15)),
        ChunkPosition::new(0, 0, 0)
    );
    assert_eq!(
        ChunkPosition::from_block_position(BlockPosition::new(16, 16, 16)),
        ChunkPosition::new(1, 1, 1)
    );
    assert_eq!(
        ChunkPosition::from_block_position(BlockPosition::new(-1, -1, -1)),
        ChunkPosition::new(-1, -1, -1)
    );
    assert_eq!(
        ChunkPosition::from_block_position(BlockPosition::new(-16, -16, -16)),
        ChunkPosition::new(-1, -1, -1)
    );
    assert_eq!(
        ChunkPosition::from_block_position(BlockPosition::new(-17, -17, -17)),
        ChunkPosition::new(-2, -2, -2)
    );
}

#[test]
fn chunk_index_uses_x_fastest_then_z_then_y_order() {
    assert_eq!(Chunk::index_for(BlockPosition::new(1, 0, 0)), Some(1));
    assert_eq!(Chunk::index_for(BlockPosition::new(0, 0, 1)), Some(16));
    assert_eq!(Chunk::index_for(BlockPosition::new(0, 1, 0)), Some(256));
    assert_eq!(Chunk::index_for(BlockPosition::new(16, 0, 0)), None);
}

#[test]
fn chunk_position_round_trips_negative_world_local_blocks() {
    let block = BlockPosition::new(-17, 33, -1);
    let chunk_position = ChunkPosition::from_block_position(block);
    let index = chunk_position
        .index_for_block(block)
        .expect("block belongs to its derived chunk");

    assert_eq!(chunk_position, ChunkPosition::new(-2, 2, -1));
    assert_eq!(chunk_position.block_position_for_index(index), Some(block));
}

#[test]
fn storage_chunk_uses_palette_indices() {
    let mut chunk = Chunk::default();
    let position = BlockPosition::new(4, 2, 8);
    let index = Chunk::index_for(position).expect("position is in the chunk");

    assert_eq!(chunk.get(index), Ok(None));
    chunk.set(index, Some(PaletteIndex::new(3))).unwrap();
    assert_eq!(chunk.get(index), Ok(Some(PaletteIndex::new(3))));
    chunk.set(index, None).unwrap();
    assert_eq!(chunk.get(index), Ok(None));
    assert!(chunk.is_empty());
    assert!(chunk.get(schematic_core::CHUNK_VOLUME).is_err());
}

#[test]
fn selection_normalizes_corners_and_intersects_half_open_bounds() {
    let selection = Selection::from_corners(Position::new(4, -2, 8), Position::new(1, 2, 5));
    assert_eq!(
        selection.bounds(),
        Bounds::new(Position::new(1, -2, 5), [5, 3, 9])
    );

    let other = Selection::from_corners(Position::new(3, 0, 7), Position::new(6, 4, 10));
    assert_eq!(
        selection.intersection(&other).map(|area| area.bounds()),
        Some(Bounds::new(Position::new(3, 0, 7), [5, 3, 9]))
    );

    let disjoint = Selection::from_corners(Position::new(20, 20, 20), Position::new(21, 21, 21));
    assert_eq!(selection.intersection(&disjoint), None);
}

#[test]
fn palette_deduplicates_canonical_block_states() {
    let mut palette = Palette::new();
    let stairs_a = state(&[
        ("facing", "north"),
        ("half", "bottom"),
        ("shape", "straight"),
        ("waterlogged", "false"),
    ]);
    let stairs_b = state(&[
        ("waterlogged", "false"),
        ("shape", "straight"),
        ("half", "bottom"),
        ("facing", "north"),
    ]);

    assert_eq!(stairs_a, stairs_b);
    assert_eq!(palette.intern(stairs_a.clone()), Ok(PaletteIndex::new(0)));
    assert_eq!(palette.intern(stairs_b), Ok(PaletteIndex::new(0)));
    assert_eq!(palette.len(), 1);
    assert_eq!(palette.get(PaletteIndex::new(0)), Some(&stairs_a));
}

#[test]
fn region_stores_palette_indexed_blocks_by_local_chunk() {
    let mut region = Region::new(
        RegionId::new("storage"),
        Position::new(100, -20, 40),
        Size::new(17, 2, 2),
    );
    let palette_index = region
        .palette_mut()
        .intern(state(&[]))
        .expect("palette has capacity");
    let local = BlockPosition::new(16, 1, 1);

    assert_eq!(region.set_block_index(local, Some(palette_index)), Ok(None));
    assert_eq!(region.block_index_at(local), Ok(Some(palette_index)));
    assert_eq!(region.chunks().count(), 1);
    assert_eq!(
        region.set_block_index(BlockPosition::new(17, 0, 0), Some(palette_index)),
        Err(schematic_core::RegionBlockError::OutsideRegion)
    );
    assert_eq!(
        region.set_block_index(local, Some(PaletteIndex::new(8))),
        Err(schematic_core::RegionBlockError::UnknownPaletteIndex(
            PaletteIndex::new(8)
        ))
    );
    assert_eq!(region.set_block_index(local, None), Ok(Some(palette_index)));
    assert_eq!(region.chunks().count(), 0);
}

#[test]
fn block_state_properties_are_sorted_and_duplicate_names_are_rejected() {
    let block_state = state(&[("waterlogged", "false"), ("facing", "north")]);
    let properties: Vec<_> = block_state.properties().collect();

    assert_eq!(properties[0].name(), "facing");
    assert_eq!(properties[1].name(), "waterlogged");
    assert!(BlockState::new(
        "minecraft:oak_stairs",
        [
            BlockProperty::new("facing", "north"),
            BlockProperty::new("facing", "south")
        ]
    )
    .is_err());
}

#[test]
fn document_holds_regions_metadata_and_entity_references() {
    let mut document = Document::new(DocumentMetadata::default());
    let region = Region::new(
        RegionId::new("room"),
        Position::new(0, 0, 0),
        Size::new(1, 1, 1),
    );
    document.insert_region(region);
    document.add_entity(EntityRef::new("minecraft:armor_stand", [0.5, 1.0, 0.5]));
    document.add_block_entity(schematic_core::BlockEntityRef::new(
        "minecraft:chest",
        Position::new(0, 0, 0),
    ));

    assert_eq!(document.regions().count(), 1);
    assert_eq!(document.entities().count(), 1);
    assert_eq!(document.block_entities().count(), 1);
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 4096,
        .. ProptestConfig::default()
    })]

    #[test]
    fn local_block_coordinate_chunk_index_round_trip(x in any::<i64>(), y in any::<i64>(), z in any::<i64>()) {
        let block = BlockPosition::new(x, y, z);
        let chunk = ChunkPosition::from_block_position(block);
        let index = chunk.index_for_block(block);

        prop_assert_eq!(index.and_then(|index| chunk.block_position_for_index(index)), Some(block));
    }

    #[test]
    fn selection_normalization_contains_both_corners(
        ax in any::<i32>(), ay in any::<i32>(), az in any::<i32>(),
        bx in any::<i32>(), by in any::<i32>(), bz in any::<i32>()
    ) {
        let a = Position::new(ax, ay, az);
        let b = Position::new(bx, by, bz);
        let selection = Selection::from_corners(a, b);

        prop_assert!(selection.contains(a));
        prop_assert!(selection.contains(b));
        prop_assert_eq!(selection.bounds().min(), Position::new(ax.min(bx), ay.min(by), az.min(bz)));
    }

    #[test]
    fn region_world_local_conversion_round_trip(
        ox in any::<i32>(), oy in any::<i32>(), oz in any::<i32>(),
        x in 0i64..256, y in 0i64..256, z in 0i64..256
    ) {
        let region = Region::new(
            RegionId::new("property"),
            Position::new(ox, oy, oz),
            Size::new(256, 256, 256),
        );
        let local = BlockPosition::new(x, y, z);

        if let Some(world) = region.local_to_world(local) {
            prop_assert_eq!(region.world_to_local(world), Some(local));
        }
    }
}
