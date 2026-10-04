use crate::{Bounds, Position};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionBox {
    bounds: Bounds,
}

impl SelectionBox {
    pub const fn new(bounds: Bounds) -> Self {
        Self { bounds }
    }

    pub const fn bounds(self) -> Bounds {
        self.bounds
    }

    pub fn contains(self, position: Position) -> bool {
        self.bounds.contains(position)
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        self.bounds.intersection(other.bounds).map(Self::new)
    }
}

impl From<Bounds> for SelectionBox {
    fn from(bounds: Bounds) -> Self {
        Self::new(bounds)
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Selection {
    boxes: Vec<SelectionBox>,
}

impl Selection {
    pub fn empty() -> Self {
        Self { boxes: Vec::new() }
    }

    pub fn from_corners(first: Position, second: Position) -> Self {
        let bounds = Bounds::from_inclusive(first, second);
        Self {
            boxes: vec![SelectionBox::new(bounds)],
        }
    }

    pub fn from_bounds(bounds: Bounds) -> Self {
        if bounds.is_empty() {
            Self::empty()
        } else {
            Self {
                boxes: vec![SelectionBox::new(bounds)],
            }
        }
    }

    pub fn from_boxes(boxes: impl IntoIterator<Item = SelectionBox>) -> Self {
        let mut set = BTreeSet::new();
        for b in boxes {
            if !b.bounds().is_empty() {
                set.insert(b);
            }
        }
        Self {
            boxes: set.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.boxes.is_empty()
    }

    pub fn boxes(&self) -> &[SelectionBox] {
        &self.boxes
    }

    pub fn bounds(&self) -> Bounds {
        if self.boxes.is_empty() {
            return Bounds::new(Position::default(), [0, 0, 0]);
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut min_z = i32::MAX;
        let mut max_x = i64::MIN;
        let mut max_y = i64::MIN;
        let mut max_z = i64::MIN;

        for b in &self.boxes {
            let m = b.bounds().min();
            let mx = b.bounds().max_exclusive();
            min_x = min_x.min(m.x);
            min_y = min_y.min(m.y);
            min_z = min_z.min(m.z);
            max_x = max_x.max(mx[0]);
            max_y = max_y.max(mx[1]);
            max_z = max_z.max(mx[2]);
        }
        Bounds::new(Position::new(min_x, min_y, min_z), [max_x, max_y, max_z])
    }

    pub fn contains(&self, position: Position) -> bool {
        self.boxes.iter().any(|b| b.contains(position))
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let mut intersections = BTreeSet::new();
        for b1 in &self.boxes {
            for b2 in &other.boxes {
                if let Some(inter) = b1.intersection(*b2) {
                    intersections.insert(inter);
                }
            }
        }
        if intersections.is_empty() {
            None
        } else {
            Some(Self {
                boxes: intersections.into_iter().collect(),
            })
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PlacementTarget {
    regions: Vec<Bounds>,
}

impl PlacementTarget {
    pub fn new(regions: impl IntoIterator<Item = Bounds>) -> Self {
        let mut set = BTreeSet::new();
        for r in regions {
            if !r.is_empty() {
                set.insert(r);
            }
        }
        Self {
            regions: set.into_iter().collect(),
        }
    }

    pub fn regions(&self) -> &[Bounds] {
        &self.regions
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct OperationTarget {
    selection: Selection,
    placement: Option<PlacementTarget>,
}

impl OperationTarget {
    pub fn new(selection: Selection, placement: Option<PlacementTarget>) -> Self {
        Self {
            selection,
            placement,
        }
    }

    pub fn empty() -> Self {
        Self {
            selection: Selection::empty(),
            placement: None,
        }
    }

    pub fn unconstrained(selection: Selection) -> Self {
        let boxes = selection.boxes().iter().map(|b| b.bounds());
        let placement = PlacementTarget::new(boxes);
        Self {
            selection,
            placement: Some(placement),
        }
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn placement(&self) -> Option<&PlacementTarget> {
        self.placement.as_ref()
    }

    pub fn world_regions(&self) -> Vec<Bounds> {
        let Some(placement) = &self.placement else {
            return Vec::new();
        };
        let mut set = BTreeSet::new();
        for box_item in self.selection.boxes() {
            for region in placement.regions() {
                if let Some(inter) = box_item.bounds().intersection(*region) {
                    set.insert(inter);
                }
            }
        }
        set.into_iter().collect()
    }
}

pub struct WorldBoundsMapping;

impl WorldBoundsMapping {
    pub fn from_inclusive(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> Bounds {
        Bounds::from_inclusive(Position::new(x1, y1, z1), Position::new(x2, y2, z2))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn require_safe_transform(
        ox: i32,
        oy: i32,
        oz: i32,
        px: i32,
        py: i32,
        pz: i32,
        sx: i32,
        sy: i32,
        sz: i32,
    ) -> Result<(), &'static str> {
        if sx == 0 || sy == 0 || sz == 0 {
            return Err("placement subregion has zero size");
        }
        let max_magnitude = |x: i32, y: i32, z: i32| -> i64 {
            (x as i64).abs().max((y as i64).abs()).max((z as i64).abs())
        };
        let origin = max_magnitude(ox, oy, oz);
        let offset = max_magnitude(px, py, pz);
        let extent = max_magnitude(sx, sy, sz) - 1;
        if origin.saturating_add(offset).saturating_add(extent) > i32::MAX as i64 {
            return Err("placement transform exceeds safe integer bounds");
        }
        Ok(())
    }
}
