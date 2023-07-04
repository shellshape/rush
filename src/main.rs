mod duration;
mod request;

use crate::duration::DurationRange;
use anyhow::Result;
use clap::Parser;
use humantime::format_duration;
use rayon::{
    prelude::{IntoParallelIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use request::{Client, Response};
use std::{
    fs::{self, File},
    io::{Read, Write},
    num::{NonZeroU32, NonZeroUsize},
    os,
    path::Path,
    thread,
    time::Duration,
};

/// A tiny HTTP benchmarking and performance testing tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The URL to be requested
    url: String,

    /// The HTTP method to be used
    #[arg(short = 'X', long, default_value = "GET")]
    method: String,

    /// The HTTP headers to be sent with the request;
    /// format is 'key: value'
    #[arg(short = 'H', long)]
    header: Vec<String>,

    /// The body content to be sent with the request
    #[arg(short, long)]
    body: Option<String>,

    /// Reads the contents of the file and uses it as body
    /// for the request; overwrites `body`, if both set
    #[arg(short = 'f', long)]
    body_file: Option<String>,

    /// The amount of requests which will be sent
    #[arg(short, long, visible_short_alias = 'n', default_value = "1")]
    count: NonZeroU32,

    /// The maximum amount of requests which will be sent
    /// concurrently at a given time
    #[arg(short, long, default_value = "1")]
    parallel: NonZeroUsize,

    /// A duration awaited before a request is sent; you can pass
    /// a range (format: 'from..to', e.g. '10ms..20ms') from which
    /// a random duration will be picked
    #[arg(short, long)]
    wait: Option<String>,

    /// Writes the results of each request formatted as CSV to
    /// the given output directory; appends the file if it already
    /// exists
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let wait: Option<DurationRange> = args.wait.map(|v| v.parse()).transpose()?;

    if args.parallel.get() > 1 && wait.as_ref().is_some_and(|v| v.is_flat()) {
        println!(
            "warning: `wait` is set to a fixed duration and `parallel` is set to more than 1. \
            That means that all requests will wait the same time for each worker. To avoid this, \
            use a range for `wait`. For example: `-w 900ms..1100ms`."
        );
    }

    let body = args
        .body_file
        .map(|path| read_body_from_file(&path))
        .or_else(|| args.body.map(|v| Ok(v.into_bytes())))
        .transpose()?;

    let client = Client::new(&args.url, &args.method, body, &args.header)?;

    let pool = ThreadPoolBuilder::new()
        .num_threads(args.parallel.into())
        .build()?;

    let res: Result<Vec<_>, _> = pool.install(|| {
        (0..args.count.into())
            .into_par_iter()
            .map(|_| {
                if let Some(wait) = &wait {
                    thread::sleep(wait.get_random());
                }
                client.send()
            })
            .collect()
    });

    let mut res = res?;

    if let Some(csv) = args.output {
        write_to_csv(&csv, &res)?;
    }

    res.sort();

    print_stats(&res);

    Ok(())
}

fn read_body_from_file(file_path: &str) -> Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let mut buf = vec![];
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

fn write_to_csv(path: &str, res: &[Response]) -> Result<()> {
    let pth = Path::new(&path);

    let mut f = if pth.exists() {
        File::options().append(true).open(pth)
    } else {
        if let Some(parent) = pth.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        File::create(pth)
    }?;

    for r in res {
        writeln!(f, "{},{},{}", r.timestamp, r.status, r.took.as_nanos())?;
    }

    Ok(())
}

fn print_stats(res: &[Response]) {
    if res.is_empty() {
        println!("no result values");
        return;
    }

    let n = res.len() as f64;

    let min = res.iter().min();
    let max = res.iter().max();

    let min_t = min.unwrap().took;
    let min_s = min.unwrap().status;
    let max_t = max.unwrap().took;
    let max_s = max.unwrap().status;

    let first_t = res.first().unwrap().took;
    let first_s = res.first().unwrap().status;

    let times: Vec<_> = res.iter().map(|r| r.took).collect();
    let sum: Duration = times.iter().sum();
    let avg = sum.as_nanos() as f64 / n;
    let sd = (times
        .iter()
        .map(|v| v.as_nanos() as f64)
        .fold(0f64, |acc, v| acc + (v - avg).powf(2f64))
        / n)
        .sqrt();

    let pct_90 = get_nth_percentile(&times, 0.90);
    let pct_95 = get_nth_percentile(&times, 0.95);
    let pct_99 = get_nth_percentile(&times, 0.99);

    println!(
        "Results of {n} probes:\n\
        \n\
        Min:         {}\t({min_s})\n\
        Max:         {}\t({max_s})\n\
        First:       {}\t({max_s})\n\
        Average:     {}\t({first_s})\n\
        Std. Dev.:   {}\n\
        90th %ile.:  {}\n\
        95th %ile.:  {}\n\
        99th %ile.:  {}\n\
        Total:       {}\n\
        ",
        format_duration(min_t),
        format_duration(max_t),
        format_duration(first_t),
        format_duration(Duration::from_nanos(avg as u64)),
        format_duration(Duration::from_nanos(sd as u64)),
        format_duration(pct_90),
        format_duration(pct_95),
        format_duration(pct_99),
        format_duration(sum),
    );
}

fn get_nth_percentile(times: &[Duration], percentile: f64) -> Duration {
    let el = times.len() as f64 * percentile;
    let el_trunc = el as isize - 1;
    if el_trunc < 0 {
        return times[0];
    }

    if el_trunc as usize + 1 >= times.len() {
        return times[el_trunc as usize];
    }

    let el_a = times[el_trunc as usize];
    let el_b = times[el_trunc as usize + 1];

    let el_fract_b = el - el_trunc as f64;
    let el_fract_a = 1f64 - el_fract_b;

    let res = (el_a.as_nanos() as f64 * el_fract_a + el_b.as_nanos() as f64 * el_fract_b).round();

    Duration::from_nanos(res as u64)
}
