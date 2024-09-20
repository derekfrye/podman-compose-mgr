use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

pub fn args_checks() -> Args {
    let xx = Args::parse();
    xx
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Search path for docker-compose files
    #[arg(short = 'p', long, value_name = "PATH", default_value = ".", value_parser = check_readable_dir)]
    pub path: PathBuf,
    /// rebuild = pull latest docker.io images and rebuild custom images, secrets = refresh secrets files (not impl yet)
    #[arg(short = 'm', long, default_value = "Rebuild", value_parser = clap::value_parser!(Mode))]
    pub mode: Mode,
    /// Optional secrets file path, must be readable if supplied (not impl yet)
    #[arg(short = 's', long, value_name = "SECRETS_FILE", value_parser = check_readable)]
    pub secrets_file: Option<PathBuf>,
    /// Print extra stuff
    #[arg(short, long)]
    pub verbose: bool,
    /// Regex pattern(s) to exclude paths, e.g., docker/archive or [^\.]+/archive
    #[arg(short, long)]
    pub exclude_path_patterns: Vec<String>,
    #[arg(short, long)]
    pub build_args: Vec<String>,
}

/// Enumeration of possible modes
#[derive(Clone, ValueEnum, Debug)]
pub enum Mode {
    Rebuild,
    Secrets,
    RestartSvcs,
}

fn check_readable(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    if path.is_file() && fs::metadata(&path).is_ok() && fs::File::open(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The file '{}' is not readable.", file))
    }
}

fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    if path.is_dir() && fs::metadata(&path).is_ok() && fs::read_dir(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The dir '{}' is not readable.", dir))
    }
}
