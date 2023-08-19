mod collect;
mod generic;
mod tree;
use crate::{rfr::BlockTile, WithDFCoords};
pub use generic::BlockTileExt;
pub use tree::BlockTilePlantExt;

impl WithDFCoords for BlockTile<'_> {
    fn coords(&self) -> crate::DFCoords {
        self.coords()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampContactKind {
    Wall,
    Ramp,
    Empty,
}

impl RampContactKind {
    fn height(&self) -> usize {
        match self {
            RampContactKind::Wall => 5,
            RampContactKind::Ramp => 3,
            RampContactKind::Empty => 1,
        }
    }
}

fn corner_ramp_level(c1: RampContactKind, c2: RampContactKind) -> usize {
    match (c1, c2) {
        (RampContactKind::Ramp, RampContactKind::Ramp) => 3, // should be 1 for concave, 5 for convexe todo
        (RampContactKind::Ramp, c) | (c, RampContactKind::Ramp) => c.height(),
        (c1, c2) => (c1.height() + c2.height()) / 2,
    }
}
