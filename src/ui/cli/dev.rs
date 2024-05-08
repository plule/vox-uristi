use std::path::{Path, PathBuf};

use anyhow::Result;
use dfhack_remote::{BasicMaterialInfoMask, BlockRequest, ListMaterialsIn};
use protobuf::{Message, MessageDyn, MessageField};

use crate::{rfr, rfr::DFHackExt, DFCoords, DevCommand};

pub fn run(cmd: DevCommand) -> Result<(), anyhow::Error> {
    match cmd {
        DevCommand::DumpLists { destination } => dump_lists(destination),
        DevCommand::Probe { destination } => probe(destination),
        DevCommand::RegenTestData => regen_test_data(),
        DevCommand::SetElevation { elevation } => set_elevation(elevation),
    }
}

pub fn probe(destination: PathBuf) -> Result<(), anyhow::Error> {
    let mut client = dfhack_remote::connect()?;
    let view_info = client.remote_fortress_reader().get_view_info()?;
    let x = view_info.cursor_pos_x();
    let y = view_info.cursor_pos_y();
    let z = view_info.cursor_pos_z();
    let tile_type_list = client.remote_fortress_reader().get_tiletype_list()?;
    let probe = DFCoords::new(x, y, z);
    for block_list in rfr::BlockListIterator::try_new(&mut client, 100, 0..1000, 0..1000, z..z + 1)?
    {
        for block in block_list?.map_blocks {
            for tile in rfr::TileIterator::new(&block, &tile_type_list) {
                if tile.coords() == probe {
                    println!("{}", tile);
                }
            }

            for (i, building) in block.buildings.into_iter().enumerate() {
                let bx = building.pos_x_min()..=building.pos_x_max();
                let by = building.pos_y_min()..=building.pos_y_max();
                let bz = building.pos_z_min()..=building.pos_z_max();
                if building.room.is_none() && bx.contains(&x) && by.contains(&y) && bz.contains(&z)
                {
                    dump(
                        &building,
                        &destination,
                        format!("building_{i}.json").as_str(),
                    )?;
                }
            }
            for (i, flow) in block.flows.iter().enumerate() {
                if DFCoords::from(flow.pos.get_or_default()) == probe {
                    dump(flow, &destination, format!("flow_{i}.json").as_str())?;
                }
            }
        }
    }

    Ok(())
}

fn regen_test_data() -> Result<(), anyhow::Error> {
    let destination = PathBuf::from("testdata");
    let mut client = dfhack_remote::connect()?;
    client.remote_fortress_reader().reset_map_hashes()?;
    let view_info = client.remote_fortress_reader().get_view_info()?;
    let z = view_info.cursor_pos_z();
    for (index, block_list) in
        rfr::BlockListIterator::try_new(&mut client, 100, 0..1000, 0..1000, z..z + 1)?.enumerate()
    {
        let data = block_list?.write_to_bytes()?;
        let mut dest = destination.clone();
        dest.push(format!("block_{index}.dat"));
        println!("{}", &dest.display());
        std::fs::write(dest, data)?;
    }

    let building_defs = client.remote_fortress_reader().get_building_def_list()?;
    let data = building_defs.write_to_bytes()?;
    let mut dest = destination.clone();
    dest.push("building_defs.dat");
    println!("{}", &dest.display());
    std::fs::write(dest, data)?;

    Ok(())
}

fn dump_lists(destination: PathBuf) -> Result<()> {
    let mut client = dfhack_remote::connect()?;

    let req = ListMaterialsIn {
        mask: MessageField::some(BasicMaterialInfoMask {
            flags: Some(true),
            reaction: Some(true),
            ..Default::default()
        }),
        inorganic: Some(true),
        builtin: Some(true),
        ..Default::default()
    };

    let basic_materials = client.core().list_materials(req)?;
    dump(&basic_materials, &destination, "basic_materials.json")?;

    let materials = client.remote_fortress_reader().get_material_list()?;
    dump(&materials, &destination, "materials.json")?;

    let plants = client.remote_fortress_reader().get_plant_raws()?;
    dump(&plants, &destination, "plant_raws.json")?;

    let ttypes = client.remote_fortress_reader().get_tiletype_list()?;
    dump(&ttypes, &destination, "tiletypes.json")?;

    let building_defs = client.remote_fortress_reader().get_building_def_list()?;
    dump(&building_defs, &destination, "building_defs.json")?;

    let growth_list = client.remote_fortress_reader().get_growth_list()?;
    dump(&growth_list, &destination, "growths.json")?;

    let item_list = client.remote_fortress_reader().get_item_list()?;
    dump(&item_list, &destination, "items.json")?;

    let language = client.remote_fortress_reader().get_language()?;
    dump(&language, &destination, "language.json")?;

    let view_info = client.remote_fortress_reader().get_view_info()?;
    client.remote_fortress_reader().reset_map_hashes()?;
    let z = view_info.cursor_pos_z();
    let req = BlockRequest {
        blocks_needed: Some(1),
        min_x: Some(0),
        max_x: Some(1000),
        min_y: Some(0),
        max_y: Some(1000),
        min_z: Some(z),
        max_z: Some(z + 1),
        ..Default::default()
    };
    let blocks = client.remote_fortress_reader().get_block_list(req)?;
    dump(&blocks, &destination, "blocks.json")?;

    let enums = client.core().list_enums()?;
    dump(&enums, &destination, "enums.json")?;

    Ok(())
}

fn dump(message: &dyn MessageDyn, folder: &Path, filename: &str) -> Result<()> {
    let json = protobuf_json_mapping::print_to_string(message)?;
    let mut dest = folder.to_path_buf();
    dest.push(filename);
    println!("{}", &dest.display());
    std::fs::write(dest, json)?;
    Ok(())
}

pub fn set_elevation(elevation: i32) -> Result<(), anyhow::Error> {
    let mut client = dfhack_remote::connect()?;
    client.set_elevation(elevation)?;
    Ok(())
}
