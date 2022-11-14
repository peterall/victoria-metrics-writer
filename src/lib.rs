/*!
A Rust library for writing data to Victoria Metrics' JSON import endpoint. More information [here](https://docs.victoriametrics.com/#how-to-import-data-in-json-line-format).

# Usage
```rust
let mut writer = MetricsWriter::new("localhost:8428");

writer.add(
    "up",
    &BTreeMap::from([("job", "node_exporter"), ("instance", "localhost:9100")]),
    &[0, 0, 0],
    &[
        Utc.timestamp_millis_opt(1549891472010).unwrap(),
        Utc.timestamp_millis_opt(1549891487724).unwrap(),
        Utc.timestamp_millis_opt(1549891503438).unwrap(),
    ],
);

writer.send().await?;
```
*/

use std::{collections::BTreeMap, io::Write};

use bytes::{buf::Writer, BufMut};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Serialize;

use thiserror::Error;

pub struct MetricsWriter {
    url: String,
    client: reqwest::Client,
    writer: Option<Writer<Vec<u8>>>,
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error("error sending request")]
    RequestError(#[from] reqwest::Error),
    #[error("invalid response status code {0}")]
    InvalidResponseStatusCode(StatusCode),
}

#[derive(Serialize)]
struct Metric<'a, T> {
    #[serde(rename = "metric")]
    meta: MetricMeta<'a>,
    values: &'a [T],
    timestamps: &'a [i64],
}

#[derive(Serialize)]
struct MetricMeta<'a> {
    #[serde(rename = "__name__")]
    name: &'a str,
    #[serde(flatten)]
    labels: &'a BTreeMap<&'a str, &'a str>,
}

impl MetricsWriter {
    pub fn new(host: &str) -> Self {
        MetricsWriter {
            url: format!("http://{}/api/v1/import", host),
            client: reqwest::Client::new(),
            writer: None,
        }
    }

    pub fn add<T>(
        &mut self,
        name: &str,
        labels: &BTreeMap<&str, &str>,
        values: &[T],
        timestamps: &[DateTime<Utc>],
    ) where
        T: serde::Serialize,
    {
        let writer = self.writer.get_or_insert_with(|| vec![].writer());

        let ts: Vec<i64> = timestamps.iter().map(|ts| ts.timestamp_millis()).collect();
        let metric = Metric {
            meta: MetricMeta { name, labels },
            timestamps: &ts,
            values,
        };
        serde_json::to_writer(writer, &metric).unwrap();
        self.writer.as_mut().unwrap().write_all(b"\r\n").unwrap();
    }

    pub async fn send(&mut self) -> Result<(), SendError> {
        if let Some(writer) = self.writer.take() {
            let response = self
                .client
                .post(&self.url)
                .body(writer.into_inner())
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(SendError::InvalidResponseStatusCode(response.status()));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn payload(&mut self) -> Option<String> {
        self.writer
            .take()
            .map(|writer| String::from_utf8(writer.into_inner()).unwrap())
    }
}

#[cfg(test)]

mod tests {
    use chrono::TimeZone;

    use super::*;
    #[test]
    fn test_metric() {
        let mut writer = MetricsWriter::new("localhost:8428");

        writer.add(
            "up",
            &BTreeMap::from([("job", "node_exporter"), ("instance", "localhost:9100")]),
            &[0, 0, 0],
            &[
                Utc.timestamp_millis_opt(1549891472010).unwrap(),
                Utc.timestamp_millis_opt(1549891487724).unwrap(),
                Utc.timestamp_millis_opt(1549891503438).unwrap(),
            ],
        );

        writer.add(
            "up",
            &BTreeMap::from([("job", "prometheus"), ("instance", "localhost:9090")]),
            &[1, 1, 1],
            &[
                Utc.timestamp_millis_opt(1549891461511).unwrap(),
                Utc.timestamp_millis_opt(1549891476511).unwrap(),
                Utc.timestamp_millis_opt(1549891491511).unwrap(),
            ],
        );

        let payload = writer.payload().unwrap();
        assert_eq!(
            payload,
            concat!(
                r#"{"metric":{"__name__":"up","instance":"localhost:9100","job":"node_exporter"},"values":[0,0,0],"timestamps":[1549891472010,1549891487724,1549891503438]}"#,
                "\r\n",
                r#"{"metric":{"__name__":"up","instance":"localhost:9090","job":"prometheus"},"values":[1,1,1],"timestamps":[1549891461511,1549891476511,1549891491511]}"#,
                "\r\n"
            )
        );
    }
}
