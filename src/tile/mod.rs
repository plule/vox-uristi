mod collect;
mod extensions;
mod plant;
pub use extensions::TileExtensions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RampContactKind {
    Wall,
    Ramp,
    Empty,
}

impl RampContactKind {
    fn height(&self) -> usize {
        match self {
            RampContactKind::Wall => 3,
            RampContactKind::Ramp => 2,
            RampContactKind::Empty => 1,
        }
    }
}

fn corner_ramp_level(c1: RampContactKind, c2: RampContactKind) -> usize {
    match (c1, c2) {
        (RampContactKind::Ramp, RampContactKind::Ramp) => 2, // should be 1 for concave, 3 for convexe todo
        (RampContactKind::Ramp, c) | (c, RampContactKind::Ramp) => c.height(),
        (c1, c2) => (c1.height() + c2.height()) / 2,
    }
}
