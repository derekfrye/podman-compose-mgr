use crate::helpers::podman_helper_fns;
use crate::rebuild::Image;
use crate::helpers::cmd_helper_fns as cmd;

use std::io::{self, Write};
use walkdir::DirEntry;
use std::cmp::max;

pub struct Result {
    pub user_entered_val: Option<String>,
    pub img: Image,
}


fn unroll_vecs_into_string(v: &Vec<&str> ,separtor: &str,termintor: &str) -> String {
    let mut x = String::new();
    for i in  v.iter() {
        x.push_str(&i.to_string());
        if i != &v[v.len() - 1] {
        x.push_str(separtor);}
        else {
            x.push_str(termintor);
        }
    }
    x
}

// moved from main, i've got to believe i'll use it for secrets and restartsvcs too
pub fn read_val_from_cmd_line_and_proceed(
    entry: &DirEntry,
    image: &str,
    build_args: &Vec<String>,
    container_name: &str,
    display_verbiage: &Vec<&str>,
    choices: &Vec<&str>,
)-> Result 
{

let mut x = Result {
    user_entered_val: None,
    img: Image {
        name: image.to_string(),
        container: container_name.to_string(),
        skipall_by_this_name: false,
    },
};

    
    
    // let refresh_static = format!("Refresh  from ? p/N/d/b/s/?: ");
    let refresh_static = unroll_vecs_into_string(display_verbiage, " ", "? ").push_str(
        &unroll_vecs_into_string(choices, "/", ": ")
    );
    // let refresh_prompt = format!(
    //     "Refresh {} from {}? p/N/d/b/s/?: ",
    //     image, docker_compose_pth_fmtted
    // );
    let refresh_prompt = unroll_vecs_into_string(display_verbiage, " ", " ").push_str(
        &unroll_vecs_into_string(choices, "/", ": ")
    );

    // if the prompt is too long, we need to shorten some stuff.
    // At a minimum, we'll display our 23 chars of "refresh ... from ?" stuff.
    // Then we divide remaining space equally between image name and path name.
    // We're not going to go less than 12 chars for path and image name, anything less feels like we're cutting too much off maybe.
    // This means total display chars is 23 + 12 + 12 = 47 at a min
    // if user has less than 47 wide, well then we'll have to let the terminal word-wrap.
    let term_width = cmd::get_terminal_display_width();
    // println!("term_width: {}", term_width);
    // println!("refresh_prompt len: {}", refresh_prompt.len());
    let mut docker_compose_pth_shortened = docker_compose_pth_fmtted.to_string();
    // let docker_compose_path_orig = docker_compose_pth_shortened.to_string();
    let mut image_shortened = image.to_string();
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
    // make sure this str matches str refresh_prompt above or the wrap logic above breaks
    // also, this same string is also used near end of this loop, make sure it matches there too
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
            pull_it(image);
            break;
        } else if input.eq_ignore_ascii_case("d") {
            println!("Image: {}", image);
            println!("Container name: {}", container_name);
            println!("Compose file: {}", docker_compose_pth_fmtted);
            println!(
                "Created: {}",
                format_time_ago(
                    podman_helper_fns::get_podman_image_upstream_create_time(image).unwrap()
                )
            );
            println!(
                "Pulled: {}",
                format_time_ago(podman_helper_fns::get_podman_ondisk_modify_time(image).unwrap())
            );
            println!(
                "Dockerfile exists: {}",
                cmd::dockerfile_exists_and_readable(
                    &entry
                        .path()
                        .parent()
                        .unwrap()
                        .join("Dockerfile")
                        .to_path_buf()
                )
            );
            print!(
                "Refresh {} from {}? p/N/d/b/s/?: ",
                image_shortened, docker_compose_pth_shortened
            );
        } else if input.eq_ignore_ascii_case("?") {
            println!("p = Pull image from upstream.");
            println!("N = Do nothing, skip this image.");
            println!("d = Display info (image name, docker-compose.yml path, upstream img create date, and img on-disk modify date).");
            println!("b = Build image from the Dockerfile residing in same path as the docker-compose.yml.");
            println!("s = Skip all subsequent images with this same name (regardless of container name).");
            println!("? = Display this help.");
            print!(
                "Refresh {} from {}? p/N/d/b/s/?: ",
                image_shortened, docker_compose_pth_shortened
            );
        } else if input.eq_ignore_ascii_case("b") {
            build_image_from_dockerfile(
                entry,
                image,
                build_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            );
            break;
        } else if input.eq_ignore_ascii_case("s") {
            let c = Image {
                name: image.to_string(),
                container: container_name.to_string(),
                skipall_by_this_name: true,
            };
            images_checked.push(c);
            break;
        } else {
            break;
        }
    }

    x
}