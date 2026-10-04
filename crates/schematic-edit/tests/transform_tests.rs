use proptest::prelude::*;
use schematic_core::{
    BlockProperty, BlockState, Document, Position, Region, RegionId, Selection, Size,
};
use schematic_edit::transform::{
    mirror_block_state, mirror_relative_coords, rotate_block_state, rotate_relative_coords,
    MirrorAxis, RotationAngle,
};
use schematic_edit::{
    Clipboard, EditWorkspace, FillCommand, MirrorCommand, MoveCommand, PasteCommand, RotateCommand,
};

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        .. ProptestConfig::default()
    })]

    #[test]
    fn prop_four_90_rotations_is_identity(
        w in 1i64..50,
        h in 1i64..50,
        x in 0i64..50,
        z in 0i64..50,
    ) {
        let x = x % w;
        let z = z % h;

        // 1st 90 deg rotation
        let (x1, z1) = rotate_relative_coords(x, z, w, h, RotationAngle::Deg90);
        let w1 = h;
        let h1 = w;

        // 2nd 90 deg rotation
        let (x2, z2) = rotate_relative_coords(x1, z1, w1, h1, RotationAngle::Deg90);
        let w2 = w;
        let h2 = h;

        // 3rd 90 deg rotation
        let (x3, z3) = rotate_relative_coords(x2, z2, w2, h2, RotationAngle::Deg90);
        let w3 = h;
        let h3 = w;

        // 4th 90 deg rotation
        let (x4, z4) = rotate_relative_coords(x3, z3, w3, h3, RotationAngle::Deg90);

        prop_assert_eq!((x4, z4), (x, z));
    }

    #[test]
    fn prop_two_mirrors_is_identity(
        w in 1i64..50,
        h in 1i64..50,
        x in 0i64..50,
        z in 0i64..50,
    ) {
        let x = x % w;
        let z = z % h;

        let (mx1, mz1) = mirror_relative_coords(x, z, w, h, MirrorAxis::X);
        let (mx2, mz2) = mirror_relative_coords(mx1, mz1, w, h, MirrorAxis::X);
        prop_assert_eq!((mx2, mz2), (x, z));

        let (mz1_z, mz2_z) = mirror_relative_coords(x, z, w, h, MirrorAxis::Z);
        let (mz1_z2, mz2_z2) = mirror_relative_coords(mz1_z, mz2_z, w, h, MirrorAxis::Z);
        prop_assert_eq!((mz1_z2, mz2_z2), (x, z));
    }

    #[test]
    fn prop_block_state_four_rotations_is_identity(
        facing in prop_oneof![Just("north"), Just("east"), Just("south"), Just("west")],
        rot in 0u32..16,
    ) {
        let state = BlockState::new(
            "minecraft:oak_sign",
            vec![
                BlockProperty::new("facing", facing),
                BlockProperty::new("rotation", rot.to_string()),
            ],
        ).unwrap();

        let s1 = rotate_block_state(&state, RotationAngle::Deg90);
        let s2 = rotate_block_state(&s1, RotationAngle::Deg90);
        let s3 = rotate_block_state(&s2, RotationAngle::Deg90);
        let s4 = rotate_block_state(&s3, RotationAngle::Deg90);

        prop_assert_eq!(s4, state);
    }

    #[test]
    fn prop_block_state_two_mirrors_is_identity(
        facing in prop_oneof![Just("north"), Just("east"), Just("south"), Just("west")],
        shape in prop_oneof![Just("straight"), Just("inner_left"), Just("inner_right"), Just("outer_left"), Just("outer_right")],
        rot in 0u32..16,
    ) {
        let state = BlockState::new(
            "minecraft:oak_stairs",
            vec![
                BlockProperty::new("facing", facing),
                BlockProperty::new("shape", shape),
                BlockProperty::new("rotation", rot.to_string()),
            ],
        ).unwrap();

        let sx1 = mirror_block_state(&state, MirrorAxis::X);
        let sx2 = mirror_block_state(&sx1, MirrorAxis::X);
        prop_assert_eq!(sx2, state.clone());

        let sz1 = mirror_block_state(&state, MirrorAxis::Z);
        let sz2 = mirror_block_state(&sz1, MirrorAxis::Z);
        prop_assert_eq!(sz2, state);
    }
}

fn create_test_doc() -> (Document, RegionId) {
    let region_id = RegionId::new("main");
    let region = Region::new(
        region_id.clone(),
        Position::new(0, 0, 0),
        Size::new(10, 10, 10),
    );
    let mut doc = Document::new(schematic_core::DocumentMetadata::default());
    doc.insert_region(region);
    (doc, region_id)
}

fn state_at(
    document: &Document,
    region_id: &RegionId,
    pos: schematic_core::BlockPosition,
) -> Option<BlockState> {
    let region = document.region(region_id)?;
    let index = region.block_index_at(pos).ok()??;
    region.palette().get(index).cloned()
}

