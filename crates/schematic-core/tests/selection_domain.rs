use schematic_core::{
    Bounds, OperationTarget, PlacementTarget, Position, Selection, SelectionBox, WorldBoundsMapping,
};

fn cube(min: i32, max: i32) -> Bounds {
    Bounds::new(
        Position::new(min, min, min),
        [max as i64, max as i64, max as i64],
    )
}

fn sbox(min: i32, max: i32) -> SelectionBox {
    SelectionBox::new(cube(min, max))
}

#[test]
fn normal_region_uses_exclusive_maximum() {
    let region = cube(1, 5);
    assert!(region.contains(Position::new(1, 2, 4)));
    assert!(!region.contains(Position::new(5, 2, 4)));
    assert!(!region.contains(Position::new(2, 5, 4)));
    assert!(!region.contains(Position::new(2, 4, 5)));
}

#[test]
fn reversed_boundary_endpoints_normalize_per_axis() {
    let normalized = Bounds::between(Position::new(8, 1, 3), Position::new(-2, 9, -4)).unwrap();
    assert_eq!(normalized.min(), Position::new(-2, 1, -4));
    assert_eq!(normalized.max_exclusive(), [8, 9, 3]);
}

#[test]
fn single_block_intersection() {
    assert_eq!(cube(2, 3).intersection(cube(0, 10)), Some(cube(2, 3)));
}

#[test]
fn partial_intersection() {
    assert_eq!(cube(0, 5).intersection(cube(3, 8)), Some(cube(3, 5)));
}

#[test]
fn full_containment_is_symmetric() {
    assert_eq!(cube(0, 8).intersection(cube(2, 4)), Some(cube(2, 4)));
    assert_eq!(cube(2, 4).intersection(cube(0, 8)), Some(cube(2, 4)));
}

#[test]
fn no_intersection() {
    assert_eq!(cube(0, 2).intersection(cube(3, 5)), None);
}

#[test]
fn touching_face_edge_and_corner_have_no_blocks() {
    let region = cube(0, 2);
    let touching_face = Bounds::new(Position::new(2, 0, 0), [3, 2, 2]);
    assert_eq!(region.intersection(touching_face), None);

    let touching_edge = Bounds::new(Position::new(2, 2, 0), [3, 3, 2]);
    assert_eq!(region.intersection(touching_edge), None);

    let touching_corner = cube(2, 3);
    assert_eq!(region.intersection(touching_corner), None);
}

#[test]
fn negative_coordinates() {
    assert_eq!(cube(-10, -2).intersection(cube(-5, 1)), Some(cube(-5, -2)));
}

#[test]
fn multiple_selections_and_placement_gaps_are_preserved() {
    let selection = Selection::from_boxes([sbox(0, 3), sbox(8, 12)]);
    let placement = PlacementTarget::new([cube(1, 10)]);
    let target = OperationTarget::new(selection, Some(placement));

    let world_regions = target.world_regions();
    assert_eq!(world_regions, vec![cube(1, 3), cube(8, 10)]);

    assert!(!world_regions
        .iter()
        .any(|r| r.contains(Position::new(5, 5, 5))));

    let split_placement = PlacementTarget::new([cube(1, 3), cube(8, 10)]);
    let split_target =
        OperationTarget::new(Selection::from_boxes([sbox(0, 20)]), Some(split_placement));
    assert_eq!(target.world_regions(), split_target.world_regions());
}

#[test]
fn empty_selection_and_missing_placement_grant_nothing() {
    let empty_sel = Selection::empty();
    let target_empty_sel =
        OperationTarget::new(empty_sel, Some(PlacementTarget::new([cube(0, 5)])));
    assert!(target_empty_sel.world_regions().is_empty());

    let target_no_placement = OperationTarget::new(Selection::from_boxes([sbox(0, 5)]), None);
    assert!(target_no_placement.world_regions().is_empty());

    assert!(OperationTarget::empty().world_regions().is_empty());
}

#[test]
fn overlapping_boxes_remain_a_union_without_filling_outside() {
    let selection = Selection::from_boxes([sbox(0, 4), sbox(2, 6)]);
    let placement = PlacementTarget::new([cube(0, 10)]);
    let target = OperationTarget::new(selection, Some(placement));

    for x in -1..=7 {
        for y in -1..=7 {
            for z in -1..=7 {
                let p = Position::new(x, y, z);
                let expected = cube(0, 4).contains(p) || cube(2, 6).contains(p);
                let actual = target.world_regions().iter().any(|r| r.contains(p));
                assert_eq!(expected, actual, "Mismatch at {x}, {y}, {z}");
            }
        }
    }
}

#[test]
fn world_bounds_mapping_from_inclusive_and_safe_transforms() {
    let b = WorldBoundsMapping::from_inclusive(-1, 2, 3, -1, 2, 3);
    assert_eq!(b.min(), Position::new(-1, 2, 3));
    assert_eq!(b.max_exclusive(), [0, 3, 4]);

    let b2 = WorldBoundsMapping::from_inclusive(5, -1, 9, -2, 8, 3);
    let b3 = WorldBoundsMapping::from_inclusive(-2, 8, 3, 5, -1, 9);
    assert_eq!(b2, b3);

    let max_b = WorldBoundsMapping::from_inclusive(i32::MIN, 0, i32::MAX, i32::MAX, 0, i32::MAX);
    assert_eq!(max_b.min(), Position::new(i32::MIN, 0, i32::MAX));
    assert_eq!(
        max_b.max_exclusive(),
        [i32::MAX as i64 + 1, 1, i32::MAX as i64 + 1]
    );

    assert!(WorldBoundsMapping::require_safe_transform(-10, 2, 3, 4, -5, 6, -7, 8, -9).is_ok());
    assert!(WorldBoundsMapping::require_safe_transform(i32::MAX, 0, 0, 0, 0, 0, 1, 1, 1).is_ok());

    assert!(WorldBoundsMapping::require_safe_transform(i32::MAX, 0, 0, 1, 0, 0, 1, 1, 1).is_err());
    assert!(WorldBoundsMapping::require_safe_transform(0, 0, 0, 0, 0, 0, 0, 1, 1).is_err());
}
