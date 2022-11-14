# victoria-metrics-writer
A Rust library for writing data to Victoria Metrics' JSON import endpoint. More information [here](https://docs.victoriametrics.com/#how-to-import-data-in-json-line-format).

## Example
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