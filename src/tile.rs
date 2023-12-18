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
