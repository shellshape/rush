# rush

A tiny HTTP benchmarking and performance testing CLI tool.

This tool has been primarily created to collect the data for [this project](https://github.com/zekroTJA/vercel-benchmarks), but it is also desigend to be used in a general purpose manner.

## Usage

```
$ rush --help
A tiny HTTP benchmarking and performance testing CLI tool.

Usage: rush [OPTIONS] <URL>

Arguments:
  <URL>  The URL to be requested

Options:
  -X, --method <METHOD>        The HTTP method to be used [default: GET]
  -H, --header <HEADER>        The HTTP headers to be sent with the request; format is 'key: value'
  -b, --body <BODY>            The body content to be sent with the request
  -f, --body-file <BODY_FILE>  Reads the contents of the file and uses it as body for the request; overwrites `body`, if both set
  -c, --count <COUNT>          The amount of requests which will be sent [default: 1] [short aliases: n]
  -p, --parallel <PARALLEL>    The maximum amount of requests which will be sent concurrently at a given time [default: 1]
  -w, --warmup <WARMUP>        Perform warmup requests which do not count to the benchmark result
  -w, --wait <WAIT>            A duration awaited before a request is sent; you can pass a range (format: 'from..to', e.g. '10ms..20ms') from which a random duration will be picked
  -o, --output <OUTPUT>        Writes the results of each request formatted as CSV to the given output directory; appends the file if it already exists
      --csv                    Prints the results of each request to stdout CSV formatted; bypasses `silent`, if set
  -s, --silent                 Do not print any output
  -i, --insecure               Disable TLS certificate invalidation
  -h, --help                   Print help
  -V, --version                Print version
```

## Install

You can either download the latest release builds form the [Releases page](https://github.com/shellshape/rush/releases) or you can install it using cargo install.
```
cargo install --git https://github.com/shellshape/rush
```

Alternatively, you can also use the provided Docker image.
```
docker run --rm -it ghcr-io/shellshape/rush \
    https://example.com -X GET -n 100 -p 3 -w 10ms..20ms --csv \
        > results.csv
```
