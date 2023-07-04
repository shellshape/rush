use std::{str::FromStr, time::Duration};

use humantime::parse_duration;
use rand::Rng;

pub struct DurationRange(Duration, Duration);

impl DurationRange {
    pub fn get_random(&self) -> Duration {
        if self.is_flat() {
            return self.0;
        }

        rand::thread_rng().gen_range(self.0..self.1)
    }

    pub fn is_flat(&self) -> bool {
        self.0 == self.1
    }
}

impl FromStr for DurationRange {
    type Err = humantime::DurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((start, end)) = s.split_once("..") {
            let start = parse_duration(start)?;
            let end = parse_duration(end)?;
            return Ok(Self(start, end));
        }

        let d = parse_duration(s)?;
        Ok(Self(d, d))
    }
}
