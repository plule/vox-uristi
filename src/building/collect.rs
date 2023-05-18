use super::BuildingInstanceExt;
use crate::{
    context::DFContext,
    direction::DirectionFlat,
    map::Map,
    shape,
    voxel::{voxels_from_uniform_shape, CollectVoxels, FromPrefab, Voxel},
    Coords, WithCoords,
};
use dfhack_remote::BuildingInstance;

impl CollectVoxels for BuildingInstance {
    fn collect_voxels(&self, map: &Map, context: &DFContext) -> Vec<Voxel> {
        if let Some(building_definition) =
            context.building_definition(self.building_type.get_or_default())
        {
            // Look for a static mesh
            if let Some(prefab) = crate::prefabs::MODELS.building(building_definition.id()) {
                return self.voxels_from_prefab(prefab, map, context);
            }

            // No static mesh, apply a dynamic one
            let coords = self.origin();
            let shape = match building_definition.id() {
                "GrateFloor" | "BarsFloor" => [
                    shape::slice_empty(),
                    shape::slice_empty(),
                    shape::slice_empty(),
                    shape::slice_empty(),
                    shape::slice_from_fn(|x, y| {
                        (coords.x + x as i32) % 2 == 0 || (coords.y + y as i32) % 2 == 0
                    }),
                ],
                "Table" | "TractionBench" => {
                    let edges = map.neighbouring_flat(self.coords(), |_, buildings| {
                        !buildings.iter().any(|building| {
                            matches!(
                                context
                                    .building_definition(building.building_type.get_or_default())
                                    .map(|t| t.id()),
                                Some("Table") | Some("TractionBench")
                            )
                        })
                    });
                    [
                        shape::slice_empty(),
                        shape::slice_empty(),
                        shape::slice_full(),
                        [
                            [edges.n && edges.w, false, edges.n && edges.e],
                            [false, false, false],
                            [edges.s && edges.w, false, edges.s && edges.e],
                        ],
                        shape::slice_empty(),
                    ]
                }
                "Bridge" => {
                    let direction = DirectionFlat::maybe_from_df(&self.direction());
                    let mut voxels = Vec::new();
                    let sn = matches!(direction, Some(DirectionFlat::North | DirectionFlat::South));
                    let ew = matches!(direction, Some(DirectionFlat::East | DirectionFlat::West));
                    let bounding_box = self.bounding_box();
                    for x in bounding_box.x.clone() {
                        for y in bounding_box.y.clone() {
                            let w = sn && x == *bounding_box.x.start();
                            let e = sn && x == *bounding_box.x.end();
                            let n = ew && y == *bounding_box.y.start();
                            let s = ew && y == *bounding_box.y.end();
                            let shape = [
                                shape::slice_empty(),
                                shape::slice_empty(),
                                shape::slice_empty(),
                                [[w || n, n, e || n], [w, false, e], [w || s, s, e || s]],
                                shape::slice_full(),
                            ];
                            let mut shape_voxels = voxels_from_uniform_shape(
                                shape,
                                Coords::new(x, y, self.origin().z),
                                self.material(),
                            );
                            voxels.append(&mut shape_voxels);
                        }
                    }
                    return voxels;
                }
                _ => return vec![],
            };
            return voxels_from_uniform_shape(shape, coords, self.material());
        }
        vec![]
    }
}
