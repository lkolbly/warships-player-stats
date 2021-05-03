use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct GenericReplyMeta {
    pub count: Option<u64>,
    pub page_total: Option<u64>,
    pub total: Option<u64>,
    pub limit: Option<u64>,
    pub page: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GenericReply<T> {
    pub status: String,
    pub meta: GenericReplyMeta,
    pub data: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MobilityProfile {
    pub rudder_time: f32,
    pub total: f32,
    pub turning_radius: f32,
    pub max_speed: f32,
}

/*
                "torpedoes": {
                    "visibility_dist": 1.3,
                    "distance": 8.0,
                    "torpedoes_id": 3763744720,
                    "torpedo_name": "533 mm Mk\u00a0IX",
                    "reload_time": 96,
                    "torpedo_speed": 61,
                    "rotation_time": 7.2,
                    "torpedoes_id_str": "PBUT506",
                    "slots": {
                        "0": {
                            "barrels": 4,
                            "caliber": 533,
                            "name": "533 mm QR Mk\u00a0IV",
                            "guns": 2
                        }
                    },
                    "max_damage": 15433
                },
*/

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TorpedoesProfile {
    #[serde(rename = "torpedoes_id")]
    pub id: u64,
    #[serde(rename = "torpedo_name")]
    pub name: String,
    #[serde(rename = "distance")]
    pub range: f32,
    #[serde(rename = "torpedo_speed")]
    pub speed: u64,
    pub reload_time: u32,
    pub max_damage: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetailedModuleInfoTorpedoProfile {
    pub torpedo_speed: u64,
    pub shot_speed: f32,
    pub max_damage: u64,
    pub distance: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetailedModuleInfoProfile {
    pub torpedoes: Option<DetailedModuleInfoTorpedoProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetailedModuleInfo {
    pub module_id: u64,
    #[serde(rename = "type")]
    pub module_type: String,
    pub profile: DetailedModuleInfoProfile,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipProfile {
    pub mobility: Option<MobilityProfile>,
    pub torpedoes: Option<TorpedoesProfile>,
    pub battle_level_range_max: Option<u16>,
    pub battle_level_range_min: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    //next_modules
    pub is_default: bool,
    pub price_xp: u64,
    pub price_credit: u64,
    pub next_ships: Option<Vec<u64>>,
    pub next_modules: Option<Vec<u64>>,
    pub module_id: u64,

    #[serde(rename = "type")]
    pub module_type: String,

    pub module_id_str: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShipInfo {
    pub description: String,
    pub price_gold: u32,
    pub ship_id_str: String,
    pub has_demo_profile: bool,
    pub images: HashMap<String, String>,
    pub modules: HashMap<String, Vec<u64>>,
    pub modules_tree: HashMap<String, ModuleInfo>,
    pub nation: String,
    pub is_premium: bool,
    pub ship_id: u64,
    pub price_credit: u64,
    pub default_profile: ShipProfile,
    pub upgrades: Option<Vec<u64>>,
    pub tier: u64,
    pub next_ships: HashMap<String, u64>,
    pub mod_slots: u64,
    #[serde(rename = "type")]
    pub ship_type: String,
    pub is_special: bool,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatteryStats {
    pub max_frags_battle: u8,
    pub frags: u32,

    #[serde(default)]
    pub hits: u32,

    #[serde(default)]
    pub shots: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedStats {
    pub max_xp: u32,
    pub damage_to_buildings: u32,
    pub main_battery: BatteryStats,
    pub suppressions_count: u32,
    pub max_damage_scouting: u32,
    pub art_agro: u64,
    pub ships_spotted: u32,
    pub second_battery: BatteryStats,
    pub xp: u32,
    pub survived_battles: u32,
    pub dropped_capture_points: u32,
    pub max_damage_dealt_to_buildings: u32,
    pub torpedo_agro: u64,
    pub draws: u32,
    pub battles_since_510: u32,
    pub planes_killed: u32,
    pub battles: u32,
    pub max_ships_spotted: u32,
    pub team_capture_points: u32,
    pub frags: u32,
    pub damage_scouting: u32,
    pub max_total_agro: u32,
    pub max_frags_battle: u32,
    pub capture_points: u32,
    pub ramming: BatteryStats,
    pub torpedoes: BatteryStats,
    pub aircraft: BatteryStats,
    pub survived_wins: u32,
    pub max_damage_dealt: u32,
    pub wins: u32,
    pub losses: u32,
    pub damage_dealt: u32,
    pub max_planes_killed: u32,
    pub max_suppressions_count: u32,
    pub team_dropped_capture_points: u32,
    pub battles_since_512: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedStatTypes {
    pub pvp: DetailedStats,
    pub last_battle_time: u64,
    pub account_id: u64,
    pub distance: u64,
    pub updated_at: u64,
    pub battles: u64,
    pub ship_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub nickname: String,
    pub account_id: u64,
}
