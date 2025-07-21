use std::path::PathBuf;

#[derive(Debug, PartialEq, Clone)]
pub struct BuildFile {
    pub filetype: BuildChoice,
    pub filepath: Option<PathBuf>,
    pub parent_dir: PathBuf,
    pub link_target_dir: Option<PathBuf>,
    pub base_image: Option<String>,
    pub custom_img_nm: Option<String>,
    pub build_args: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BuildChoice {
    Dockerfile,
    Makefile,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WhatWereBuilding {
    pub file: BuildFile,
    pub follow_link: bool,
}
