use serde_derive::{Deserialize, Serialize};
use serde_json::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;

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
                .map(|(k, v)| {
                    let options: Vec<_> = v
                        .abils
                        .iter()
                        .filter_map(|raw_abil| {
                            //println!("{} {}", raw_abil[0], raw_abil[1]);
                            // Note: Some abilities aren't actually defined, such as JP_Carriers_Gold
                            let a = abilities.get(&raw_abil[0])?.get(&raw_abil[1])?.clone();
                            /*if ship.id == 3753818064 {
                                println!("{} {} is {:?}", raw_abil[0], raw_abil[1], a);
                            }*/
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
    /*pub fn load_raw(data: &[u8]) -> std::result::Result<Self, serde_pickle::Error> {
        let mut data = data.to_vec();
        data.reverse();
        let mut decoder = libflate::zlib::Decoder::new(&data[..]).unwrap();
        let mut decoded_data = vec![];
        decoder.read_to_end(&mut decoded_data).unwrap();

        let v: serde_pickle::Value = serde_pickle::from_slice(&decoded_data).unwrap();
        //let v = Self::load_from_value(v)?;

        //let v = v.as_array().unwrap()[0].as_object().unwrap();

        // Extract the first element of the array as an object
        let v = match &v {
            serde_pickle::Value::List(l) => match &l[0] {
                serde_pickle::Value::Dict(d) => d,
                _ => {
                    panic!("First element was incorrect pickle type!");
                }
            },
            _ => {
                panic!("Got incorrect pickle type!");
            }
        };

        println!("There are {} elements", v.len());
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for (k, entry) in v.iter() {
            /*let typeinfo = entry
            .as_object()
            .unwrap()
            .get("typeinfo")
            .expect("Couldn't find typeinfo");*/
            let typeinfo = match entry {
                serde_pickle::Value::Dict(d) => d
                    .get(&serde_pickle::value::HashableValue::String(
                        "typeinfo".to_string(),
                    ))
                    .expect("Couldn't find typeinfo"),
                _ => {
                    panic!("Wrong type for entry");
                }
            };
            let typeinfo: TypeInfo = serde_pickle::value::from_value(typeinfo.clone())?;
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
            /*let typeinfo = entry
            .as_object()
            .unwrap()
            .get("typeinfo")
            .expect("Couldn't find typeinfo");*/
            let typeinfo = match entry {
                serde_pickle::Value::Dict(d) => d
                    .get(&serde_pickle::value::HashableValue::String(
                        "typeinfo".to_string(),
                    ))
                    .expect("Couldn't find typeinfo"),
                _ => {
                    panic!("Wrong type for entry");
                }
            };
            let typeinfo: TypeInfo = serde_pickle::value::from_value(typeinfo.clone())?;
            if typeinfo.object_type == "Ability" {
                let k = match k {
                    serde_pickle::value::HashableValue::String(i) => i,
                    _ => panic!("Ability index isn't a string"),
                };
                //let entry = entry.as_object().unwrap();
                let entry = match entry {
                    serde_pickle::Value::Dict(d) => d,
                    _ => panic!("Ability isn't a dict!"),
                };
                let mut ability = HashMap::new();
                for (k, v) in entry.iter() {
                    let k = match k {
                        serde_pickle::value::HashableValue::String(i) => i,
                        _ => panic!("Ability index isn't a string"),
                    };
                    match serde_pickle::value::from_value(v.clone()) {
                        Ok(parsed_ability) => {
                            ability.insert(k, parsed_ability);
                        }
                        Err(_) => {
                            // Ignore
                        }
                    }
                }
                /*let ability: HashMap<String, Ability> =
                serde_json::value::from_value(entry.clone())?;*/
                abilities.insert(k, ability);
            }
        }

        // Parse out the ships
        let mut ships = HashMap::new();
        for (k, entry) in v.iter() {
            /*let typeinfo = entry
            .as_object()
            .unwrap()
            .get("typeinfo")
            .expect("Couldn't find typeinfo");*/
            let typeinfo = match entry {
                serde_pickle::Value::Dict(d) => d
                    .get(&serde_pickle::value::HashableValue::String(
                        "typeinfo".to_string(),
                    ))
                    .expect("Couldn't find typeinfo"),
                _ => {
                    panic!("Wrong type for entry");
                }
            };
            let typeinfo: TypeInfo = serde_pickle::value::from_value(typeinfo.clone())?;
            if typeinfo.object_type == "Ship" {
                let ship: Ship = serde_pickle::value::from_value(entry.clone())?;
                let ship = ProcessedShip::from(ship, &abilities);
                ships.insert(ship.id, ship);
            }
        }

        Ok(GameParams { ships })

        /*std::fs::File::create("tmp")
            .unwrap()
            .write_all(&decoded_data)
            .unwrap();
        panic!("foo");*/
    }*/

    pub fn load(data: &[u8]) -> Result<Self> {
        let data = std::str::from_utf8(data).unwrap();
        let v: Value = serde_json::from_str(data)?;
        let v = v.as_array().unwrap()[0].as_object().unwrap();
        println!("There are {} elements", v.len());
        let mut type_counts: HashMap<String, usize> = HashMap::new();
        for (k, entry) in v.iter() {
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
                /*let ability: HashMap<String, Ability> =
                serde_json::value::from_value(entry.clone())?;*/
                abilities.insert(k, ability);
            }
        }

        // Parse out the ships
        let mut ships = HashMap::new();
        for (k, entry) in v.iter() {
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
