use proptest::prelude::*;
use schematic_core::{
    BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata, Position, Region,
    RegionId, Selection, Size,
};
use schematic_edit::{DeleteCommand, EditCommand, FillCommand, History, ReplaceCommand};

fn block_state(id: &str) -> BlockState {
    BlockState::new(id, [BlockProperty::new("variant", "default")])
        .expect("test block state is valid")
}

fn document(size: Size) -> Document {
    let mut document = Document::new(DocumentMetadata::default());
    document.insert_region(Region::new(
        RegionId::new("main"),
        Position::new(0, 0, 0),
        size,
    ));
    document
}

fn state_at(document: &Document, position: BlockPosition) -> Option<&BlockState> {
    let region = document.region(&RegionId::new("main"))?;
    let index = region.block_index_at(position).ok()??;
    region.palette().get(index)
}

fn selection(first: Position, second: Position) -> Selection {
    Selection::from_corners(first, second)
}

#[test]
fn fill_patch_contains_only_changed_blocks_and_undo_redo_round_trips() {
    let mut document = document(Size::new(4, 2, 2));
    let mut history = History::new();
    let stone = block_state("minecraft:stone");
    let command = FillCommand::new(
        RegionId::new("main"),
        selection(Position::new(1, 0, 0), Position::new(2, 0, 0)),
        stone.clone(),
    );

    let patch = command.create_patch(&document).unwrap();
    assert_eq!(patch.changed_block_count(), 2);
    patch.apply(&mut document).unwrap();
    assert_eq!(
        state_at(&document, BlockPosition::new(1, 0, 0)),
        Some(&stone)
    );
    assert_eq!(
        state_at(&document, BlockPosition::new(2, 0, 0)),
        Some(&stone)
    );
    assert_eq!(state_at(&document, BlockPosition::new(0, 0, 0)), None);
    patch.revert(&mut document).unwrap();

    history.execute(&mut document, &command).unwrap();
    assert_eq!(history.undo_depth(), 1);
    assert!(history.undo(&mut document).unwrap());
    assert_eq!(state_at(&document, BlockPosition::new(1, 0, 0)), None);
    assert!(history.redo(&mut document).unwrap());
    assert_eq!(
        state_at(&document, BlockPosition::new(2, 0, 0)),
        Some(&stone)
    );
}

#[test]
fn replace_changes_only_matching_blocks_and_undo_restores_semantic_state() {
    let mut document = document(Size::new(3, 1, 1));
    let mut history = History::new();
    let stone = block_state("minecraft:stone");
    let dirt = block_state("minecraft:dirt");
    history
        .execute(
            &mut document,
            &FillCommand::new(
                RegionId::new("main"),
                selection(Position::new(0, 0, 0), Position::new(2, 0, 0)),
                stone.clone(),
            ),
        )
        .unwrap();
    let gold = block_state("minecraft:gold_block");
    history
        .execute(
            &mut document,
            &FillCommand::new(
                RegionId::new("main"),
                selection(Position::new(1, 0, 0), Position::new(1, 0, 0)),
                gold.clone(),
            ),
        )
        .unwrap();
    history
        .execute(
            &mut document,
            &ReplaceCommand::new(
                RegionId::new("main"),
                selection(Position::new(0, 0, 0), Position::new(2, 0, 0)),
                stone.clone(),
                dirt.clone(),
            ),
        )
        .unwrap();

    assert_eq!(
        state_at(&document, BlockPosition::new(0, 0, 0)),
        Some(&dirt)
    );
    assert_eq!(
        state_at(&document, BlockPosition::new(1, 0, 0)),
        Some(&gold)
    );
    assert_eq!(
        state_at(&document, BlockPosition::new(2, 0, 0)),
        Some(&dirt)
    );
    assert!(history.undo(&mut document).unwrap());
    assert_eq!(
        state_at(&document, BlockPosition::new(0, 0, 0)),
        Some(&stone)
    );
    assert_eq!(
        state_at(&document, BlockPosition::new(1, 0, 0)),
        Some(&gold)
    );
}

#[test]
fn delete_patch_clears_selected_blocks_and_undo_restores_them() {
    let mut document = document(Size::new(3, 1, 1));
    let mut history = History::new();
    let stone = block_state("minecraft:stone");
    history
        .execute(
            &mut document,
            &FillCommand::new(
                RegionId::new("main"),
                selection(Position::new(0, 0, 0), Position::new(2, 0, 0)),
                stone.clone(),
            ),
        )
        .unwrap();
    history
        .execute(
            &mut document,
            &DeleteCommand::new(
                RegionId::new("main"),
                selection(Position::new(1, 0, 0), Position::new(2, 0, 0)),
            ),
        )
        .unwrap();

    assert_eq!(
        state_at(&document, BlockPosition::new(0, 0, 0)),
        Some(&stone)
    );
    assert_eq!(state_at(&document, BlockPosition::new(1, 0, 0)), None);
    assert!(history.undo(&mut document).unwrap());
    assert_eq!(
        state_at(&document, BlockPosition::new(2, 0, 0)),
        Some(&stone)
    );
}

