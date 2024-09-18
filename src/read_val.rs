use crate::helpers::cmd_helper_fns as cmd;
use crate::rebuild::Image;

use std::io::{self, Write};
// use walkdir::DirEntry;
use std::cmp::max;

pub struct Result {
    pub user_entered_val: Option<String>,
    pub img: Image,
    pub gm: Vec<Grammer>,
}

#[derive(Debug, PartialEq)]
pub enum GrammerType {
    Verbiage,
    UserChoice,
    Image,
    DockerComposePath,
    // BuildArgs,
    ContainerName,
}

#[derive(Debug, PartialEq)]
pub struct Grammer {
    pub original_val_for_prompt: Option<String>,
    pub shortend_val_for_prompt: Option<String>,
    pub pos: u8,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub grammer_type: GrammerType,
    pub include_in_base_string: bool,
    pub display_at_all: bool,
}

impl Default for Grammer {
    fn default() -> Self {
        Grammer {
            original_val_for_prompt: None,
            shortend_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: None,
            grammer_type: GrammerType::Verbiage,
            include_in_base_string: false,
            display_at_all: false,
        }
    }
}

// impl Grammer {
//     fn new() -> Self {
//         Self::default()
//     }
// }

// fn unroll_vecs_into_string(v: &Vec<&str> ,separtor: &str,termintor: &str) -> String {
//     let mut x = String::new();
//     for i in  v.iter() {
//         x.push_str(&i.to_string());
//         if i != &v[v.len() - 1] {
//         x.push_str(separtor);}
//         else {
//             x.push_str(termintor);
//         }
//     }
//     x
// }

fn unroll_grammer_into_string(v: &Vec<Grammer>, excl_if_not_in_base_prompt: bool) -> String {
    let mut x = String::new();
    // lets loop through based on the pos
    // let mut t = v.clone();
    // t.sort_by(|a, b| a.pos.cmp(&b.pos));
    for i in v.iter() {
        if excl_if_not_in_base_prompt && !i.include_in_base_string {
            x.push_str(" ");
            continue;
        }
        if let Some(prefix) = &i.prefix {
            x.push_str(prefix);
        }
        x.push_str(i.original_val_for_prompt.as_ref().unwrap().as_str());
        if let Some(suffix) = &i.suffix {
            x.push_str(suffix);
        }
    }
    x
}

// moved from main, i've got to believe i'll use it for secrets and restartsvcs too
pub fn read_val_from_cmd_line_and_proceed(
    // entry: &DirEntry,
    // image: &str,
    // build_args: &Vec<String>,
    // container_name: &str,
    // display_verbiage: &Vec<&str>,
    // choices: &Vec<&str>,
    grammers: &Vec<Grammer>,
) -> Result {
    // let nm = grammers.iter().find(|x| x.grammer_type == GrammerType::Image).map(|f| f.original_val_for_prompt).unwrap();
    let containera = grammers
        .iter()
        .find(|x| x.grammer_type == GrammerType::ContainerName)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap()
        .unwrap();
    let cmp_path = grammers
        .iter()
        .find(|x| x.grammer_type == GrammerType::DockerComposePath)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap();
    let iiimmmggg = grammers
        .iter()
        .find(|x| x.grammer_type == GrammerType::Image)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap()
        .unwrap();
    let mut x = Result {
        user_entered_val: None,
        img: Image {
            name: grammers
                .iter()
                .find(|x| x.grammer_type == GrammerType::Image)
                .map(|f| f.original_val_for_prompt.clone())
                .unwrap(),
            container: Some(containera.clone()),
            skipall_by_this_name: false,
        },
        gm: Vec::new(),
    };

    // let refresh_static = format!("Refresh  from ? p/N/d/b/s/?: ");
    let refresh_static = unroll_grammer_into_string(grammers, true);

    // let refresh_prompt = format!(
    //     "Refresh {} from {}? p/N/d/b/s/?: ",
    //     image, docker_compose_pth_fmtted
    // );
    let refresh_prompt = unroll_grammer_into_string(grammers, false);

    // if the prompt is too long, we need to shorten some stuff.
    // At a minimum, we'll display our 23 chars of "refresh ... from ?" stuff.
    // Then we divide remaining space equally between image name and path name.
    // We're not going to go less than 12 chars for path and image name, anything less feels like we're cutting too much off maybe.
    // This means total display chars is 23 + 12 + 12 = 47 at a min
    // if user has less than 47 wide, well then we'll have to let the terminal word-wrap.
    let term_width = cmd::get_terminal_display_width();
    // println!("term_width: {}", term_width);
    // println!("refresh_prompt len: {}", refresh_prompt.len());
    let mut docker_compose_pth_shortened = cmp_path.clone().unwrap();
    // let docker_compose_path_orig = docker_compose_pth_shortened.to_string();
    let mut image_shortened = iiimmmggg.clone();
    // let image_orig = image.to_string();
    // 1 char for a little buffer so it doesnt wrap after user input
    if refresh_prompt.len() > term_width - 1 {
        let truncated_symbols = "...";
        let mut max_avail_chars_for_image_and_path =
            max(term_width, 47) - refresh_static.len() - (2 * truncated_symbols.len()) - 1;
        if max_avail_chars_for_image_and_path % 2 != 0 {
            max_avail_chars_for_image_and_path -= 1;
        }

        if docker_compose_pth_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            docker_compose_pth_shortened = format!(
                "...{}",
                docker_compose_pth_shortened
                    [docker_compose_pth_shortened.len() - max_avail_chars_for_image_and_path / 2..]
                    .to_string()
            );
        }

        if image_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            image_shortened = format!(
                "...{}",
                image_shortened[image_shortened.len() - max_avail_chars_for_image_and_path / 2..]
                    .to_string()
            );
        }
    }

    let gmrr = Grammer {
        original_val_for_prompt: Some(cmp_path.clone().unwrap()),
        shortend_val_for_prompt: Some(docker_compose_pth_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammer_type: GrammerType::DockerComposePath,
        include_in_base_string: true,
        display_at_all: true,
    };

    let gmrr1 = Grammer {
        original_val_for_prompt: Some(iiimmmggg.clone()),
        shortend_val_for_prompt: Some(image_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammer_type: GrammerType::Image,
        include_in_base_string: true,
        display_at_all: true,
    };
    x.gm.push(gmrr);
    x.gm.push(gmrr1);

    // make sure this str matches str refresh_prompt above or the wrap logic above breaks
    // also, this same string is also used near end of this loop, make sure it matches there too
    // TODO FIXME
    print!(
        "Refresh {} from {}? p/N/d/b/s/?: ",
        image_shortened, docker_compose_pth_shortened
    );
    loop {
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        if input.eq_ignore_ascii_case("p") {
            // Pull the image using podman and stream the output
            // pull_it(image);
            x.user_entered_val = Some("p".to_string());
            break;
        } else if input.eq_ignore_ascii_case("d") {
            x.user_entered_val = Some("d".to_string());
            break;
        } else if input.eq_ignore_ascii_case("?") {
            x.user_entered_val = Some("?".to_string());
            break;
        } else if input.eq_ignore_ascii_case("b") {
            x.user_entered_val = Some("b".to_string());
            break;
        } else if input.eq_ignore_ascii_case("s") {
            x.user_entered_val = Some("s".to_string());
            let c = Image {
                name: Some(iiimmmggg.to_string()),
                container: Some(containera.to_string()),
                skipall_by_this_name: true,
            };
            x.img = c;
            // images_checked.push(c);
            break;
        } else {
            break;
        }
    }

    x
}
