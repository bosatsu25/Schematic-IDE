use schematic_core::{
    BlockEntityRef, BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata, Position,
    Region, RegionId, Size,
};
use schematic_diff::{diff_documents, diff_regions, DiffKind};

fn create_test_region(id: &str, blocks: Vec<(i64, i64, i64, &str)>) -> Region {
    let mut region = Region::new(
        RegionId::new(id),
        Position::new(0, 0, 0),
        Size::new(16, 16, 16),
    );
    for (x, y, z, name) in blocks {
        let state = BlockState::new(name, vec![]).unwrap();
        let idx = region.palette_mut().intern(state).unwrap();
        region
            .set_block_index(BlockPosition::new(x, y, z), Some(idx))
            .unwrap();
    }
    region
}

#[test]
fn test_identical_regions_no_changes() {
    let r1 = create_test_region(
        "Main",
        vec![
            (0, 0, 0, "minecraft:stone"),
            (1, 0, 0, "minecraft:oak_planks"),
        ],
    );
    let r2 = create_test_region(
        "Main",
        vec![
            (0, 0, 0, "minecraft:stone"),
            (1, 0, 0, "minecraft:oak_planks"),
        ],
    );

    let (summary, diffs) = diff_regions(r1.id(), Some(&r1), Some(&r2), false);
    assert_eq!(summary.added_count, 0);
    assert_eq!(summary.removed_count, 0);
    assert_eq!(summary.modified_count, 0);
    assert_eq!(summary.unchanged_count, 2);
    assert!(diffs.is_empty());

    let (summary_all, diffs_all) = diff_regions(r1.id(), Some(&r1), Some(&r2), true);
    assert_eq!(summary_all.unchanged_count, 2);
    assert_eq!(diffs_all.len(), 2);
    assert!(diffs_all.iter().all(|d| d.kind == DiffKind::Unchanged));
}

#[test]
fn test_diff_added_blocks() {
    let r1 = create_test_region("Main", vec![(0, 0, 0, "minecraft:stone")]);
    let r2 = create_test_region(
        "Main",
        vec![(0, 0, 0, "minecraft:stone"), (2, 0, 0, "minecraft:glass")],
    );

    let (summary, diffs) = diff_regions(r1.id(), Some(&r1), Some(&r2), false);
    assert_eq!(summary.added_count, 1);
    assert_eq!(summary.removed_count, 0);
    assert_eq!(summary.modified_count, 0);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].kind, DiffKind::Added);
    assert_eq!(diffs[0].position, [2, 0, 0]);
    assert_eq!(diffs[0].after.as_ref().unwrap().name, "minecraft:glass");
}

#[test]
fn test_diff_removed_blocks() {
    let r1 = create_test_region(
        "Main",
        vec![(0, 0, 0, "minecraft:stone"), (1, 1, 1, "minecraft:dirt")],
    );
    let r2 = create_test_region("Main", vec![(0, 0, 0, "minecraft:stone")]);

    let (summary, diffs) = diff_regions(r1.id(), Some(&r1), Some(&r2), false);
    assert_eq!(summary.added_count, 0);
    assert_eq!(summary.removed_count, 1);
    assert_eq!(summary.modified_count, 0);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].kind, DiffKind::Removed);
    assert_eq!(diffs[0].position, [1, 1, 1]);
    assert_eq!(diffs[0].before.as_ref().unwrap().name, "minecraft:dirt");
}

#[test]
fn test_diff_modified_blocks() {
    let furnace_north = BlockState::new(
        "minecraft:furnace",
        vec![BlockProperty::new("facing", "north")],
    )
    .unwrap();

    let furnace_south = BlockState::new(
        "minecraft:furnace",
        vec![BlockProperty::new("facing", "south")],
    )
    .unwrap();

    let mut r1 = Region::new(
        RegionId::new("Main"),
        Position::new(0, 0, 0),
        Size::new(16, 16, 16),
    );
    let idx1 = r1.palette_mut().intern(furnace_north).unwrap();
    r1.set_block_index(BlockPosition::new(3, 4, 5), Some(idx1))
        .unwrap();

    let mut r2 = Region::new(
        RegionId::new("Main"),
        Position::new(0, 0, 0),
        Size::new(16, 16, 16),
    );
    let idx2 = r2.palette_mut().intern(furnace_south).unwrap();
    r2.set_block_index(BlockPosition::new(3, 4, 5), Some(idx2))
        .unwrap();

    let (summary, diffs) = diff_regions(r1.id(), Some(&r1), Some(&r2), false);
    assert_eq!(summary.added_count, 0);
    assert_eq!(summary.removed_count, 0);
    assert_eq!(summary.modified_count, 1);
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].kind, DiffKind::Modified);
    assert_eq!(
        diffs[0].before.as_ref().unwrap().properties.get("facing"),
        Some(&"north".to_string())
    );
    assert_eq!(
        diffs[0].after.as_ref().unwrap().properties.get("facing"),
        Some(&"south".to_string())
    );
}

#[test]
fn test_document_diff_with_entities() {
    let mut doc_before = Document::new(DocumentMetadata::default());
    let mut doc_after = Document::new(DocumentMetadata::default());

    let r_before = create_test_region("R1", vec![(0, 0, 0, "minecraft:stone")]);
    let r_after = create_test_region("R1", vec![(0, 0, 0, "minecraft:granite")]);

    doc_before.insert_region(r_before);
    doc_after.insert_region(r_after);

    // Add block entity to doc_after
    doc_after.add_block_entity(BlockEntityRef::new(
        "minecraft:chest",
        Position::new(0, 0, 0),
    ));

    let diff = diff_documents(&doc_before, &doc_after, false);
    assert_eq!(diff.total_modified, 1);
    assert_eq!(diff.total_added, 0);
    assert_eq!(diff.total_removed, 0);
    assert_eq!(diff.block_diffs.len(), 1);
    assert_eq!(diff.block_diffs[0].kind, DiffKind::Modified);

    assert_eq!(diff.entity_diffs.len(), 1);
    assert_eq!(diff.entity_diffs[0].kind, DiffKind::Added);
    assert_eq!(
        diff.entity_diffs[0].after_type.as_deref(),
        Some("minecraft:chest")
    );
}