#[test]
fn editing_outside_region_is_a_no_op() {
    let mut document = document(Size::new(1, 1, 1));
    let mut history = History::new();
    history
        .execute(
            &mut document,
            &FillCommand::new(
                RegionId::new("main"),
                selection(Position::new(5, 5, 5), Position::new(6, 6, 6)),
                block_state("minecraft:stone"),
            ),
        )
        .unwrap();

    assert_eq!(history.undo_depth(), 0);
    assert!(!history.undo(&mut document).unwrap());
}

#[test]
fn undo_conflict_is_reported_without_discarding_history() {
    let mut document = document(Size::new(1, 1, 1));
    let mut history = History::new();
    history
        .execute(
            &mut document,
            &FillCommand::new(
                RegionId::new("main"),
                selection(Position::new(0, 0, 0), Position::new(0, 0, 0)),
                block_state("minecraft:stone"),
            ),
        )
        .unwrap();
    let region = document
        .region_mut(&RegionId::new("main"))
        .expect("region exists");
    let external_index = region
        .palette_mut()
        .intern(block_state("minecraft:dirt"))
        .unwrap();
    region
        .set_block_index(BlockPosition::new(0, 0, 0), Some(external_index))
        .unwrap();

    assert!(history.undo(&mut document).is_err());
    assert_eq!(history.undo_depth(), 1);
    assert_eq!(
        state_at(&document, BlockPosition::new(0, 0, 0)),
        Some(&block_state("minecraft:dirt"))
    );
}

#[test]
fn successful_new_edit_clears_redo_history() {
    let mut document = document(Size::new(2, 1, 1));
    let mut history = History::new();
    let area = selection(Position::new(0, 0, 0), Position::new(0, 0, 0));
    history
        .execute(
            &mut document,
            &FillCommand::new(
                RegionId::new("main"),
                area.clone(),
                block_state("minecraft:stone"),
            ),
        )
        .unwrap();
    assert!(history.undo(&mut document).unwrap());
    history
        .execute(
            &mut document,
            &FillCommand::new(RegionId::new("main"), area, block_state("minecraft:dirt")),
        )
        .unwrap();

    assert_eq!(history.redo_depth(), 0);
}

#[test]
fn patch_set_groups_chunks_and_rejects_conflicts_before_any_partial_apply() {
    let mut document = document(Size::new(17, 1, 1));
    let fill = FillCommand::new(
        RegionId::new("main"),
        selection(Position::new(15, 0, 0), Position::new(16, 0, 0)),
        block_state("minecraft:stone"),
    );
    let patch = fill.create_patch(&document).unwrap();

    assert_eq!(patch.changed_block_count(), 2);
    assert_eq!(patch.patches().count(), 2);

    let region = document
        .region_mut(&RegionId::new("main"))
        .expect("region exists");
    let external_index = region
        .palette_mut()
        .intern(block_state("minecraft:dirt"))
        .unwrap();
    region
        .set_block_index(BlockPosition::new(16, 0, 0), Some(external_index))
        .unwrap();

    assert!(patch.apply(&mut document).is_err());
    assert_eq!(state_at(&document, BlockPosition::new(15, 0, 0)), None);
    assert_eq!(
        state_at(&document, BlockPosition::new(16, 0, 0)),
        Some(&block_state("minecraft:dirt"))
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        .. ProptestConfig::default()
    })]

    #[test]
    fn fill_undo_redo_preserves_generated_semantic_states(
        x in 1u32..6,
        y in 1u32..6,
        z in 1u32..6,
        origin_x in -100i32..100,
        origin_y in -100i32..100,
        origin_z in -100i32..100,
    ) {
        let mut document = Document::new(DocumentMetadata::default());
        document.insert_region(Region::new(
            RegionId::new("main"),
            Position::new(origin_x, origin_y, origin_z),
            Size::new(x, y, z),
        ));
        let mut history = History::new();
        let fill = FillCommand::new(
            RegionId::new("main"),
            selection(
                Position::new(origin_x, origin_y, origin_z),
                Position::new(
                    origin_x + x as i32 - 1,
                    origin_y + y as i32 - 1,
                    origin_z + z as i32 - 1,
                ),
            ),
            block_state("minecraft:stone"),
        );

        history.execute(&mut document, &fill).unwrap();
        let edited = state_at(&document, BlockPosition::new(0, 0, 0)).cloned();
        prop_assert_eq!(edited.as_ref(), Some(&block_state("minecraft:stone")));
        for local_x in 0..x {
            for local_y in 0..y {
                for local_z in 0..z {
                    prop_assert_eq!(
                        state_at(
                            &document,
                            BlockPosition::new(local_x as i64, local_y as i64, local_z as i64)
                        ),
                        Some(&block_state("minecraft:stone"))
                    );
                }
            }
        }
        prop_assert!(history.undo(&mut document).unwrap());
        for local_x in 0..x {
            for local_y in 0..y {
                for local_z in 0..z {
                    prop_assert_eq!(
                        state_at(
                            &document,
                            BlockPosition::new(local_x as i64, local_y as i64, local_z as i64)
                        ),
                        None
                    );
                }
            }
        }
        prop_assert!(history.redo(&mut document).unwrap());
        for local_x in 0..x {
            for local_y in 0..y {
                for local_z in 0..z {
                    prop_assert_eq!(
                        state_at(
                            &document,
                            BlockPosition::new(local_x as i64, local_y as i64, local_z as i64)
                        ),
                        edited.as_ref()
                    );
                }
            }
        }
    }
}
