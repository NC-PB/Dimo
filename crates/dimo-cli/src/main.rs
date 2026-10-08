//! Headless command line tool for Dimo.

use clap::Parser;

/// Dimo command line interface.
#[derive(Debug, Parser)]
#[command(name = "dimo", version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
