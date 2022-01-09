use serde_derive::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::ships::ShipDb;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipClass {
    Destroyer,
    Cruiser,
    Battleship,
    AircraftCarrier,
}

impl ShipClass {
    pub fn short(&self) -> &'static str {
        match self {
            Self::Destroyer => "DD",
            Self::Cruiser => "CR",
            Self::Battleship => "BB",
            Self::AircraftCarrier => "CV",
        }
    }
}

impl std::cmp::PartialOrd for ShipClass {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }
        Some(match (self, other) {
            (Self::Destroyer, _)
            | (Self::Cruiser, Self::Battleship)
            | (Self::Cruiser, Self::AircraftCarrier)
            | (Self::Battleship, Self::AircraftCarrier) => Ordering::Less,
            _ => Ordering::Greater,
        })
    }
}

impl std::cmp::Ord for ShipClass {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl std::convert::From<String> for ShipClass {
    fn from(s: String) -> Self {
        if s == "Cruiser" {
            Self::Cruiser
        } else if s == "Destroyer" {
            Self::Destroyer
        } else if s == "Battleship" {
            Self::Battleship
        } else if s == "AirCarrier" {
            Self::AircraftCarrier
        } else {
            panic!("Unknown ship type {}", s);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ship {
    pub name: String,
    pub id: u64,
    pub tier: u64,
    pub profile_url: String,
    pub class: ShipClass,
    pub speed: f32,
    pub torpedoes: Option<f32>,
    pub hydro: Option<f32>,
    pub radar: Option<f32>,
    pub min_tier: u16,
    pub max_tier: u16,
    pub nation: String,
}

impl Ship {
    fn from(
        shipinfo: &crate::wows_data::ShipInfo,
        params: &crate::gameparams::ProcessedShip,
        modules: &HashMap<u64, crate::wows_data::DetailedModuleInfo>,
    ) -> Self {
        let mut hydro = None;
        let mut radar = None;
        for (slotid, slot) in params.ability_slots.iter().enumerate() {
            for ability in slot.options.iter() {
                //println!("{} type is {}", slotid, ability.consumable_type);
                if ability.consumable_type == "sonar" {
                    hydro = Some(ability.dist_ship.unwrap() / 33.333333);
                } else if ability.consumable_type == "rls" {
                    radar = Some(ability.dist_ship.unwrap() / 33.333333);
                }
            }
        }
        let mut torpedoes = None;
        for (_, module_info) in shipinfo.modules_tree.iter() {
            let module = modules.get(&module_info.module_id).expect(&format!(
                "Couldn't find module {} specified in ship",
                module_info.module_id
            ));
            if let Some(torpedo_spec) = &module.profile.torpedoes {
                torpedoes = Some(
                    torpedoes
                        .map(|x: f32| x.max(torpedo_spec.distance))
                        .unwrap_or(torpedo_spec.distance),
                );
            }
        }
        Ship {
            name: shipinfo.name.clone(),
            id: shipinfo.ship_id,
            tier: shipinfo.tier,
            profile_url: shipinfo
                .images
                .get("contour")
                .expect("Couldn't find contour image")
                .clone(),
            class: shipinfo.ship_type.clone().into(),
            speed: shipinfo
                .default_profile
                .mobility
                .as_ref()
                .map(|x| x.max_speed)
                .unwrap_or(0.0),
            torpedoes: torpedoes,
            hydro: hydro,
            radar: radar,
            min_tier: shipinfo.default_profile.battle_level_range_min.unwrap_or(0),
            max_tier: shipinfo.default_profile.battle_level_range_max.unwrap_or(0),
            nation: shipinfo.nation.clone(),
        }
    }
}

//#[derive(Serialize, Deserialize)]
pub struct CheatsheetDb {
    //pub ships: HashMap<u64, Ship>,
    shipdb: ShipDb,
    gameparams: crate::gameparams::GameParams,
}

impl CheatsheetDb {
    pub fn from(shipdb: ShipDb, gameparams: crate::gameparams::GameParams) -> Self {
        /*let mut ships = HashMap::new();
        for (id, ship) in shipinfos.iter() {
            let param = gameparams
                .get_ship(*id)
                .expect("Couldn't find ship gameparam");
            let ship: Ship = Ship::from(ship, param, modules);
            ships.insert(*id, ship);
        }*/
        CheatsheetDb { shipdb, gameparams }
    }

    pub fn enumerate_ships(&self) -> Vec<u64> {
        self.shipdb.enumerate_ships()
    }

    pub fn get_ship(&self, id: u64) -> Option<Ship> {
        //self.ships.get(&id)
        let param = self.gameparams.get_ship(id).unwrap();
        let shipinfo = self.shipdb.get_ship_info(id).unwrap();
        let modules = self.shipdb.get_modules();
        Some(Ship::from(&shipinfo, param, &modules))
    }
}
