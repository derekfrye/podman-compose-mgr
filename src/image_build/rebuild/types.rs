#[derive(Debug, PartialEq)]
pub struct Image {
    pub name: Option<String>,
    pub container: Option<String>,
    pub skipall_by_this_name: bool,
}

#[derive(Copy, Clone)]
pub struct RebuildSelection<'a> {
    pub image: &'a str,
    pub container: &'a str,
}

impl<'a> RebuildSelection<'a> {
    pub fn new(image: &'a str, container: &'a str) -> Self {
        Self { image, container }
    }
}
