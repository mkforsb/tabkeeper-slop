//! Runs an Interest script headlessly and prints the output and logs.
//!
//!     cargo run --example run_script -- path/to/script.rhai
//!     cargo run --example run_script -- --template youtube

use tabkeeper::script::{self, RunInput};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let script = match args.as_slice() {
        [flag, key] if flag == "--template" => {
            tabkeeper::templates::find(key).unwrap_or_else(|| panic!("no template '{key}'")).script.to_string()
        }
        [path] => std::fs::read_to_string(path).expect("read script"),
        _ => {
            eprintln!("usage: run_script <file.rhai> | --template <key>");
            std::process::exit(2);
        }
    };
    let report = script::run(RunInput { script, name: "cli".into(), prev: None, cors_proxy: String::new() }).await;
    for f in &report.fetches {
        eprintln!("GET {} -> {:?} {} bytes {} ms {}", f.url, f.status, f.bytes, f.ms, f.error.clone().unwrap_or_default());
    }
    for l in &report.logs {
        eprintln!("log: {l}");
    }
    eprintln!("{} rounds, {} ms", report.rounds, report.duration_ms);
    match report.result {
        Ok(out) => println!("{}", serde_json::to_string_pretty(&out).unwrap()),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
