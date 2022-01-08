use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;

#[derive(Debug, Deserialize)]
pub struct GenericReplyMeta {
    pub count: Option<u64>,
    pub page_total: Option<u64>,
    pub total: Option<u64>,
    pub limit: Option<u64>,
    pub page: Option<u64>,
}

//{"status":"error","error":{"code":504,"message":"SOURCE_NOT_AVAILABLE","field":null,"value":null}}
#[derive(Debug, Deserialize)]
pub struct GenericReplyError {
    pub code: u32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct GenericReply<T> {
    pub status: String,
    pub meta: Option<GenericReplyMeta>,
    pub error: Option<GenericReplyError>,
    pub data: Option<T>,
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

impl BatteryStats {
    pub fn into_map(&self, m: &mut HashMap<String, f64>, prefix: &str, nbattles: f64) {
        m.insert(
            format!("{}.max_frags_battle", prefix),
            self.max_frags_battle as f64,
        );
        m.insert(format!("{}.frags", prefix), self.frags as f64 / nbattles);
        m.insert(format!("{}.hits", prefix), self.hits as f64 / nbattles);
        m.insert(format!("{}.shots", prefix), self.shots as f64 / nbattles);
    }
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

impl DetailedStats {
    pub fn into_map(&self) -> HashMap<String, f64> {
        let nbattles = self.battles as f64;

        let mut m = HashMap::new();
        self.main_battery.into_map(&mut m, "main_battery", nbattles);
        self.second_battery
            .into_map(&mut m, "second_battery", nbattles);
        self.ramming.into_map(&mut m, "ramming", nbattles);
        self.torpedoes.into_map(&mut m, "torpedoes", nbattles);
        self.aircraft.into_map(&mut m, "aircraft", nbattles);
        m.insert("max_xp".to_owned(), self.max_xp as f64);
        m.insert(
            "damage_to_buildings".to_owned(),
            self.damage_to_buildings as f64,
        );
        m.insert(
            "suppressions_count".to_owned(),
            self.suppressions_count as f64,
        );
        m.insert(
            "max_damage_scouting".to_owned(),
            self.max_damage_scouting as f64,
        );
        m.insert("art_agro".to_owned(), self.art_agro as f64);
        m.insert(
            "ships_spotted".to_owned(),
            self.ships_spotted as f64 / nbattles,
        );
        m.insert("xp".to_owned(), self.xp as f64 / nbattles);
        m.insert("survived_battles".to_owned(), self.survived_battles as f64);
        m.insert(
            "dropped_capture_points".to_owned(),
            self.dropped_capture_points as f64 / nbattles,
        );
        m.insert(
            "max_damage_dealt_to_buildings".to_owned(),
            self.max_damage_dealt_to_buildings as f64,
        );
        m.insert("torpedo_agro".to_owned(), self.torpedo_agro as f64);
        m.insert("draws".to_owned(), self.draws as f64);
        m.insert(
            "battles_since_510".to_owned(),
            self.battles_since_510 as f64,
        );
        m.insert(
            "planes_killed".to_owned(),
            self.planes_killed as f64 / nbattles,
        );
        m.insert("battles".to_owned(), self.battles as f64);
        m.insert(
            "max_ships_spotted".to_owned(),
            self.max_ships_spotted as f64,
        );
        m.insert(
            "team_capture_points".to_owned(),
            self.team_capture_points as f64 / nbattles,
        );
        m.insert("frags".to_owned(), self.frags as f64 / nbattles);
        m.insert(
            "damage_scouting".to_owned(),
            self.damage_scouting as f64 / nbattles,
        );
        m.insert("max_total_agro".to_owned(), self.max_total_agro as f64);
        m.insert("max_frags_battle".to_owned(), self.max_frags_battle as f64);
        m.insert(
            "capture_points".to_owned(),
            self.capture_points as f64 / nbattles,
        );
        m.insert(
            "survived_wins".to_owned(),
            self.survived_wins as f64 / self.wins as f64,
        );
        m.insert("max_damage_dealt".to_owned(), self.max_damage_dealt as f64);
        m.insert("wins".to_owned(), self.wins as f64 / nbattles);
        m.insert("losses".to_owned(), self.losses as f64 / nbattles);
        m.insert(
            "damage_dealt".to_owned(),
            self.damage_dealt as f64 / nbattles,
        );
        m.insert(
            "max_planes_killed".to_owned(),
            self.max_planes_killed as f64,
        );
        m.insert(
            "max_suppressions_count".to_owned(),
            self.max_suppressions_count as f64,
        );
        m.insert(
            "team_dropped_capture_points".to_owned(),
            self.team_dropped_capture_points as f64,
        );
        m.insert(
            "battles_since_512".to_owned(),
            self.battles_since_512 as f64,
        );
        m
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerRecord {
    pub nickname: String,
    pub account_id: u64,
}
