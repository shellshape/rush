mod duration;
mod request;

use crate::duration::{format_duration, DurationRange};
use anyhow::Result;
use clap::Parser;
use rayon::{
    prelude::{IntoParallelIterator, ParallelIterator},
    ThreadPoolBuilder,
};
use request::{Client, Response};
use reqwest::StatusCode;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read},
    num::{NonZeroU32, NonZeroUsize},
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

    /// Prints the results of each request to stdout CSV formatted;
    /// bypasses `silent`, if set
    #[arg(long)]
    csv: bool,

    /// Do not print any output.
    #[arg(short, long)]
    silent: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let wait: Option<DurationRange> = args.wait.map(|v| v.parse()).transpose()?;

    if !args.silent && args.parallel.get() > 1 && wait.as_ref().is_some_and(|v| v.is_flat()) {
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

    if let Some(path) = args.output {
        let f = get_output_file(&path)?;
        write_csv(&f, &res)?;
    }

    if args.csv {
        write_csv(io::stdout(), &res)?;
    } else if !args.silent {
        res.sort();
        print_stats(&res);
    }

    Ok(())
}

fn read_body_from_file(file_path: &str) -> Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let mut buf = vec![];
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

fn get_output_file(path: &str) -> Result<File> {
    let pth = Path::new(&path);

    let f = if pth.exists() {
        File::options().append(true).open(pth)
    } else {
        if let Some(parent) = pth.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        File::create(pth)
    }?;

    Ok(f)
}

fn write_csv(mut w: impl io::Write, res: &[Response]) -> Result<()> {
    for r in res {
        writeln!(w, "{},{},{}", r.timestamp, r.status, r.took.as_nanos())?;
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

    let median = get_median(&times);
    let pct_90 = get_nth_percentile(&times, 0.90);
    let pct_95 = get_nth_percentile(&times, 0.95);
    let pct_99 = get_nth_percentile(&times, 0.99);

    println!(
        "Results of {n} probes:\n\
        \n\
        Min:        {:>10.4}  ({min_s})\n\
        Max:        {:>10.4}  ({max_s})\n\
        First:      {:>10.4}  ({max_s})\n\
        Average:    {:>10.4}  ({first_s})\n\
        Median:     {:>10.4}\n\
        Std. Dev.:  {:>10.4}\n\
        90th %ile.: {:>10.4}\n\
        95th %ile.: {:>10.4}\n\
        99th %ile.: {:>10.4}\n\
        Total:      {:>10.4}\n\
        ",
        format_duration(min_t),
        format_duration(max_t),
        format_duration(first_t),
        format_duration(Duration::from_nanos(avg as u64)),
        format_duration(median),
        format_duration(Duration::from_nanos(sd as u64)),
        format_duration(pct_90),
        format_duration(pct_95),
        format_duration(pct_99),
        format_duration(sum),
    );

    print_binned_statuscodes(res);
}

fn get_median(times: &[Duration]) -> Duration {
    if times.len() % 2 == 1 {
        let middle = ((times.len() + 1) / 2) - 1;
        return times[middle];
    }

    let middle_l = (times.len() / 2) - 1;
    let middle_r = times.len() / 2;

    (times[middle_l] + times[middle_r]) / 2
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

fn print_binned_statuscodes(res: &[Response]) {
    let all = res.len() as f32;

    let res = res
        .iter()
        .fold(HashMap::<StatusCode, u64>::new(), |mut m, resp| {
            m.entry(resp.status).and_modify(|v| *v += 1).or_insert(1);
            m
        });

    let pad = res
        .iter()
        .max_by_key(|(_, v)| *v)
        .unwrap()
        .1
        .to_string()
        .len();

    for (status_code, n) in res {
        let prct = n as f32 / all * 100f32;
        println!("{status_code}:  {n:>0$} ({prct:>5.2}%)", pad);
    }
}
