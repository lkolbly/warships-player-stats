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
