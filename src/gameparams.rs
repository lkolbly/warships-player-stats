use serde_derive::Deserialize;
use serde_json::Result;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Ability {
    #[serde(rename = "consumableType")]
    pub consumable_type: String,

    #[serde(rename = "distShip")]
    pub dist_ship: Option<f32>,

    #[serde(rename = "distTorpedo")]
    pub dist_torpedo: Option<f32>,

    pub group: String,

    #[serde(rename = "numConsumables")]
    pub num_consumables: i32,

    #[serde(rename = "reloadTime")]
    pub reload_time: f32,

    #[serde(rename = "workTime")]
    pub work_time: f32,
}

#[derive(Deserialize)]
struct ShipAbility {
    pub abils: Vec<Vec<String>>,
    pub slot: u32,
}

#[derive(Deserialize)]
struct Ship {
    pub id: u64,
    pub name: String,
    pub level: u32,
    #[serde(rename = "ShipAbilities")]
    pub abilities: Option<HashMap<String, ShipAbility>>,
}

pub struct AbilitySlot {
    pub options: Vec<Ability>,
}

pub struct ProcessedShip {
    pub id: u64,
    pub name: String,
    pub level: u32,
    pub ability_slots: Vec<AbilitySlot>,
}

impl ProcessedShip {
    fn from(ship: Ship, abilities: &HashMap<&String, HashMap<&String, Ability>>) -> Self {
        let abilities: Vec<_> = match ship.abilities {
            Some(x) => x
                .iter()
                .map(|(_, v)| {
                    let options: Vec<_> = v
                        .abils
                        .iter()
                        .filter_map(|raw_abil| {
                            // Note: Some abilities aren't actually defined, such as JP_Carriers_Gold
                            let a = abilities.get(&raw_abil[0])?.get(&raw_abil[1])?.clone();
                            Some(a)
                        })
                        .collect();
                    AbilitySlot { options }
                })
                .collect(),
            None => vec![],
        };
        ProcessedShip {
            id: ship.id,
            name: ship.name,
            level: ship.level,
            ability_slots: abilities,
        }
    }
}

pub struct GameParams {
    ships: HashMap<u64, ProcessedShip>,
}

#[derive(Deserialize)]
struct TypeInfo {
    nation: Option<String>,
    species: Option<String>,

    #[serde(rename = "type")]
    object_type: String,
}

impl GameParams {
    pub fn load(data: &[u8]) -> Result<Self> {
        let data = std::str::from_utf8(data).unwrap();
        let v: Value = serde_json::from_str(data)?;
        let v = v.as_object().unwrap();
        println!("There are {} elements", v.len());
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for (_, entry) in v.iter() {
            let typeinfo = entry
                .as_object()
                .unwrap()
                .get("typeinfo")
                .expect("Couldn't find typeinfo");
            let typeinfo: TypeInfo = serde_json::value::from_value(typeinfo.clone())?;
            if !type_counts.contains_key(&typeinfo.object_type) {
                type_counts.insert(typeinfo.object_type.clone(), 0);
            }
            *type_counts.get_mut(&typeinfo.object_type).unwrap() += 1;
        }
        for (k, v) in type_counts.iter() {
            println!("- {}: {}", k, v);
        }

        // Parse out the abilities
        let mut abilities = HashMap::new();
        for (k, entry) in v.iter() {
            let typeinfo = entry
                .as_object()
                .unwrap()
                .get("typeinfo")
                .expect("Couldn't find typeinfo");
            let typeinfo: TypeInfo = serde_json::value::from_value(typeinfo.clone())?;
            if typeinfo.object_type == "Ability" {
                let entry = entry.as_object().unwrap();
                let mut ability = HashMap::new();
                for (k, v) in entry.iter() {
                    match serde_json::value::from_value(v.clone()) {
                        Ok(parsed_ability) => {
                            ability.insert(k, parsed_ability);
                        }
                        Err(_) => {
                            // Ignore
                        }
                    }
                }
                abilities.insert(k, ability);
            }
        }

        // Parse out the ships
        let mut ships = HashMap::new();
        for (_, entry) in v.iter() {
            let typeinfo = entry
                .as_object()
                .unwrap()
                .get("typeinfo")
                .expect("Couldn't find typeinfo");
            let typeinfo: TypeInfo = serde_json::value::from_value(typeinfo.clone())?;
            if typeinfo.object_type == "Ship" {
                let ship: Ship = serde_json::value::from_value(entry.clone())?;
                let ship = ProcessedShip::from(ship, &abilities);
                ships.insert(ship.id, ship);
            }
        }

        Ok(GameParams { ships })
    }

    pub fn get_ship(&self, id: u64) -> Option<&ProcessedShip> {
        self.ships.get(&id)
    }
}
