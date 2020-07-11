use std::time::{Instant, self};

pub struct ProgressLogger {
    tagline: String,
    last_report_time: Instant,
    item_count: usize,
    total: usize,
    target: Option<usize>,
}

impl ProgressLogger {
    pub fn new(tagline: &str) -> ProgressLogger {
        ProgressLogger {
            tagline: tagline.to_string(),
            last_report_time: Instant::now(),
            item_count: 0,
            total: 0,
            target: None,
        }
    }

    pub fn new_with_target(tagline: &str, target: usize) -> ProgressLogger {
        ProgressLogger {
            tagline: tagline.to_string(),
            last_report_time: Instant::now(),
            item_count: 0,
            total: 0,
            target: Some(target),
        }
    }

    pub fn increment(&mut self, count: usize) {
        self.item_count += count;
        self.total += count;
        let elapsed = self.last_report_time.elapsed().as_secs_f64();
        if elapsed > 60.0 {
            let rate = self.item_count as f64 / elapsed;
            match self.target {
                Some(target) => {
                    let remaining = if target < self.total { 0 } else { target - self.total };
                    println!("{}: {}/{} items. {} in {:.2}s = {:.2} items/sec ETA {:.0}s", self.tagline, self.total, target, self.item_count, elapsed, rate, remaining as f64 / rate);
                },
                None => {
                    println!("{}: {} items in {}s = {:.2} items/sec (total: {})", self.tagline, self.item_count, elapsed, self.item_count as f64 / elapsed, self.total);
                }
            }
            //self.total += self.item_count;
            self.last_report_time = Instant::now();
            self.item_count = 0;
        }
    }
}
