#[derive(Default)]
pub struct DirInfo {
    pub dockerfiles: Vec<std::path::PathBuf>,
    pub makefiles: Vec<std::path::PathBuf>,
    pub compose_files: Vec<ComposeInfo>,
    pub container_files: Vec<ContainerInfo>,
}

pub struct ComposeInfo {
    pub first_image: Option<String>,
}

pub struct ContainerInfo {
    pub path: std::path::PathBuf,
}
