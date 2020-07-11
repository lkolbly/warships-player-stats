use serde_derive::{Deserialize, Serialize};

use crate::histogram::Histogram;
use crate::wows_data::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct AveragedBatteryStats {
    pub frags: f32,
    pub hits: f32,
    pub hitrate: f32,
    pub shots: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AveragedShipStats {
    pub xp: f32,
    pub main_battery: AveragedBatteryStats,
    pub secondary_battery: AveragedBatteryStats,
    pub torpedoes: AveragedBatteryStats,
    pub ramming: AveragedBatteryStats,
    pub winrate: f32,
    pub win_survival_rate: f32,
    pub damage_dealt: f32,
    pub kills: f32,
    pub planes_killed: f32,
    pub points_captured: f32,
    pub spotted: f32,
    pub damage_on_spotting: f32,
}

impl AveragedBatteryStats {
    fn calculate(stats: &BatteryStats, num_battles: f32) -> AveragedBatteryStats {
        AveragedBatteryStats {
            frags: stats.frags as f32 / num_battles,
            hits: stats.hits as f32 / num_battles,
            hitrate: stats.hits as f32 / stats.shots as f32,
            shots: stats.shots as f32 / num_battles,
        }
    }
}

impl AveragedShipStats {
    pub fn calculate(stats: &DetailedStats) -> AveragedShipStats {
        let nbattles = (stats.wins + stats.losses) as f32;
        AveragedShipStats {
            xp: stats.xp as f32 / nbattles,
            main_battery: AveragedBatteryStats::calculate(&stats.main_battery, nbattles),
            secondary_battery: AveragedBatteryStats::calculate(&stats.second_battery, nbattles),
            torpedoes: AveragedBatteryStats::calculate(&stats.torpedoes, nbattles),
            ramming: AveragedBatteryStats::calculate(&stats.ramming, nbattles),
            winrate: stats.wins as f32 / nbattles,
            win_survival_rate: stats.survived_wins as f32 / stats.wins as f32,
            damage_dealt: stats.damage_dealt as f32 / nbattles,
            kills: stats.frags as f32 / nbattles,
            planes_killed: stats.planes_killed as f32 / nbattles,
            points_captured: stats.capture_points as f32 / nbattles,
            spotted: stats.ships_spotted as f32 / nbattles,
            damage_on_spotting: stats.damage_scouting as f32 / nbattles,
        }
    }
}

#[derive(Clone)]
pub struct BatteryHistogram {
    pub frags: Histogram,
    pub hits: Histogram,
    pub hitrate: Histogram,
    pub shots: Histogram,
}

#[derive(Clone)]
pub struct ShipStatsHistogram {
    pub xp: Histogram,
    pub main_battery: BatteryHistogram,
    pub secondary_battery: BatteryHistogram,
    pub torpedoes: BatteryHistogram,
    pub ramming: BatteryHistogram,
    pub winrate: Histogram,
    pub win_survival_rate: Histogram,
    pub damage_dealt: Histogram,
    pub kills: Histogram,
    pub planes_killed: Histogram,
    pub points_captured: Histogram,
    pub spotted: Histogram,
    pub damage_on_spotting: Histogram,
}

impl BatteryHistogram {
    fn new() -> BatteryHistogram {
        BatteryHistogram {
            frags: Histogram::new(20.0),
            hits: Histogram::new(1000.0),
            hitrate: Histogram::new(1.0),
            shots: Histogram::new(10_000.),
        }
    }

    fn increment(&mut self, value: &AveragedBatteryStats) -> Result<(), &'static str> {
        self.frags.increment(value.frags)?;
        self.hits.increment(value.hits)?;
        self.hitrate.increment(value.hitrate)?;
        self.shots.increment(value.shots)?;
        Ok(())
    }

    pub fn get_percentile(
        &self,
        value: &AveragedBatteryStats,
    ) -> Result<AveragedBatteryStats, &'static str> {
        Ok(AveragedBatteryStats {
            frags: self.frags.get_percentile(value.frags.into())? as f32,
            hits: self.hits.get_percentile(value.hits.into())? as f32,
            hitrate: self.hitrate.get_percentile(value.hitrate.into())? as f32,
            shots: self.shots.get_percentile(value.shots.into())? as f32,
        })
    }
}

impl ShipStatsHistogram {
    pub fn new() -> ShipStatsHistogram {
        ShipStatsHistogram {
            xp: Histogram::new(10_000.),
            main_battery: BatteryHistogram::new(),
            secondary_battery: BatteryHistogram::new(),
            torpedoes: BatteryHistogram::new(),
            ramming: BatteryHistogram::new(),
            winrate: Histogram::new(1.),
            win_survival_rate: Histogram::new(1.),
            damage_dealt: Histogram::new(1_000_000.),
            kills: Histogram::new(12.),
            planes_killed: Histogram::new(100.),
            points_captured: Histogram::new(10.),
            spotted: Histogram::new(100.),
            damage_on_spotting: Histogram::new(1_000_000.),
        }
    }

    pub fn increment(&mut self, value: &AveragedShipStats) -> Result<(), &'static str> {
        self.xp.increment(value.xp.into())?;
        self.main_battery.increment(&value.main_battery)?;
        self.secondary_battery.increment(&value.secondary_battery)?;
        self.torpedoes.increment(&value.torpedoes)?;
        self.ramming.increment(&value.ramming)?;
        self.winrate.increment(value.winrate.into())?;
        self.win_survival_rate
            .increment(value.win_survival_rate.into())?;
        self.damage_dealt.increment(value.damage_dealt.into())?;
        self.kills.increment(value.kills.into())?;
        self.planes_killed.increment(value.planes_killed.into())?;
        self.points_captured
            .increment(value.points_captured.into())?;
        self.spotted.increment(value.spotted.into())?;
        self.damage_on_spotting
            .increment(value.damage_on_spotting.into())?;
        Ok(())
    }

    pub fn get_percentile(
        &self,
        value: &AveragedShipStats,
    ) -> Result<AveragedShipStats, &'static str> {
        Ok(AveragedShipStats {
            xp: self.xp.get_percentile(value.xp.into())? as f32,
            main_battery: self.main_battery.get_percentile(&value.main_battery)?,
            secondary_battery: self
                .secondary_battery
                .get_percentile(&value.secondary_battery)?,
            torpedoes: self.torpedoes.get_percentile(&value.torpedoes)?,
            ramming: self.ramming.get_percentile(&value.ramming)?,
            winrate: self.winrate.get_percentile(value.winrate.into())? as f32,
            win_survival_rate: self
                .win_survival_rate
                .get_percentile(value.win_survival_rate.into())?
                as f32,
            damage_dealt: self
                .damage_dealt
                .get_percentile(value.damage_dealt.into())? as f32,
            kills: self.kills.get_percentile(value.kills.into())? as f32,
            planes_killed: self
                .planes_killed
                .get_percentile(value.planes_killed.into())? as f32,
            points_captured: self
                .points_captured
                .get_percentile(value.points_captured.into())? as f32,
            spotted: self.spotted.get_percentile(value.spotted.into())? as f32,
            damage_on_spotting: self
                .damage_on_spotting
                .get_percentile(value.damage_on_spotting.into())?
                as f32,
        })
    }
}
