use eframe::egui;
use itertools::Itertools;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Clone, Copy, Display, IntoPrimitive, Serialize, Deserialize, PartialEq, EnumIter)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
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

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimeOfTheYear {
    Tick(i32),
    Month(Month),
}

impl TimeOfTheYear {
    pub fn ticks(&self) -> i32 {
        match self {
            TimeOfTheYear::Tick(tick) => *tick,
            TimeOfTheYear::Month(month) => month.year_tick(),
        }
    }
}

impl Display for TimeOfTheYear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeOfTheYear::Tick(tick) => {
                for (month, next_month) in Month::iter().circular_tuple_windows() {
                    if (month.year_tick()..=next_month.year_tick()).contains(tick) {
                        return f.write_fmt(format_args!("~{}", month));
                    }
                }
                f.write_str("?")
            }
            TimeOfTheYear::Month(month) => month.fmt(f),
        }
    }
}

impl Default for TimeOfTheYear {
    fn default() -> Self {
        Self::Tick(0)
    }
}

impl Month {
    pub fn color(&self) -> egui::Color32 {
        match self {
            Month::Granite | Month::Slate | Month::Felsite => egui::Color32::GREEN,
            Month::Hematite | Month::Malachite | Month::Galena => egui::Color32::YELLOW,
            Month::Limestone | Month::Sandstone | Month::Timber => egui::Color32::RED,
            Month::Moonstone | Month::Opal | Month::Obsidian => egui::Color32::BLUE,
        }
    }
}
