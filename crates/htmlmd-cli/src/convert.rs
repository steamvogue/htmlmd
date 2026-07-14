// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use htmlmd_core::{convert, ConversionOptions, ConversionResult};
use sha2::{Digest, Sha256};

use crate::cli::{Cli, OutputPolicyArg};
use crate::error::{CliError, Result};

/// A single input/output pair.
pub struct Job {
    pub input_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub html: String,
    pub base_dir: Option<PathBuf>,
}

/// A recorded result for a single job.
pub struct JobRecord {
    pub input_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub result: ConversionResult,
    pub input_hash: String,
    pub output_hash: String,
    pub skipped: bool,
    pub changed: Option<bool>,
}

/// Resolve all inputs into jobs.
pub fn read_jobs(cli: &Cli) -> Result<Vec<Job>> {
    if cli.inputs.is_empty() || (cli.inputs.len() == 1 && cli.inputs[0] == "-") {
        let mut html = String::new();
        io::stdin().read_to_string(&mut html)?;
        return Ok(vec![Job {
            input_path: None,
            output_path: cli.output.clone(),
            html,
            base_dir: None,
        }]);
    }

    // Reject -o with multiple inputs unless --output-dir is used.
    if cli.output.is_some() && cli.output_dir.is_none() && total_input_count(&cli.inputs)? > 1 {
        return Err(CliError::OutputRequired);
    }

    let mut jobs = Vec::new();
    for input in &cli.inputs {
        resolve_input(input, cli, &mut jobs)?;
    }
    Ok(jobs)
}

fn total_input_count(inputs: &[String]) -> Result<usize> {
    let mut count = 0;
    for input in inputs {
        if is_glob(input) {
            for entry in glob::glob(input)? {
                let path = entry?;
                if path.is_file() {
                    count += 1;
                }
            }
        } else {
            let path = PathBuf::from(input);
            if path.is_dir() {
                // approximate: will be validated later
                count += 1;
            } else {
                count += 1;
            }
        }
    }
    Ok(count)
}

fn resolve_input(input: &str, cli: &Cli, jobs: &mut Vec<Job>) -> Result<()> {
    if is_glob(input) {
        for entry in glob::glob(input)? {
            let path = entry?;
            if path.is_file() {
                add_file_job(&path, cli, jobs)?;
            } else if path.is_dir() && cli.recursive {
                collect_directory(&path, cli, jobs)?;
            }
        }
        return Ok(());
    }

    let path = PathBuf::from(input);
    if path.is_dir() {
        if !cli.recursive {
            return Err(CliError::Config(format!(
                "{} is a directory; use --recursive to process directories",
                path.display()
            )));
        }
        collect_directory(&path, cli, jobs)?;
    } else {
        add_file_job(&path, cli, jobs)?;
    }
    Ok(())
}

fn is_glob(s: &str) -> bool {
    s.contains(['*', '?', '['])
}

fn collect_directory(dir: &Path, cli: &Cli, jobs: &mut Vec<Job>) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && is_html(path) {
            add_file_job(path, cli, jobs)?;
        }
    }
    Ok(())
}

fn is_html(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
        .unwrap_or(false)
}

fn add_file_job(path: &Path, cli: &Cli, jobs: &mut Vec<Job>) -> Result<()> {
    let html = read_html(path, cli.encoding.as_deref())?;
    let base_dir = if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        path.parent().map(Path::to_path_buf)
    };
    jobs.push(Job {
        input_path: Some(path.to_path_buf()),
        output_path: None,
        html,
        base_dir,
    });
    Ok(())
}

fn read_html(path: &Path, encoding_override: Option<&str>) -> Result<String> {
    let bytes = fs::read(path)?;

    // BOM detection
    let (cow, used_encoding, had_errors) = if let Some(label) = encoding_override {
        let enc = encoding_rs::Encoding::for_label(label.as_bytes())
            .ok_or_else(|| CliError::Config(format!("unknown encoding: {label}")))?;
        enc.decode(&bytes)
    } else if bytes.starts_with(b"\xef\xbb\xbf") {
        encoding_rs::UTF_8.decode(&bytes)
    } else if bytes.starts_with(b"\xff\xfe") {
        encoding_rs::UTF_16LE.decode(&bytes)
    } else if bytes.starts_with(b"\xfe\xff") {
        encoding_rs::UTF_16BE.decode(&bytes)
    } else {
        encoding_rs::UTF_8.decode(&bytes)
    };

    if had_errors {
        return Err(CliError::Config(format!(
            "could not decode {} as {} (invalid byte sequence)",
            path.display(),
            used_encoding.name()
        )));
    }
    Ok(cow.into_owned())
}

/// Compute final output paths for every job.
pub fn assign_output_paths(jobs: &mut [Job], cli: &Cli) -> Result<()> {
    if jobs.len() == 1 && jobs[0].input_path.is_none() {
        // stdin: honor -o if provided, else stdout.
        return Ok(());
    }

    // A single file without --output-dir writes to stdout (or -o if given).
    if jobs.len() == 1 && cli.output_dir.is_none() {
        if let Some(out) = &cli.output {
            jobs[0].output_path = Some(out.clone());
        }
        return Ok(());
    }

    let output_dir = cli.output_dir.as_ref().ok_or(CliError::OutputRequired)?;
    fs::create_dir_all(output_dir)?;

    for job in jobs.iter_mut() {
        let Some(input) = &job.input_path else {
            continue;
        };
        let out = if cli.mirror {
            let base = job.base_dir.as_deref().unwrap_or_else(|| Path::new("."));
            let relative = input.strip_prefix(base).unwrap_or(input.as_path());
            output_dir.join(relative).with_extension("md")
        } else {
            let name = input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            output_dir.join(format!("{name}.md"))
        };
        job.output_path = Some(out);
    }

    Ok(())
}

