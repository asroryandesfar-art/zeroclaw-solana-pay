//! `solpay` CLI entrypoint.
//!
//! Exit codes (the contract ZeroClaw SOPs branch on):
//!   0 success · 2 invalid input · 3 config error · 4 RPC/transient · 5 internal
//!
//! clap maps argument/usage errors to exit code 2, which aligns with our
//! "invalid input" category.

use clap::Parser;

use solpay::cli::{dispatch, Cli};
use solpay::output::OutputFormat;

fn main() {
    let cli = Cli::parse();
    let format: OutputFormat = cli.format.into();

    match dispatch(cli.command, format) {
        Ok(rendered) => {
            println!("{rendered}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error: {}", e.message);
            std::process::exit(e.code.as_i32());
        }
    }
}
