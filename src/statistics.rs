use std::collections::HashMap;

use crate::histogram::RunningHistogram;
use crate::wows_data::*;

fn initial_max_val(key: &str) -> f64 {
    match key {
        "main_battery.frags" => 20.0,
        "main_battery.shots" => 10_000.0,
        "main_battery.hits" => 1000.0,
        "main_battery.hitrate" => 1.0,
        "second_battery.frags" => 20.0,
        "second_battery.shots" => 10_000.0,
        "second_battery.hits" => 1000.0,
        "second_battery.hitrate" => 1.0,
        "torpedoes.frags" => 20.0,
        "torpedoes.shots" => 10_000.0,
        "torpedoes.hits" => 1000.0,
        "torpedoes.hitrate" => 1.0,
        "ramming.frags" => 20.0,
        "ramming.shots" => 10_000.0,
        "ramming.hits" => 1000.0,
        "ramming.hitrate" => 1.0,
        "aircraft.frags" => 20.0,
        "aircraft.shots" => 10_000.0,
        "aircraft.hits" => 1000.0,
        "aircraft.hitrate" => 1.0,

        "xp" => 10_000.0,
        "capture_points" => 10.0,
        "planes_killed" => 100.0,
        "survived_wins" => 1.0,
        "damage_scouting" => 1_000_000.0,
        "damage_dealt" => 1_000_000.0,
        "ships_spotted" => 100.0,
        "frags" => 12.0,
        "win_surviverate" => 1.0,
        "winrate" => 1.0,

        _ => 0.0,
    }
}

pub struct StatsHistogram {
    pub ships: HashMap<u64, HashMap<String, RunningHistogram>>,
    database_size: u64,
}

impl StatsHistogram {
    pub fn new() -> Self {
        Self {
            ships: HashMap::new(),
            database_size: 100_000,
        }
    }

    pub fn set_database_size(&mut self, total_size: u64) {
        self.database_size = total_size;
        for (_, v) in self.ships.iter_mut() {
            for (_, h) in v.iter_mut() {
                h.update_db_size(total_size);
            }
        }
    }

    pub fn increment(&mut self, shipid: u64, stats: &DetailedStats) {
        let stats = stats.into_map();
        if !self.ships.contains_key(&shipid) {
            self.ships.insert(shipid, HashMap::new());
        }

        let entry = self.ships.get_mut(&shipid).unwrap();
        for (k, v) in stats.iter() {
            if !entry.contains_key(k) {
                let mut h = RunningHistogram::new(format!("{}-{}", shipid, k), initial_max_val(k));
                h.update_db_size(self.database_size);
                entry.insert(k.to_owned(), h);
            }
            entry.get_mut(k).unwrap().increment(*v);
        }
    }

    pub fn get_percentiles(&self, shipid: u64, stats: &DetailedStats) -> HashMap<String, f64> {
        let stats = stats.into_map();
        let entry = self.ships.get(&shipid).unwrap();
        let mut result = HashMap::new();
        for (k, v) in stats.iter() {
            let histogram = entry.get(k).unwrap();
            result.insert(k.to_owned(), histogram.percentile(*v).unwrap_or(0.0));
        }
        result
    }
}
