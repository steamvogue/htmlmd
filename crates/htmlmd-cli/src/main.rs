// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::{self, Write};
use std::process;

use clap::Parser;
use htmlmd_core::ConversionOptions;
use rayon::prelude::*;

mod cli;
mod config;
mod convert;
mod error;
mod mappings;

use cli::Cli;
use config::load_config;
use convert::{JobRecord, assign_output_paths, execute_job, read_jobs, write_manifest};
use error::{CliError, Result};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    if let Err(e) = run() {
        let _ = writeln!(io::stderr(), "htmlmd: {e}");
        process::exit(e.exit_code());
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.print_default_config {
        let default = ConversionOptions::default();
        let toml = toml::to_string_pretty(&default).map_err(|e| CliError::Config(e.to_string()))?;
        println!("{toml}");
        return Ok(());
    }

    let options = load_config(&cli)?;

    if cli.print_effective_config {
        let json =
            serde_json::to_string_pretty(&options).map_err(|e| CliError::Config(e.to_string()))?;
        println!("{json}");
        return Ok(());
    }

    if cli.dry_run && cli.check {
        return Err(CliError::Config(
            "--dry-run and --check are mutually exclusive".to_string(),
        ));
    }

    let mut jobs = read_jobs(&cli)?;
    assign_output_paths(&mut jobs, &cli)?;

    if cli.dry_run && !cli.quiet {
        eprintln!("htmlmd: dry run, no output will be written");
    }

    let pool = build_thread_pool(cli.jobs)?;
    let records: Vec<Result<JobRecord>> = pool.install(|| {
        jobs.into_par_iter()
            .map(|job| {
                let record = execute_job(&job, &options, &cli)?;
                if !cli.quiet && !cli.check {
                    if let Some(path) = &job.input_path {
                        if record.skipped {
                            eprintln!("htmlmd: skipped {} (output exists)", path.display());
                        } else if job.output_path.is_some() {
                            eprintln!("htmlmd: converted {}", path.display());
                        }
                    }
                }
                Ok(record)
            })
            .collect()
    });

    let mut conversion_errors = 0usize;
    let mut config_errors = 0usize;
    let mut changed = 0usize;
    let mut collected = Vec::with_capacity(records.len());

    for res in records {
        match res {
            Ok(record) => {
                if record.result.has_errors() {
                    conversion_errors += 1;
                }
                if record.changed == Some(true) {
                    changed += 1;
                }
                collected.push(record);
            }
            Err(e) => {
                eprintln!("htmlmd: {e}");
                if matches!(e, CliError::Config(_) | CliError::OutputRequired) {
                    config_errors += 1;
                } else {
                    conversion_errors += 1;
                }
            }
        }
    }

    if let Some(manifest_path) = &cli.manifest {
        write_manifest(manifest_path, &collected)?;
    }

    if cli.check && changed > 0 {
        return Err(CliError::Config(format!(
            "--check failed: {changed} output(s) changed"
        )));
    }

    if config_errors > 0 {
        return Err(CliError::Config(format!(
            "{config_errors} config/output error(s) occurred"
        )));
    }

    if conversion_errors > 0 {
        return Err(CliError::Conversion(htmlmd_core::Error::Other(format!(
            "{conversion_errors} conversion(s) failed"
        ))));
    }

    Ok(())
}

fn build_thread_pool(jobs: Option<usize>) -> Result<rayon::ThreadPool> {
    let mut builder = rayon::ThreadPoolBuilder::new();
    if let Some(j) = jobs {
        if j > 0 {
            builder = builder.num_threads(j);
        }
    }
    builder.build().map_err(|e| CliError::Config(e.to_string()))
}
