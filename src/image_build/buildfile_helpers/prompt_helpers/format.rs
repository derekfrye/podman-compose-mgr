use super::super::grammar_helpers::make_choice_grammar;
use super::validate::build_prompt_fragment;
use crate::image_build::buildfile_types::{BuildChoice, BuildFile};
use crate::read_interactive_input::{GrammarFragment, GrammarType};

/// Setup prompts for buildfile selection
#[must_use]
pub fn setup_prompts(files: &[BuildFile]) -> (Vec<GrammarFragment>, Vec<&str>, bool) {
    let mut prompt_grammars: Vec<GrammarFragment> = vec![];
    let mut user_choices: Vec<&str> = vec![];

    let buildfile = &files[0];
    let are_there_multiple_files = files.iter().filter(|x| x.filepath.is_some()).count() > 1;

    if are_there_multiple_files {
        let grm1 = GrammarFragment {
            original_val_for_prompt: Some("Prefer Dockerfile or Makefile?".to_string()),
            ..Default::default()
        };
        prompt_grammars.push(grm1);

        user_choices = vec!["D", "M", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(
            &user_choices,
            u8::try_from(prompt_grammars.len()).unwrap_or(255),
            None,
        ));
    } else if buildfile.link_target_dir.is_some() {
        let t = make_build_prompt_grammar(buildfile);
        user_choices = vec!["1", "2", "d", "?"];
        prompt_grammars.extend(make_choice_grammar(
            &user_choices,
            u8::try_from(t.len()).unwrap_or(255),
            None,
        ));
    }

    (prompt_grammars, user_choices, are_there_multiple_files)
}

/// Display information about available buildfiles
///
/// # Panics
/// Panics if buildfile paths contain invalid UTF-8 characters or if filepath is None
pub fn handle_display_info(
    files: &[BuildFile],
    buildfile: &BuildFile,
    user_choices: &[&str],
    are_there_multiple_files: bool,
) {
    for f in files
        .iter()
        .filter(|f| f.filetype == BuildChoice::Dockerfile)
    {
        let dockerfile = &f.filepath.as_ref().unwrap().to_str().unwrap();
        println!("Dockerfile: {dockerfile}");
    }
    for f in files.iter().filter(|f| f.filetype == BuildChoice::Makefile) {
        let makefile = &f.filepath.as_ref().unwrap().to_str().unwrap();
        println!("Makefile: {makefile}");
    }

    println!("Choices:");

    if are_there_multiple_files
        && !user_choices.is_empty()
        && user_choices.contains(&"D")
        && user_choices.contains(&"M")
    {
        println!("D = Build an image from a Dockerfile.");
        println!("M = Execute `make` on a Makefile.");
    } else {
        if buildfile.link_target_dir.is_some() {
            let location1 = match buildfile.link_target_dir.as_ref().unwrap().parent() {
                Some(parent) => parent.display().to_string(),
                None => buildfile
                    .link_target_dir
                    .as_ref()
                    .unwrap()
                    .display()
                    .to_string(),
            };
            println!("1 = Set build working dir to:\n\t{location1}");
        }

        let location2 = buildfile.parent_dir.display();
        println!("2 = Set build working dir to:\n\t{location2}");
    }
    println!("d = Display info about Dockerfile and/or Makefile.");
    println!("? = Display this help.");
}

/// Create grammar fragments for build prompts
///
/// # Panics
/// Panics if buildfile contains invalid link target directory paths
#[must_use]
pub fn make_build_prompt_grammar(buildfile: &BuildFile) -> Vec<GrammarFragment> {
    vec![
        build_prompt_fragment(
            format!(
                "Run `{}` in (1):",
                match buildfile.filetype {
                    BuildChoice::Dockerfile => "podman build",
                    BuildChoice::Makefile => "make",
                }
            ),
            0,
            None,
            None,
        ),
        build_prompt_fragment(
            buildfile
                .link_target_dir
                .clone()
                .unwrap()
                .display()
                .to_string(),
            1,
            Some(GrammarType::FileName),
            None,
        ),
        build_prompt_fragment(", or (2):".to_string(), 2, None, None),
        build_prompt_fragment(
            buildfile.parent_dir.display().to_string(),
            3,
            Some(GrammarType::FileName),
            None,
        ),
        build_prompt_fragment("?".to_string(), 4, None, Some(" ")),
    ]
}
