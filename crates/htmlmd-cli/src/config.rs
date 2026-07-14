// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::{Path, PathBuf};

use figment::{
    Figment,
    providers::{Env, Format, Json, Toml},
};
use htmlmd_core::ConversionOptions;

use crate::cli::Cli;
use crate::error::{CliError, Result};

/// Load configuration from discovered files, explicit file, environment, and CLI.
pub fn load_config(cli: &Cli) -> Result<ConversionOptions> {
    let mut figment = Figment::new();

    // Lowest precedence: discovered project and user config files.
    for path in discover_configs() {
        figment = merge_file(figment, &path)?;
    }

    // Middle: explicit --config file.
    if let Some(path) = &cli.config {
        if !path.exists() {
            return Err(CliError::Config(format!(
                "configuration file not found: {}",
                path.display()
            )));
        }
        figment = merge_file(figment, path)?;
    }

    // Higher: environment variables.
    figment = figment.merge(Env::prefixed("HTMLMD_").split("__"));

    // Highest: explicit CLI flags.
    let mut opts: ConversionOptions = figment.extract().map_err(|e| CliError::Config(e.to_string()))?;
    apply_cli_overrides(&mut opts, cli);
    opts.apply_profile_defaults();
    opts.validate().map_err(|e| CliError::Config(e.to_string()))?;
    Ok(opts)
}

fn merge_file(figment: Figment, path: &Path) -> Result<Figment> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "toml" => Ok(figment.merge(Toml::file(path))),
        "json" => Ok(figment.merge(Json::file(path))),
        _ => Err(CliError::Config(format!(
            "unsupported config format: {}. Use .toml or .json",
            path.display()
        ))),
    }
}

fn discover_configs() -> Vec<PathBuf> {
    let mut discovered = Vec::new();

    // User-global config: $CONFIG_DIR/htmlmd/config.toml
    if let Some(config_dir) = dirs::config_dir() {
        let global = config_dir.join("htmlmd").join("config.toml");
        if global.exists() {
            discovered.push(global);
        }
    }

    // Project-local config: .htmlmd.toml in the current working directory.
    let local = PathBuf::from(".htmlmd.toml");
    if local.exists() {
        discovered.push(local);
    }

    discovered
}

fn apply_cli_overrides(opts: &mut ConversionOptions, cli: &Cli) {
    if let Some(p) = cli.profile {
        opts.profile = p.into();
    }
    if let Some(h) = cli.heading_style {
        opts.render.heading_style = h.into();
    }
    if let Some(b) = cli.bullet {
        opts.render.bullet_marker = b.into();
    }
    if let Some(l) = cli.link_style {
        opts.render.link_style = l.into();
    }
    if let Some(c) = cli.code_fence {
        opts.render.code_fence = c.into();
    }
    if let Some(h) = cli.hr_style {
        opts.render.hr_style = h.into();
    }
    if let Some(b) = cli.br_style {
        opts.render.hard_break_style = b.into();
    }
    if let Some(tags) = &cli.skip_tags {
        opts.cleanup.remove_tags = tags.clone();
    }
    if let Some(sel) = &cli.remove_selectors {
        opts.cleanup.remove_selectors = sel.clone();
    }
    if let Some(sel) = &cli.unwrap_selectors {
        opts.cleanup.unwrap_selectors = sel.clone();
    }
    if let Some(sel) = &cli.keep_only_selectors {
        opts.cleanup.keep_only_selectors = sel.clone();
    }
    if let Some(sel) = &cli.extract_selector {
        opts.cleanup.extract_selector = Some(sel.clone());
    }
    if let Some(base) = &cli.base_url {
        opts.cleanup.base_url = Some(base.clone());
    }
    if let Some(r) = cli.remove_tracking_params {
        opts.cleanup.remove_tracking_params = r;
    }
    if cli.strict {
        opts.strict = true;
    }
    if cli.metadata_title {
        opts.cleanup.metadata.title = true;
    }
    if cli.metadata_description {
        opts.cleanup.metadata.description = true;
    }
    if cli.metadata_canonical_url {
        opts.cleanup.metadata.canonical_url = true;
    }
    if let Some(r) = cli.reference_placement {
        opts.render.reference_placement = r.into();
    }
    if let Some(i) = cli.image_mode {
        opts.cleanup.image_mode = i.into();
    }
}
