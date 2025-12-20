//! Dwarf Fortress calendar
use clap::ValueEnum;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    ops::{Add, Sub},
};
use strum::{Display, EnumIter};

#[derive(
    Clone,
    Copy,
    Display,
    IntoPrimitive,
    TryFromPrimitive,
    Serialize,
    Deserialize,
    PartialEq,
    EnumIter,
    ValueEnum,
)]
#[repr(i32)]
pub enum Month {
    Granite,
    Slate,
    Felsite,
    Hematite,
    Malachite,
    Galena,
    Limestone,
    Sandstone,
    Timber,
    Moonstone,
    Opal,
    Obsidian,
}

impl Month {
    pub fn year_tick(self) -> i32 {
        let index: i32 = self.into();
        index * 33600
    }
}

impl Add<i32> for Month {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        let index: i32 = self.into();
        Self::try_from((index + rhs).rem_euclid(12)).unwrap()
    }
}

impl Sub<i32> for Month {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        let index: i32 = self.into();
        Self::try_from((index - rhs).rem_euclid(12)).unwrap()
    }
}

#[derive(Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimeOfTheYear {
    #[default]
    Current,
    Month(Month),
}

impl TimeOfTheYear {
    pub fn ticks(&self, df: &mut dfhack_remote::Client) -> i32 {
        match self {
            TimeOfTheYear::Current => df
                .remote_fortress_reader()
                .get_world_map()
                .map(|wm| wm.cur_year_tick())
                .unwrap_or_default(),
            TimeOfTheYear::Month(month) => month.year_tick(),
        }
    }
}

impl Display for TimeOfTheYear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeOfTheYear::Current => f.write_str("Current"),
            /*TimeOfTheYear::Tick(tick) => {
                for (month, next_month) in Month::iter().circular_tuple_windows() {
                    if (month.year_tick()..=next_month.year_tick()).contains(tick) {
                        return f.write_fmt(format_args!("~{}", month));
                    }
                }
                f.write_str("?")
            }*/
            TimeOfTheYear::Month(month) => month.fmt(f),
        }
    }
}

impl Add<i32> for TimeOfTheYear {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        let month = match self {
            TimeOfTheYear::Current => Month::Granite,
            TimeOfTheYear::Month(month) => month,
        };
        Self::Month(month + rhs)
    }
}

impl Sub<i32> for TimeOfTheYear {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        let month = match self {
            TimeOfTheYear::Current => Month::Granite,
            TimeOfTheYear::Month(month) => month,
        };
        Self::Month(month - rhs)
    }
}

impl Month {
    #[cfg(feature = "gui")]
    pub fn gui_color(&self) -> eframe::egui::Color32 {
        use eframe::egui;
        match self {
            Month::Granite | Month::Slate | Month::Felsite => egui::Color32::GREEN,
            Month::Hematite | Month::Malachite | Month::Galena => egui::Color32::YELLOW,
            Month::Limestone | Month::Sandstone | Month::Timber => egui::Color32::RED,
            Month::Moonstone | Month::Opal | Month::Obsidian => egui::Color32::BLUE,
        }
    }
}
