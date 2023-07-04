use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::{
    blocking::Request,
    header::{HeaderMap, HeaderName},
    Method, StatusCode, Url,
};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub struct Response {
    pub status: StatusCode,
    pub took: Duration,
    pub timestamp: DateTime<Utc>,
}

impl Ord for Response {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.took.cmp(&other.took)
    }
}

impl PartialOrd for Response {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.took.partial_cmp(&other.took)
    }
}

pub struct Client {
    client: reqwest::blocking::Client,
    url: Url,
    method: Method,
    body: Option<Vec<u8>>,
}

impl Client {
    pub fn new(url: &str, method: &str, body: Option<Vec<u8>>, headers: &[String]) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .default_headers(into_header_map(headers)?)
            .build()?;

        let url = url.parse()?;
        let method = method.parse()?;

        Ok(Self {
            client,
            url,
            method,
            body,
        })
    }

    pub fn send(&self) -> Result<Response> {
        let req = self.create_request();

        let before = Instant::now();
        let res = self.client.execute(req)?;
        let after = Instant::now();

        Ok(Response {
            status: res.status(),
            took: after - before,
            timestamp: Utc::now(),
        })
    }

    fn create_request(&self) -> Request {
        let mut req = Request::new(self.method.clone(), self.url.clone());
        if let Some(body) = self.body.clone() {
            *req.body_mut() = Some(body.try_into().unwrap());
        }

        req
    }
}

fn into_header_map(headers: &[String]) -> Result<HeaderMap> {
    let mut header_map = HeaderMap::new();

    for header_kv in headers {
        let (key, value) = header_kv
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("invalid header format"))?;

        let key = key.trim();
        let value = value.trim();

        if key.is_empty() {
            anyhow::bail!("empty header key");
        }

        if value.is_empty() {
            anyhow::bail!("empty header value");
        }

        let header_name: HeaderName = key.try_into()?;
        header_map.insert(header_name, value.parse()?);
    }

    Ok(header_map)
}
