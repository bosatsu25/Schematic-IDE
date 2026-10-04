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

pub fn rotate_block_state(state: &BlockState, angle: RotationAngle) -> BlockState {
    let mut props = Vec::new();
    for prop in state.properties() {
        let name = prop.name();
        let val = prop.value();
        match name {
            "facing" => {
                let new_val = rotate_facing(val, angle);
                props.push(BlockProperty::new(name, new_val));
            }
            "axis" => {
                let new_val = rotate_axis(val, angle);
                props.push(BlockProperty::new(name, new_val));
            }
            "rotation" => {
                if let Ok(rot) = val.parse::<u32>() {
                    let delta = match angle {
                        RotationAngle::Deg90 => 4,
                        RotationAngle::Deg180 => 8,
                        RotationAngle::Deg270 => 12,
                    };
                    let new_rot = (rot + delta) % 16;
                    props.push(BlockProperty::new(name, new_rot.to_string()));
                } else {
                    props.push(prop.clone());
                }
            }
            _ => {
                props.push(prop.clone());
            }
        }
    }
    BlockState::new(state.id(), props).unwrap_or_else(|_| state.clone())
}

pub fn mirror_block_state(state: &BlockState, axis: MirrorAxis) -> BlockState {
    let mut props = Vec::new();
    for prop in state.properties() {
        let name = prop.name();
        let val = prop.value();
        match name {
            "facing" => {
                let new_val = mirror_facing(val, axis);
                props.push(BlockProperty::new(name, new_val));
            }
            "shape" => {
                let new_val = mirror_shape(val);
                props.push(BlockProperty::new(name, new_val));
            }
            "rotation" => {
                if let Ok(rot) = val.parse::<u32>() {
                    let new_rot = match axis {
                        MirrorAxis::X => (16 - rot) % 16,
                        MirrorAxis::Z => (24 - rot) % 16,
                    };
                    props.push(BlockProperty::new(name, new_rot.to_string()));
                } else {
                    props.push(prop.clone());
                }
            }
            _ => {
                props.push(prop.clone());
            }
        }
    }
    BlockState::new(state.id(), props).unwrap_or_else(|_| state.clone())
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
