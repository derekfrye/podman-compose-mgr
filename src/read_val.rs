use crate::helpers::cmd_helper_fns as cmd;
use crate::rebuild::Image;

use std::io::{ self, Write };
// use walkdir::DirEntry;
use std::cmp::max;

pub struct Result {
    pub user_entered_val: Option<String>,
    pub img: Image,
    pub grammar: Vec<Grammar>,
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
pub struct Grammar {
    pub original_val_for_prompt: Option<String>,
    pub shortend_val_for_prompt: Option<String>,
    pub pos: u8,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub grammer_type: GrammerType,
    pub part_of_static_prompt: bool,
    pub display_at_all: bool,
}

impl Default for Grammar {
    fn default() -> Self {
        Grammar {
            original_val_for_prompt: None,
            shortend_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: None,
            grammer_type: GrammerType::Verbiage,
            part_of_static_prompt: false,
            display_at_all: false,
        }
    }
}

fn unroll_grammer_into_string(
    grammars: &Vec<Grammar>,
    excl_if_not_in_base_prompt: bool,
    use_shortened_val: bool
) -> String {
    let mut return_result = String::new();
    // lets loop through based on the position
    for grammar in grammars.iter() {
        if excl_if_not_in_base_prompt && !grammar.part_of_static_prompt {
            return_result.push_str(" ");
            continue;
        }
        if let Some(prefix) = &grammar.prefix {
            return_result.push_str(prefix);
        }

        if use_shortened_val && grammar.shortend_val_for_prompt.is_some() {
            return_result.push_str(grammar.shortend_val_for_prompt.as_ref().unwrap().as_str());
        } else if grammar.display_at_all {
            return_result.push_str(grammar.original_val_for_prompt.as_ref().unwrap().as_str());
        }

        if let Some(suffix) = &grammar.suffix {
            return_result.push_str(suffix);
        }
    }
    return_result
}

// moved from main, i've got to believe i'll use it for secrets and restartsvcs too
pub fn read_val_from_cmd_line_and_proceed(grammars: &mut Vec<Grammar>) -> Result {
    let container_name = grammars
        .iter()
        .find(|x| x.grammer_type == GrammerType::ContainerName)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap()
        .unwrap();
    let docker_compose_path = grammars
        .iter()
        .find(|x| x.grammer_type == GrammerType::DockerComposePath)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap();
    let image = grammars
        .iter()
        .find(|x| x.grammer_type == GrammerType::Image)
        .map(|f| f.original_val_for_prompt.clone())
        .unwrap()
        .unwrap();
    let mut return_result = Result {
        user_entered_val: None,
        img: Image {
            name: grammars
                .iter()
                .find(|x| x.grammer_type == GrammerType::Image)
                .map(|f| f.original_val_for_prompt.clone())
                .unwrap(),
            container: Some(container_name.clone()),
            skipall_by_this_name: false,
        },
        grammar: Vec::new(),
    };

    // let refresh_static = format!("Refresh  from ? p/N/d/b/s/?: ");
    //
    let refresh_static = unroll_grammer_into_string(grammars, true, false);
    let refresh_prompt = unroll_grammer_into_string(grammars, false, false);

    // if the prompt is too long, we need to shorten some stuff.
    // At a minimum, we'll display our 23 chars of "refresh ... from ?" stuff.
    // Then we divide remaining space equally between image name and path name.
    // We're not going to go less than 12 chars for path and image name, anything less feels like we're cutting too much off maybe.
    // This means total display chars is 23 + 12 + 12 = 47 at a min
    // if user has less than 47 wide, well then we'll have to let the terminal word-wrap.
    let term_width = cmd::get_terminal_display_width();
    // println!("term_width: {}", term_width);
    // println!("refresh_prompt len: {}", refresh_prompt.len());
    let mut docker_compose_pth_shortened = docker_compose_path.clone().unwrap();
    // let docker_compose_path_orig = docker_compose_pth_shortened.to_string();
    let mut image_shortened = image.clone();
    // let image_orig = image.to_string();
    // 1 char for a little buffer so it doesnt wrap after user input
    if refresh_prompt.len() > term_width - 1 {
        let truncated_symbols = "...";
        let mut max_avail_chars_for_image_and_path =
            max(term_width, 47) - refresh_static.len() - 2 * truncated_symbols.len() - 1;
        if max_avail_chars_for_image_and_path % 2 != 0 {
            max_avail_chars_for_image_and_path -= 1;
        }

        if docker_compose_pth_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            docker_compose_pth_shortened = format!(
                "...{}",
                docker_compose_pth_shortened[
                    docker_compose_pth_shortened.len() - max_avail_chars_for_image_and_path / 2..
                ].to_string()
            );
        }

        if image_shortened.len() > max_avail_chars_for_image_and_path / 2 {
            image_shortened = format!(
                "...{}",
                image_shortened[
                    image_shortened.len() - max_avail_chars_for_image_and_path / 2..
                ].to_string()
            );
        }
    }

    let docker_compose_grammar = Grammar {
        original_val_for_prompt: Some(docker_compose_path.clone().unwrap()),
        shortend_val_for_prompt: Some(docker_compose_pth_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammer_type: GrammerType::DockerComposePath,
        part_of_static_prompt: true,
        display_at_all: true,
    };

    let image_grammar = Grammar {
        original_val_for_prompt: Some(image.clone()),
        shortend_val_for_prompt: Some(image_shortened.clone()),
        pos: 0,
        prefix: None,
        suffix: None,
        grammer_type: GrammerType::Image,
        part_of_static_prompt: true,
        display_at_all: true,
    };
    return_result.grammar.push(docker_compose_grammar);
    return_result.grammar.push(image_grammar);

    let x = return_result.grammar
        .iter()
        .find(|x| x.grammer_type == GrammerType::DockerComposePath)
        .and_then(|x| x.shortend_val_for_prompt.clone());
    let z = return_result.grammar
        .iter()
        .find(|x| x.grammer_type == GrammerType::Image)
        .and_then(|x| x.shortend_val_for_prompt.clone());
    grammars.iter_mut().for_each(|y| {
        if y.grammer_type == GrammerType::DockerComposePath {
            y.shortend_val_for_prompt = x.clone();
        }
        if y.grammer_type == GrammerType::Image {
            y.shortend_val_for_prompt = z.clone();
        }
    });

    print!("{}", unroll_grammer_into_string(grammars, false, true));
    // make sure this str matches str refresh_prompt above or the wrap logic above breaks
    // also, this same string is also used near end of this loop, make sure it matches there too
    // TODO FIXME
    // print!("Refresh {} from {}? p/N/d/b/s/?: ", image_shortened, docker_compose_pth_shortened);

    loop {
        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        if input.eq_ignore_ascii_case("p") {
            // Pull the image using podman and stream the output
            // pull_it(image);
            return_result.user_entered_val = Some("p".to_string());
            break;
        } else if input.eq_ignore_ascii_case("d") {
            return_result.user_entered_val = Some("d".to_string());
            break;
        } else if input.eq_ignore_ascii_case("?") {
            return_result.user_entered_val = Some("?".to_string());
            break;
        } else if input.eq_ignore_ascii_case("b") {
            return_result.user_entered_val = Some("b".to_string());
            break;
        } else if input.eq_ignore_ascii_case("s") {
            return_result.user_entered_val = Some("s".to_string());
            let c = Image {
                name: Some(image.to_string()),
                container: Some(container_name.to_string()),
                skipall_by_this_name: true,
            };
            return_result.img = c;
            break;
        } else {
            break;
        }
    }

    return_result
}