/// Convert a single job.
pub fn convert_job(job: &Job, options: &ConversionOptions) -> Result<ConversionResult> {
    convert(&job.html, options).map_err(Into::into)
}

/// Execute one job: possibly convert, check existing output, write, preserve timestamps.
pub fn execute_job(job: &Job, options: &ConversionOptions, cli: &Cli) -> Result<JobRecord> {
    let result = convert_job(job, options)?;

    let input_hash = hash_bytes(job.html.as_bytes());
    let output_hash = hash_bytes(result.markdown.as_bytes());

    let mut record = JobRecord {
        input_path: job.input_path.clone(),
        output_path: job.output_path.clone(),
        result,
        input_hash,
        output_hash,
        skipped: false,
        changed: None,
    };

    let Some(out) = &job.output_path else {
        if !cli.dry_run && !cli.check {
            io::stdout().write_all(record.result.markdown.as_bytes())?;
        }
        return Ok(record);
    };

    if cli.check {
        record.changed = Some(check_output(out, &record.result.markdown, cli.diff)?);
        return Ok(record);
    }

    match cli.output_policy {
        OutputPolicyArg::SkipExisting if out.exists() => {
            record.skipped = true;
            return Ok(record);
        }
        OutputPolicyArg::FailIfExists if out.exists() => {
            return Err(CliError::Config(format!(
                "output file already exists (fail-if-exists): {}",
                out.display()
            )));
        }
        _ => {}
    }

    if !cli.dry_run {
        write_output(out, &record.result.markdown, cli.atomic)?;
        if cli.preserve_timestamps {
            if let Some(input) = &job.input_path {
                copy_timestamps(input, out)?;
            }
        }
    }

    Ok(record)
}

fn check_output(path: &Path, new: &str, show_diff: bool) -> Result<bool> {
    let changed = if path.exists() {
        let existing = fs::read_to_string(path)?;
        let differs = existing != new;
        if differs && show_diff {
            emit_diff(&existing, new, path);
        }
        differs
    } else {
        true
    };
    Ok(changed)
}

fn emit_diff(old: &str, new: &str, path: &Path) {
    use similar::{ChangeTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    eprintln!("--- {}\n+++ (generated)", path.display());
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        eprint!("{sign}{change}");
    }
}

fn write_output(path: &Path, content: &str, atomic: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if atomic {
        let mut tmp = tempfile::NamedTempFile::new_in(
            path.parent().unwrap_or_else(|| Path::new(".")),
        )?;
        tmp.write_all(content.as_bytes())?;
        tmp.persist(path).map_err(|e| CliError::Io(e.error))?;
    } else {
        fs::write(path, content)?;
    }
    Ok(())
}

fn copy_timestamps(from: &Path, to: &Path) -> Result<()> {
    let meta = fs::metadata(from)?;
    let atime = filetime::FileTime::from_last_access_time(&meta);
    let mtime = filetime::FileTime::from_last_modification_time(&meta);
    filetime::set_file_times(to, atime, mtime)?;
    Ok(())
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Write a JSON manifest of the conversion run.
pub fn write_manifest(path: &Path, records: &[JobRecord]) -> Result<()> {
    #[derive(serde::Serialize)]
    struct FileEntry {
        input: Option<String>,
        output: Option<String>,
        title: Option<String>,
        description: Option<String>,
        canonical_url: Option<String>,
        input_hash: String,
        output_hash: String,
        skipped: bool,
        changed: Option<bool>,
        diagnostics: Vec<String>,
    }

    #[derive(serde::Serialize)]
    struct Manifest {
        files: Vec<FileEntry>,
        changed_count: usize,
        skipped_count: usize,
        error_count: usize,
    }

    let mut changed = 0;
    let mut skipped = 0;
    let mut errors = 0;

    let files: Vec<FileEntry> = records
        .iter()
        .map(|r| {
            if r.skipped {
                skipped += 1;
            }
            if r.changed == Some(true) {
                changed += 1;
            }
            if r.result.has_errors() {
                errors += 1;
            }
            FileEntry {
                input: r.input_path.as_ref().map(|p| p.display().to_string()),
                output: r.output_path.as_ref().map(|p| p.display().to_string()),
                title: r.result.title.clone(),
                description: r.result.description.clone(),
                canonical_url: r.result.canonical_url.clone(),
                input_hash: r.input_hash.clone(),
                output_hash: r.output_hash.clone(),
                skipped: r.skipped,
                changed: r.changed,
                diagnostics: r
                    .result
                    .diagnostics
                    .iter()
                    .map(|d| format!("{:?}: {}", d.kind, d.message))
                    .collect(),
            }
        })
        .collect();

    let manifest = Manifest {
        files,
        changed_count: changed,
        skipped_count: skipped,
        error_count: errors,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(path, json)?;
    Ok(())
}
