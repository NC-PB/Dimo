//! Spike for T3.2: Typst as a library behind a sandboxed `World`.
//!
//! Usage:
//!   cargo run --release -- render <rows> <samples> <out.pdf>   render twice, compare, time
//!   cargo run --release -- sandbox                             run the escape attempts
//!
//! Without the `render` feature the binary does nothing; it is the baseline for the binary size
//! and build time numbers in RESULTS.md.

#[cfg(feature = "render")]
mod sandbox;

#[cfg(feature = "render")]
mod spike {
    use std::fmt::Write as _;
    use std::time::Instant;

    use crate::sandbox::{SandboxWorld, render};

    const TEMPLATE: &str = include_str!("../report.typ");

    fn data_json(rows: usize, samples: usize) -> String {
        let mut s =
            String::from("{\"title\":\"Inspection report (spike) Ø ± ° ′ ″ × ⌀ − äöüÄÖÜß\",");
        s.push_str("\"samples\":[");
        for j in 0..samples {
            if j > 0 {
                s.push(',');
            }
            let _ = write!(s, "\"S{}\"", j + 1);
        }
        s.push_str("],\"rows\":[");
        for i in 0..rows {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(
                s,
                "{{\"no\":\"{n}\",\"req\":\"Ø{d}.{f:02} ±0.05\",\"nom\":\"{d}.{f:02}\",\"lo\":\"{d}.{l:02}\",\"hi\":\"{d}.{h:02}\",\"vals\":[",
                n = i + 1,
                d = 10 + i % 90,
                f = i % 100,
                l = (i % 100).saturating_sub(5),
                h = (i % 100) + 5
            );
            for j in 0..samples {
                if j > 0 {
                    s.push(',');
                }
                let _ = write!(s, "\"{}.{:03}\"", 10 + i % 90, (i * 7 + j * 3) % 1000);
            }
            s.push_str("]}");
        }
        s.push_str("]}");
        s
    }

    fn sha_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes)
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x}");
                s
            })
    }

    pub fn render_cmd(rows: usize, samples: usize, out: &str) {
        let data = data_json(rows, samples);
        let ident = sha_hex(data.as_bytes());
        let mut outputs = Vec::new();
        for run in 0..2 {
            // A fresh world and an empty memoization cache per run: nothing is shared between
            // the two renders, so both are cold and the comparison is meaningful.
            typst::comemo::evict(0);
            let world = SandboxWorld::new(TEMPLATE, &data);
            let start = Instant::now();
            let (pdf, pages) = render(&world, &ident).expect("render");
            let ms = start.elapsed().as_millis();
            println!(
                "run {run}: {rows} rows x {samples} samples, {pages} pages, {} bytes, {ms} ms, sha256 {}",
                pdf.len(),
                sha_hex(&pdf)
            );
            outputs.push(pdf);
        }
        // Third render in the same process with a reused world (comemo cache warm).
        let world = SandboxWorld::new(TEMPLATE, &data);
        let _ = render(&world, &ident).expect("render");
        let start = Instant::now();
        let (pdf, _) = render(&world, &ident).expect("render");
        println!("warm render: {} ms", start.elapsed().as_millis());
        outputs.push(pdf);
        let identical = outputs.windows(2).all(|w| w[0] == w[1]);
        println!("byte identical across runs: {identical}");
        std::fs::write(out, &outputs[0]).expect("write pdf");
    }

    pub fn sandbox_cmd() {
        let cases = [
            ("read absolute host path", "#read(\"/etc/passwd\")"),
            ("read parent path", "#read(\"../../Cargo.toml\")"),
            ("import file", "#import \"/other.typ\": *"),
            ("include file", "#include \"secret.typ\""),
            ("import package", "#import \"@preview/tablex:0.0.8\": *"),
            ("import local package", "#import \"@local/foo:0.1.0\": *"),
            ("load plugin", "#plugin(\"/evil.wasm\")"),
            ("image from disk", "#image(\"/Users/x/a.png\")"),
            ("clock", "#datetime.today().display()"),
            ("data file is readable", "#json(\"/data.json\").title"),
        ];
        for (name, src) in cases {
            let world = SandboxWorld::new(src, "{\"title\":\"ok\"}");
            match render(&world, "x") {
                Ok((pdf, _)) => println!("{name}: rendered ({} bytes)", pdf.len()),
                Err(e) => println!("{name}: error: {e}"),
            }
        }
    }
}

fn main() {
    #[cfg(feature = "render")]
    {
        let args: Vec<String> = std::env::args().collect();
        match args.get(1).map(String::as_str) {
            Some("render") => {
                let rows = args[2].parse().expect("rows");
                let samples = args[3].parse().expect("samples");
                spike::render_cmd(rows, samples, &args[4]);
            }
            Some("sandbox") => spike::sandbox_cmd(),
            _ => eprintln!("usage: render <rows> <samples> <out.pdf> | sandbox"),
        }
    }
}
