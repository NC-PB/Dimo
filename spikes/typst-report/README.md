# Spike: Typst report rendering (T3.2)

Measures whether Typst as a library fits Dimo's PDF reports (D-30, M3 decision 11): crates,
licenses, binary size, build time, render time, determinism, sandbox. Results and verdict in
[RESULTS.md](RESULTS.md).

Standalone Cargo project (own `[workspace]` and `Cargo.lock`), not a member of the Dimo
workspace, so it does not change the app's dependency tree.

```sh
cd spikes/typst-report
cargo run --release -- render 90 6 /tmp/report.pdf   # 3 pages: renders twice, compares, times
cargo run --release -- render 300 10 /tmp/big.pdf    # 300 rows by 10 samples
cargo run --release -- sandbox                       # escape attempts, each must fail
cargo build --release --no-default-features          # empty baseline binary
```
