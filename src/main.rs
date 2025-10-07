use clap::Parser;
use std::process;
use tasm_runner::Cli;
use tasm_runner::{do_work, get_config, validate_args};

fn main() {
    let cli = Cli::parse();

    let compiler_dir_input = match validate_args(&cli) {
        Ok(dir) => dir,
        Err(e) => e.exit(),
    };

    let config = match get_config(cli, compiler_dir_input) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{e}");
            process::exit(-1);
        }
    };

    match do_work(config) {
        Ok(status) => match status.code() {
            Some(code) => process::exit(code),
            None => process::exit(-1),
        },
        Err(e) => {
            eprintln!("{e}");
            process::exit(-1);
        }
    }
}
