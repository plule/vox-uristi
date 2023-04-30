use dfhack_remote::BuildingInstance;

use crate::direction::DirectionFlat;

#[derive(Debug, Eq, PartialEq)]
pub enum BuildingType {
    Chair,
    Bed,
    Table,
    Coffin,
    FarmPlot,
    Furnace { subtype: i32 },
    Door,
    Floodgate,
    Box,
    WeaponRack,
    ArmorStand,
    WindowGlass,
    WindowGem,
    Well,
    Cabinet,
    Statue,
    Workshop { subtype: i32 },
    Bridge { direction: Option<DirectionFlat> },
    RoadDirt,
    RoadPaved,
    SiegeEngine { subtype: i32 },
    Trap { subtype: i32 },
    AnimalTrap,
    Support,
    ArcheryTarget { direction: DirectionFlat },
    Chain,
    Cage,
    Stockpile,
    Civzone { subtype: i32 },
    Weapon,
    Wagon,
    ScrewPump,
    Construction { subtype: i32 },
    Hatch,
    GrateWall,
    GrateFloor,
    BarsVertical,
    BarsFloor,
    GearAssembly,
    AxleHorizontal,
    AxleVertical,
    WaterWheel,
    Windmill,
    TractionBench,
    Slab,
    Nest,
    NestBox,
    Hive,
    Rollers,
    Instrument,
    Bookcase,
    DisplayFurniture,
    OfferingPlace,
}

impl BuildingType {
    pub fn maybe_from_df(instance: &BuildingInstance) -> Option<BuildingType> {
        if instance.building_type.is_none() {
            return None;
        }

        let building_type = instance.building_type.get_or_default();
        let t = match building_type.building_type() {
            0 => BuildingType::Chair,
            1 => BuildingType::Bed,
            2 => BuildingType::Table,
            3 => BuildingType::Coffin,
            4 => BuildingType::FarmPlot,
            5 => BuildingType::Furnace {
                subtype: building_type.building_subtype(),
            },
            8 => BuildingType::Door,
            9 => BuildingType::Floodgate,
            10 => BuildingType::Box,
            11 => BuildingType::WeaponRack,
            12 => BuildingType::ArmorStand,
            13 => BuildingType::Workshop {
                subtype: building_type.building_subtype(),
            },
            14 => BuildingType::Cabinet,
            15 => BuildingType::Statue,
            16 => BuildingType::WindowGlass,
            17 => BuildingType::WindowGem,
            18 => BuildingType::Well,
            19 => BuildingType::Bridge {
                direction: DirectionFlat::maybe_from_df(&instance.direction()),
            },
            20 => BuildingType::RoadDirt,
            21 => BuildingType::RoadPaved,
            22 => BuildingType::SiegeEngine {
                subtype: building_type.building_subtype(),
            },
            23 => BuildingType::Trap {
                subtype: building_type.building_subtype(),
            },
            24 => BuildingType::AnimalTrap,
            25 => BuildingType::Support,
            26 => BuildingType::ArcheryTarget {
                direction: DirectionFlat::maybe_from_df(&instance.direction())
                    .unwrap_or(DirectionFlat::North),
            },
            27 => BuildingType::Chain,
            28 => BuildingType::Cage,
            29 => BuildingType::Stockpile,
            30 => BuildingType::Civzone {
                subtype: building_type.building_subtype(),
            },
            31 => BuildingType::Weapon,
            32 => BuildingType::Wagon,
            33 => BuildingType::ScrewPump,
            34 => BuildingType::Construction {
                subtype: building_type.building_subtype(),
            },
            35 => BuildingType::Hatch,
            36 => BuildingType::GrateWall,
            37 => BuildingType::GrateFloor,
            38 => BuildingType::BarsVertical,
            39 => BuildingType::BarsFloor,
            40 => BuildingType::GearAssembly,
            41 => BuildingType::AxleHorizontal,
            42 => BuildingType::AxleVertical,
            43 => BuildingType::WaterWheel,
            44 => BuildingType::Windmill,
            45 => BuildingType::TractionBench,
            46 => BuildingType::Slab,
            47 => BuildingType::Nest,
            48 => BuildingType::NestBox,
            49 => BuildingType::Hive,
            50 => BuildingType::Rollers,
            51 => BuildingType::Instrument,
            52 => BuildingType::Bookcase,
            53 => BuildingType::DisplayFurniture,
            54 => BuildingType::OfferingPlace,
            _ => return None,
        };
        Some(t)
    }
}
