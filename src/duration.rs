use humantime::parse_duration;
use rand::Rng;
use std::{fmt, str::FromStr, time::Duration};

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

pub struct ShortDurationFormatter(Duration);

impl ShortDurationFormatter {
    fn unitify(&self) -> (&'static str, f64) {
        let nanos = self.0.as_nanos();
        match self.0.as_nanos() {
            0..=999 => ("ns", nanos as f64),
            1_000..=999_999 => ("µs", nanos as f64 / 1_000f64),
            1_000_000..=999_999_999 => ("ms", nanos as f64 / 1_000_000f64),
            1_000_000_000..=59_999_999_999 => ("s", nanos as f64 / 1_000_000_000f64),
            60_000_000_000..=3_599_999_999_999 => ("m", nanos as f64 / 60_000_000_000f64),
            _ => ("h", nanos as f64 / 3_600_000_000_000f64),
        }
    }
}

impl From<Duration> for ShortDurationFormatter {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

// impl fmt::Display for ShortDurationFormatter {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let prec = f.precision().unwrap_or(3);
//         let nanos = self.0.as_nanos();
//         match self.0.as_nanos() {
//             0..=999 => write!(f, "{}ns", nanos),
//             1_000..=999_999 => write!(f, "{:.1$}µs", nanos as f64 / 1_000f64, prec),
//             1_000_000..=999_999_999 => write!(f, "{:.1$}ms", nanos as f64 / 1_000_000f64, prec),
//             1_000_000_000..=59_999_999_999 => {
//                 write!(f, "{:.1$}s", nanos as f64 / 1_000_000_000f64, prec)
//             }
//             60_000_000_000..=3_599_999_999_999_999 => {
//                 write!(f, "{:.1$}m", nanos as f64 / 60_000_000_000f64, prec)
//             }
//             _ => write!(f, "{:.1$}h", nanos as f64 / 3_600_000_000_000_000f64, prec),
//         }
//     }
// }

impl fmt::Display for ShortDurationFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (unit, v) = self.unitify();
        v.fmt(f)?;
        f.write_str(unit)
    }
}

pub fn format_duration(d: Duration) -> ShortDurationFormatter {
    d.into()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn duration_formatter() {
        let d = Duration::from_nanos(123);
        let f = format!("{}", format_duration(d));
        assert_eq!("123ns", f);

        let d = Duration::from_nanos(123_456);
        let f = format!("{}", format_duration(d));
        assert_eq!("123.456µs", f);

        let d = Duration::from_nanos(123_456_789);
        let f = format!("{}", format_duration(d));
        assert_eq!("123.456789ms", f);

        let d = Duration::from_nanos(23_456_789_012);
        let f = format!("{}", format_duration(d));
        assert_eq!("23.456789012s", f);

        let d = Duration::from_secs(90);
        let f = format!("{}", format_duration(d));
        assert_eq!("1.5m", f);

        let d = Duration::from_secs(1515);
        let f = format!("{}", format_duration(d));
        assert_eq!("25.25m", f);

        let d = Duration::from_secs(7200);
        let f = format!("{}", format_duration(d));
        assert_eq!("2h", f);

        let d = Duration::from_nanos(23_456_789_012);
        let f = format!("{:>8.2}", format_duration(d));
        assert_eq!("   23.46s", f);
    }
}
