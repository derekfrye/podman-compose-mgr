use clap::builder::Str;
use clap::{Parser, ValueEnum};
// use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::fs;

pub fn args_checks() -> Args {
    let xx = Args::parse();
   
 xx
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the directory to traverse
    #[arg(short = 'p', long, value_name = "PATH", default_value = ".", value_parser = check_readable_dir)]
    pub path: String,
    /// Mode to run the program in
    #[arg(short = 'm', long, default_value = "Rebuild", value_parser = clap::value_parser!(Mode))]
    pub mode: Mode,
    /// Optional secrets file path, must be readable if supplied
    #[arg(short = 's', long, value_name = "SECRETS_FILE", value_parser = check_readable)]
    pub secrets_file: Option<String>,
    /// Optional verbose flag
    #[arg(short, long)]
    pub verbose: bool,
}



/// Enumeration of possible modes
#[derive(Clone, ValueEnum, Debug)]
pub enum Mode {
    Rebuild,
    Secrets,
}

fn check_readable(file: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(file);
    if path.is_file() && fs::metadata(&path).is_ok() && fs::File::open(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The file '{}' is not readable", file))
    }
}

fn check_readable_dir(dir: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(dir);
    if path.is_dir() && fs::metadata(&path).is_ok() && fs::read_dir(&path).is_ok() {
        Ok(path)
    } else {
        Err(format!("The dir '{}' is not readable", dir))
    }
}