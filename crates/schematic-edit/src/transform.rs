use schematic_core::{BlockProperty, BlockState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationAngle {
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorAxis {
    X,
    Z,
}

pub fn rotate_facing(facing: &str, angle: RotationAngle) -> &str {
    match facing {
        "north" => match angle {
            RotationAngle::Deg90 => "east",
            RotationAngle::Deg180 => "south",
            RotationAngle::Deg270 => "west",
        },
        "east" => match angle {
            RotationAngle::Deg90 => "south",
            RotationAngle::Deg180 => "west",
            RotationAngle::Deg270 => "north",
        },
        "south" => match angle {
            RotationAngle::Deg90 => "west",
            RotationAngle::Deg180 => "north",
            RotationAngle::Deg270 => "east",
        },
        "west" => match angle {
            RotationAngle::Deg90 => "north",
            RotationAngle::Deg180 => "east",
            RotationAngle::Deg270 => "south",
        },
        other => other,
    }
}

pub fn mirror_facing(facing: &str, axis: MirrorAxis) -> &str {
    match axis {
        MirrorAxis::X => match facing {
            "east" => "west",
            "west" => "east",
            other => other,
        },
        MirrorAxis::Z => match facing {
            "north" => "south",
            "south" => "north",
            other => other,
        },
    }
}

pub fn rotate_axis(axis_val: &str, angle: RotationAngle) -> &str {
    match angle {
        RotationAngle::Deg90 | RotationAngle::Deg270 => match axis_val {
            "x" => "z",
            "z" => "x",
            other => other,
        },
        RotationAngle::Deg180 => axis_val,
    }
}

pub fn mirror_shape(shape: &str) -> &str {
    match shape {
        "inner_left" => "inner_right",
        "inner_right" => "inner_left",
        "outer_left" => "outer_right",
        "outer_right" => "outer_left",
        other => other,
    }
}

const HORIZONTALS: [&str; 4] = ["north", "east", "south", "west"];

/// Rail shapes name the directions a rail connects (`north_south`,
/// `south_east`, `ascending_west`, ...). Maps every direction token and
/// restores the canonical spelling. Other `shape` values (stairs) pass through.
fn map_rail_shape(shape: &str, map: impl Fn(&str) -> String) -> String {
    if let Some(dir) = shape.strip_prefix("ascending_") {
        return format!("ascending_{}", map(dir));
    }
    let Some((a, b)) = shape.split_once('_') else {
        return shape.to_string();
    };
    if !HORIZONTALS.contains(&a) || !HORIZONTALS.contains(&b) {
        return shape.to_string();
    }
    let (a, b) = (map(a), map(b));
    // Canonical order: north/south before east/west; "north_south", "east_west".
    let rank = |d: &str| match d {
        "north" => 0,
        "south" => 1,
        "east" => 2,
        _ => 3,
    };
    let (first, second) = if rank(&a) <= rank(&b) { (a, b) } else { (b, a) };
    format!("{first}_{second}")
}

/// Applies a horizontal direction mapping to every orientation property:
/// `facing`, the `north`/`east`/`south`/`west` connection flags of fences,
/// walls, panes, redstone wire and similar blocks, and rail shapes.
fn map_directions(
    state: &BlockState,
    map: impl Fn(&str) -> String,
    mut other: impl FnMut(&BlockProperty) -> BlockProperty,
) -> BlockState {
    let props: Vec<BlockProperty> = state
        .properties()
        .map(|prop| {
            let name = prop.name();
            let val = prop.value();
            if name == "facing" {
                BlockProperty::new(name, map(val))
            } else if HORIZONTALS.contains(&name) {
                BlockProperty::new(map(name), val)
            } else if name == "shape" {
                BlockProperty::new(name, map_rail_shape(val, &map))
            } else {
                other(prop)
            }
        })
        .collect();
    BlockState::new(state.id(), props).unwrap_or_else(|_| state.clone())
}

pub fn rotate_block_state(state: &BlockState, angle: RotationAngle) -> BlockState {
    map_directions(
        state,
        |dir| rotate_facing(dir, angle).to_string(),
        |prop| match prop.name() {
            "axis" => BlockProperty::new("axis", rotate_axis(prop.value(), angle)),
            "rotation" => match prop.value().parse::<u32>() {
                Ok(rot) => {
                    let delta = match angle {
                        RotationAngle::Deg90 => 4,
                        RotationAngle::Deg180 => 8,
                        RotationAngle::Deg270 => 12,
                    };
                    BlockProperty::new("rotation", ((rot + delta) % 16).to_string())
                }
                Err(_) => prop.clone(),
            },
            _ => prop.clone(),
        },
    )
}

pub fn mirror_block_state(state: &BlockState, axis: MirrorAxis) -> BlockState {
    // A mirror swaps handedness: stair corners, door hinges, double chests.
    let swap_side = |value: &str| match value {
        "left" => "right".to_string(),
        "right" => "left".to_string(),
        other => other.to_string(),
    };
    let mirrored = map_directions(
        state,
        |dir| mirror_facing(dir, axis).to_string(),
        |prop| match prop.name() {
            "rotation" => match prop.value().parse::<u32>() {
                Ok(rot) => {
                    let new_rot = match axis {
                        MirrorAxis::X => (16 - rot) % 16,
                        MirrorAxis::Z => (24 - rot) % 16,
                    };
                    BlockProperty::new("rotation", new_rot.to_string())
                }
                Err(_) => prop.clone(),
            },
            "hinge" => BlockProperty::new("hinge", swap_side(prop.value())),
            // Chest halves; slabs use "type" too but with top/bottom/double.
            "type" => BlockProperty::new("type", swap_side(prop.value())),
            _ => prop.clone(),
        },
    );
    // Stair shapes (inner_left, outer_right, ...) flip handedness as well.
    let props: Vec<BlockProperty> = mirrored
        .properties()
        .map(|prop| {
            if prop.name() == "shape" {
                BlockProperty::new("shape", mirror_shape(prop.value()))
            } else {
                prop.clone()
            }
        })
        .collect();
    BlockState::new(mirrored.id(), props).unwrap_or(mirrored)
}

pub fn rotate_relative_coords(
    rel_x: i64,
    rel_z: i64,
    size_x: i64,
    size_z: i64,
    angle: RotationAngle,
) -> (i64, i64) {
    match angle {
        RotationAngle::Deg90 => (size_z - 1 - rel_z, rel_x),
        RotationAngle::Deg180 => (size_x - 1 - rel_x, size_z - 1 - rel_z),
        RotationAngle::Deg270 => (rel_z, size_x - 1 - rel_x),
    }
}

pub fn mirror_relative_coords(
    rel_x: i64,
    rel_z: i64,
    size_x: i64,
    size_z: i64,
    axis: MirrorAxis,
) -> (i64, i64) {
    match axis {
        MirrorAxis::X => (size_x - 1 - rel_x, rel_z),
        MirrorAxis::Z => (rel_x, size_z - 1 - rel_z),
    }
}

/// Continuous counterpart of [`rotate_relative_coords`] for entity positions
/// (a block cell `[x, x+1)` maps onto the same cell the block itself maps to).
pub fn rotate_relative_point(
    rel_x: f64,
    rel_z: f64,
    size_x: f64,
    size_z: f64,
    angle: RotationAngle,
) -> (f64, f64) {
    match angle {
        RotationAngle::Deg90 => (size_z - rel_z, rel_x),
        RotationAngle::Deg180 => (size_x - rel_x, size_z - rel_z),
        RotationAngle::Deg270 => (rel_z, size_x - rel_x),
    }
}

/// Continuous counterpart of [`mirror_relative_coords`].
pub fn mirror_relative_point(
    rel_x: f64,
    rel_z: f64,
    size_x: f64,
    size_z: f64,
    axis: MirrorAxis,
) -> (f64, f64) {
    match axis {
        MirrorAxis::X => (size_x - rel_x, rel_z),
        MirrorAxis::Z => (rel_x, size_z - rel_z),
    }
}

const FACING_2D: [&str; 4] = ["south", "west", "north", "east"];
const FACING_3D: [&str; 6] = ["down", "up", "north", "south", "west", "east"];

fn is_3d_facing_entity(kind: &str) -> bool {
    matches!(
        kind.trim_start_matches("minecraft:"),
        "item_frame" | "glow_item_frame"
    )
}

fn map_facing_tag(
    kind: &str,
    data: &mut schematic_core::NbtCompound,
    map: impl Fn(&str) -> String,
) {
    use schematic_core::NbtTag;
    let table: &[&str] = if is_3d_facing_entity(kind) {
        &FACING_3D
    } else {
        &FACING_2D
    };
    for key in ["Facing", "facing"] {
        if let Some(tag) = data.get_mut(key) {
            let Some(current) = tag.as_i32() else {
                continue;
            };
            let Some(name) = table.get(current as usize) else {
                continue;
            };
            let mapped = map(name);
            if let Some(new_index) = table.iter().position(|n| *n == mapped) {
                *tag = match tag {
                    NbtTag::Byte(_) => NbtTag::Byte(new_index as i8),
                    NbtTag::Short(_) => NbtTag::Short(new_index as i16),
                    _ => NbtTag::Int(new_index as i32),
                };
            }
        }
    }
}

fn map_yaw(data: &mut schematic_core::NbtCompound, map: impl Fn(f64) -> f64) {
    use schematic_core::NbtTag;
    if let Some(NbtTag::List(_, values)) = data.get_mut("Rotation") {
        if let Some(first) = values.first_mut() {
            let yaw = first.as_f64().unwrap_or(0.0);
            let new_yaw = map(yaw).rem_euclid(360.0);
            *first = match first {
                NbtTag::Double(_) => NbtTag::Double(new_yaw),
                _ => NbtTag::Float(new_yaw as f32),
            };
        }
    }
}

/// Rotates an entity's orientation data (`Rotation` yaw and hanging-entity
/// `Facing`) consistently with [`rotate_block_state`].
pub fn rotate_entity_data(
    kind: &str,
    data: &mut schematic_core::NbtCompound,
    angle: RotationAngle,
) {
    let degrees = match angle {
        RotationAngle::Deg90 => 90.0,
        RotationAngle::Deg180 => 180.0,
        RotationAngle::Deg270 => 270.0,
    };
    map_yaw(data, |yaw| yaw + degrees);
    map_facing_tag(kind, data, |name| rotate_facing(name, angle).to_string());
}

/// Mirrors an entity's orientation data consistently with [`mirror_block_state`].
pub fn mirror_entity_data(kind: &str, data: &mut schematic_core::NbtCompound, axis: MirrorAxis) {
    match axis {
        MirrorAxis::X => map_yaw(data, |yaw| -yaw),
        MirrorAxis::Z => map_yaw(data, |yaw| 180.0 - yaw),
    }
    map_facing_tag(kind, data, |name| mirror_facing(name, axis).to_string());
}
