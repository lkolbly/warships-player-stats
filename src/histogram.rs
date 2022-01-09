use tracing::*;

#[derive(Clone)]
pub struct Histogram {
    underlying: histogram::Histogram,
    bucket_size: f64,
    num_buckets: u64,
    max: f64,
}

impl Histogram {
    pub fn new(max: f64) -> Histogram {
        let bucket_size = max / 10_000.0;
        let num_buckets = (max / bucket_size) as u64;
        Histogram {
            underlying: histogram::Histogram::configure()
                .max_value(num_buckets)
                .build()
                .expect(&format!(
                    "Could not construct histogram with {} buckets of size {}",
                    num_buckets, bucket_size
                )),
            bucket_size: bucket_size,
            num_buckets: num_buckets,
            max: max,
        }
    }

    pub fn increment(&mut self, value: f32) -> Result<(), &'static str> {
        // Ignore histogram errors to avoid "sample value too large" errors
        let bucket = (value as f64 / self.bucket_size).floor() as u64;
        if bucket >= self.num_buckets {
            let _ = self.underlying.increment(self.num_buckets - 1);
            return Ok(());
        }
        let _ = self.underlying.increment(bucket);
        Ok(())
    }

    pub fn percentile(&self, percentile: f64) -> Result<f64, &'static str> {
        Ok(self.underlying.percentile(percentile)? as f64 * self.bucket_size)
    }

    pub fn get_percentile(&self, value: f64) -> Result<f64, &'static str> {
        let mut lower = 0.0;
        let mut upper = 100.0;
        let mut mid = 50.0;
        for _ in 0..10 {
            let x = self.percentile(mid)?;
            if x > value {
                upper = mid;
                mid = (upper + lower) / 2.;
            } else {
                lower = mid;
                mid = (upper + lower) / 2.;
            }
        }
        Ok(mid)
    }
}

/// Maintains a set of N histograms (N=10). Queries are returned based on the oldest histogram.
/// All histograms are updated simultaneously. After a certain number of items are processed,
/// the oldest histogram is removed and a new histogram is added.
///
/// The number of items between refreshes is total_items / N. total_items is, of course, not
/// known in advance - therefore, it is periodically updated, according to the number of
/// elements in the database.
pub struct RunningHistogram {
    label: String,
    histograms: Vec<Histogram>,
    pub max_value: f64,
    database_size: u64,
    items_processed: u64,
}

impl RunningHistogram {
    pub fn new(label: String, max_value: f64) -> Self {
        Self {
            label,
            histograms: vec![],
            max_value: max_value,
            database_size: 1_000,
            items_processed: 0,
        }
    }

    pub fn increment(&mut self, value: f64) {
        self.items_processed += 1;
        if value > self.max_value {
            self.max_value = value;
        }

        let need_refresh =
            self.items_processed > self.database_size / 10 || self.histograms.len() == 0;

        if need_refresh {
            if self.histograms.len() > 10 {
                self.histograms.remove(0);
            }
            trace!(
                "Refreshing histogram {} with max_value {}",
                self.label,
                self.max_value
            );
            self.histograms.push(Histogram::new(self.max_value));
            self.items_processed = 0;
        }

        for histogram in self.histograms.iter_mut() {
            // TODO: Make the underlying accept f64
            histogram.increment(value as f32);
        }
    }

    pub fn percentile(&self, percentile: f64) -> Result<f64, &'static str> {
        if self.histograms.len() == 0 {
            return Ok(0.0);
        }
        self.histograms[0].get_percentile(percentile)
    }

    pub fn update_db_size(&mut self, db_size: u64) {
        if db_size > 1_000 {
            self.database_size = db_size;
        }
    }
}
