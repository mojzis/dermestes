//! Thin entry point: parse arguments, delegate to [`dermestes::cli::Cli::run`],
//! and map its outcome onto an exit code.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

use dermestes::cli::{Cli, Outcome};

/// Exit code for a usage or internal error. clap uses the same code for
/// usage errors, so the contract holds without special-casing them.
const EXIT_ERROR: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let result = cli.run(&mut out).and_then(|outcome| {
        out.flush()?;
        Ok(outcome)
    });

    match result {
        Ok(Outcome::Clean) => ExitCode::SUCCESS,
        Ok(Outcome::Findings) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("dermestes: {err:#}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}