#[test]
fn test_paste_command_with_undo_redo() {
    let (doc, region_id) = create_test_doc();
    let mut ws = EditWorkspace::start(doc);

    // First fill a 2x1x2 box with stone
    let sel = Selection::from_corners(Position::new(0, 0, 0), Position::new(1, 0, 1));
    let stone = BlockState::new("minecraft:stone", vec![]).unwrap();
    ws.preview_command(&FillCommand::new(
        region_id.clone(),
        sel.clone(),
        stone.clone(),
    ))
    .unwrap();
    ws.commit_preview().unwrap();

    // Copy this 2x1x2 to clipboard
    let clipboard =
        Clipboard::from_region_selection(ws.committed().region(&region_id).unwrap(), &sel).unwrap();
    assert_eq!(clipboard.blocks.len(), 4);

    // Paste at (5, 0, 5)
    let paste_target = Position::new(5, 0, 5);
    ws.preview_command(&PasteCommand::new(
        region_id.clone(),
        paste_target,
        clipboard.clone(),
    ))
    .unwrap();
    ws.commit_preview().unwrap();

    let pasted_state = state_at(
        ws.committed(),
        &region_id,
        schematic_core::BlockPosition::new(5, 0, 5),
    );
    assert_eq!(pasted_state, Some(stone.clone()));

    // Undo paste
    ws.undo().unwrap();
    assert_eq!(
        state_at(
            ws.committed(),
            &region_id,
            schematic_core::BlockPosition::new(5, 0, 5)
        ),
        None
    );

    // Redo paste
    ws.redo().unwrap();
    assert_eq!(
        state_at(
            ws.committed(),
            &region_id,
            schematic_core::BlockPosition::new(5, 0, 5)
        ),
        Some(stone)
    );
}

#[test]
fn test_move_command_with_undo_redo() {
    let (doc, region_id) = create_test_doc();
    let mut ws = EditWorkspace::start(doc);

    // Fill at (1, 0, 1)
    let sel = Selection::from_corners(Position::new(1, 0, 1), Position::new(1, 0, 1));
    let gold = BlockState::new("minecraft:gold_block", vec![]).unwrap();
    ws.preview_command(&FillCommand::new(
        region_id.clone(),
        sel.clone(),
        gold.clone(),
    ))
    .unwrap();
    ws.commit_preview().unwrap();

    // Move by (2, 0, 2)
    ws.preview_command(&MoveCommand::new(region_id.clone(), sel.clone(), [2, 0, 2]))
        .unwrap();
    ws.commit_preview().unwrap();

    assert_eq!(
        state_at(
            ws.committed(),
            &region_id,
            schematic_core::BlockPosition::new(1, 0, 1)
        ),
        None
    );
    assert_eq!(
        state_at(
            ws.committed(),
            &region_id,
            schematic_core::BlockPosition::new(3, 0, 3)
        ),
        Some(gold.clone())
    );

    // Undo move
    ws.undo().unwrap();
    assert_eq!(
        state_at(
            ws.committed(),
            &region_id,
            schematic_core::BlockPosition::new(1, 0, 1)
        ),
        Some(gold)
    );
    assert_eq!(
        state_at(
            ws.committed(),
            &region_id,
            schematic_core::BlockPosition::new(3, 0, 3)
        ),
        None
    );
}

#[test]
fn test_rotate_and_mirror_commands() {
    let (doc, region_id) = create_test_doc();
    let mut ws = EditWorkspace::start(doc);

    // Place a directional stair facing north at (1, 0, 0) in a 3x1x3 selection
    let stair = BlockState::new(
        "minecraft:oak_stairs",
        vec![
            BlockProperty::new("facing", "north"),
            BlockProperty::new("shape", "straight"),
        ],
    )
    .unwrap();
    let sel = Selection::from_corners(Position::new(0, 0, 0), Position::new(2, 0, 2));

    // Fill specific cell by using PasteCommand with single block
    let clip = Clipboard::new(
        [3, 1, 3],
        vec![schematic_edit::ClipboardBlock {
            offset: [1, 0, 0],
            state: stair.clone(),
        }],
    );
    ws.preview_command(&PasteCommand::new(
        region_id.clone(),
        Position::new(0, 0, 0),
        clip,
    ))
    .unwrap();
    ws.commit_preview().unwrap();

    // Rotate 90 degrees
    ws.preview_command(&RotateCommand::new(
        region_id.clone(),
        sel.clone(),
        RotationAngle::Deg90,
    ))
    .unwrap();
    ws.commit_preview().unwrap();

    // After 90 deg rotation of 3x3 at offset [1, 0, 0]:
    // rel_x = 1, rel_z = 0 -> (size_z - 1 - 0, 1) = (2, 1)
    let rotated_stair = state_at(
        ws.committed(),
        &region_id,
        schematic_core::BlockPosition::new(2, 0, 1),
    )
    .unwrap();
    assert_eq!(rotated_stair.property("facing"), Some("east"));

    // Mirror along X
    ws.preview_command(&MirrorCommand::new(
        region_id.clone(),
        sel.clone(),
        MirrorAxis::X,
    ))
    .unwrap();
    ws.commit_preview().unwrap();

    // (2, 0, 1) mirrored along X: rel_x = 2, size_x = 3 -> 3 - 1 - 2 = 0
    let mirrored_stair = state_at(
        ws.committed(),
        &region_id,
        schematic_core::BlockPosition::new(0, 0, 1),
    )
    .unwrap();
    // Facing "east" mirrored along X becomes "west"
    assert_eq!(mirrored_stair.property("facing"), Some("west"));
}
