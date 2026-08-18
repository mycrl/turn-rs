use std::{env, process};

use anyhow::{Result, anyhow, bail};

pub struct Args {
    pub config: String,
}

impl Args {
    pub fn parse() -> Result<Self> {
        let mut config = None;
        let mut argv = env::args().skip(1);

        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    print_help();
                    process::exit(0);
                }
                "-V" | "--version" => {
                    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                    process::exit(0);
                }
                "-c" | "--config" => {
                    config = Some(
                        argv.next()
                            .ok_or_else(|| anyhow!("missing value for --config"))?,
                    );
                }
                arg => {
                    if let Some(path) = arg.strip_prefix("--config=") {
                        if path.is_empty() {
                            bail!("missing value for --config");
                        }

                        config = Some(path.to_string());
                    } else {
                        bail!("unknown argument: {arg}");
                    }
                }
            }
        }

        Ok(Self {
            config: config.ok_or_else(|| anyhow!("missing required argument: --config <PATH>"))?,
        })
    }
}

fn print_help() {
    println!(
        "\
{} {}
{}

Usage: turn-server --config <PATH>

Options:
  -c, --config <PATH>  Path to the configuration file
  -h, --help           Print help
  -V, --version        Print version",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    );
}
