use super::BuildingInstanceExt;
use crate::{
    building::BuildingType,
    direction::DirectionFlat,
    map::Map,
    shape::{self, Box3D, Rotating},
    tile::BlockTileExt,
    IsSomeAnd,
};
use dfhack_remote::BuildingInstance;
use easy_ext::ext;

#[ext(BuildingInstanceFurnitureExt)]
pub impl BuildingInstance {
    fn window_shape(&self, map: &Map) -> Box3D<bool> {
        let conn = map.neighbouring_flat(self.origin(), |tile, buildings| {
            buildings.iter().any(|b| {
                matches!(
                    b.building_type(),
                    BuildingType::WindowGem | BuildingType::WindowGlass
                )
            }) || tile.some_and(|tile| tile.is_wall())
        });
        [
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            shape::slice_empty(),
        ]
    }

    fn door_shape(&self, map: &Map) -> Box3D<bool> {
        let conn = map.neighbouring_flat(self.origin(), |tile, buildings| {
            buildings
                .iter()
                .any(|b| matches!(b.building_type(), BuildingType::Door))
                || tile.some_and(|t| t.is_wall())
        });
        [
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            [
                [false, conn.n, false],
                [conn.w, true, conn.e],
                [false, conn.s, false],
            ],
            shape::slice_empty(),
        ]
    }

    fn archery_shape(direction: DirectionFlat) -> Box3D<bool> {
        [
            shape::slice_empty(),
            [
                [true, true, true],
                [false, true, false],
                [false, false, false],
            ],
            [
                [true, true, true],
                [false, true, false],
                [false, true, false],
            ],
            [
                [true, true, true],
                [false, true, false],
                [false, true, false],
            ],
            shape::slice_empty(),
        ]
        .looking_at(direction)
    }
}