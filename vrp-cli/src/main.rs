//! A command line interface to *Vehicle Routing Problem* solver.
//!

#[cfg(not(target_arch = "wasm32"))]
mod commands;

fn main() {
    cli::run_app()
}

#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use super::commands::import::{get_import_app, run_import};
    use super::commands::solve::{get_solve_app, run_solve};
    use crate::commands::check::{get_check_app, run_check};
    use crate::commands::create_write_buffer;
    use crate::commands::generate::{get_generate_app, run_generate};
    use clap::{crate_version, App};
    use std::process;

    pub fn run_app() {
        let matches = App::new("Vehicle Routing Problem Solver")
            .version(crate_version!())
            .author("Ilya Builuk <ilya.builuk@gmail.com>")
            .about("A command line interface to Vehicle Routing Problem solver")
            .subcommand(get_solve_app())
            .subcommand(get_import_app())
            .subcommand(get_check_app())
            .subcommand(get_generate_app())
            .get_matches();

        if let Err(err) = match matches.subcommand() {
            ("solve", Some(solve_matches)) => run_solve(solve_matches, create_write_buffer),
            ("import", Some(import_matches)) => run_import(import_matches),
            ("check", Some(check_matches)) => run_check(check_matches),
            ("generate", Some(generate_matches)) => run_generate(generate_matches),
            ("", None) => {
                eprintln!("no subcommand was used. Use -h to print help information.");
                process::exit(1);
            }
            _ => unreachable!(),
        } {
            eprintln!("{}", err);
            process::exit(1)
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod cli {
    pub fn run_app() {}
}
