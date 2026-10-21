use flash_tmux::{Result, session, tmux};
use std::{collections::BTreeMap, path::Path};

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" {
        println!(
            "flash-tmux {}\nCommands:\n  start --pane ID --client NAME [--mode search|treesitter|treesitter-search]\n  overlay --snapshot PATH --result PATH\n  validate-config",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if args[0] == "--version" {
        println!("flash-tmux {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut options = BTreeMap::new();
    for chunk in args[1..].chunks(2) {
        if chunk.len() != 2 || !chunk[0].starts_with("--") {
            return Err("expected --option VALUE".into());
        }
        options.insert(chunk[0].as_str(), chunk[1].as_str());
    }
    let required = |key: &str| -> Result<&str> {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("missing {key}").into())
    };
    match args[0].as_str() {
        "validate-config" => {
            tmux::config()?;
            Ok(())
        }
        "start" => session::start_mode(
            required("--pane")?,
            required("--client")?,
            options.get("--mode").copied().unwrap_or("search").parse()?,
        ),
        "overlay" => session::overlay(
            Path::new(required("--snapshot")?),
            Path::new(required("--result")?),
        ),
        _ => Err("unknown command; run flash-tmux --help".into()),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("flash-tmux: {error}");
        std::process::exit(1);
    }
}
