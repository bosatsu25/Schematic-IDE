use schematic_core::BlockState;

pub trait OccupancyPolicy {
    fn is_occupied(&self, state: &BlockState) -> bool;
}

impl<F> OccupancyPolicy for F
where
    F: Fn(&BlockState) -> bool,
{
    fn is_occupied(&self, state: &BlockState) -> bool {
        self(state)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NonAirPolicy;

impl OccupancyPolicy for NonAirPolicy {
    fn is_occupied(&self, state: &BlockState) -> bool {
        let id = state.id();
        id != "minecraft:air" && id != "minecraft:cave_air" && id != "minecraft:void_air"
    }
}
