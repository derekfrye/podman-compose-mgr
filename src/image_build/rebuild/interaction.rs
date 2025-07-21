use crate::image_build::buildfile::start;
use crate::image_build::ui;
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};
use crate::read_interactive_input::GrammarFragment;

use walkdir::DirEntry;

use super::grammar::build_rebuild_grammars;
use super::image_ops::pull_image;
use super::types::Image;

pub struct UserChoiceContext<'a> {
    pub entry: &'a DirEntry,
    pub custom_img_nm: &'a str,
    pub build_args: &'a [String],
    pub container_name: &'a str,
    pub grammars: &'a [GrammarFragment],
}

pub fn handle_user_choice<C: CommandHelper>(
    cmd_helper: &C,
    images_already_processed: &mut Vec<Image>,
    user_entered_val: &str,
    context: &UserChoiceContext,
) -> Result<bool, Box<dyn std::error::Error>> {
    match user_entered_val {
        "p" => {
            pull_image(cmd_helper, context.custom_img_nm)
                .unwrap_or_else(|e| eprintln!("Error pulling image: {e}"));
            Ok(true)
        }
        "N" => Ok(true),
        "d" => {
            ui::display_image_info(
                cmd_helper,
                context.custom_img_nm,
                context.container_name,
                context.entry,
                context.grammars,
            );
            Ok(false)
        }
        "?" => {
            ui::display_help();
            Ok(false)
        }
        "b" => {
            start(
                context.entry,
                context.custom_img_nm,
                &context
                    .build_args
                    .iter()
                    .map(std::string::String::as_str)
                    .collect::<Vec<_>>()
            )?;
            Ok(true)
        }
        "s" => {
            let c = Image {
                name: Some(context.custom_img_nm.to_string()),
                container: Some(context.container_name.to_string()),
                skipall_by_this_name: true,
            };
            images_already_processed.push(c);
            Ok(true)
        }
        _ => {
            eprintln!("Invalid input. Please enter p/N/d/b/s/?: ");
            Ok(false)
        }
    }
}

pub fn read_val_loop<C: CommandHelper, R: ReadInteractiveInputHelper>(
    cmd_helper: &C,
    read_val_helper: &R,
    images_already_processed: &mut Vec<Image>,
    entry: &DirEntry,
    custom_img_nm: &str,
    build_args: &[String],
    container_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // use extracted helper to build grammars
    let mut grammars = build_rebuild_grammars(entry, custom_img_nm, container_name);

    loop {
        // Get the terminal width from the command helper instead of passing None
        let term_width = cmd_helper.get_terminal_display_width(None);
        let result =
            read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, Some(term_width));

        match result.user_entered_val {
            None => {
                // Check if it's a Ctrl+C signal
                if result.was_interrupted {
                    println!("\nOperation cancelled by user");
                    std::process::exit(0);
                }
                break;
            }
            Some(user_entered_val) => {
                let context = UserChoiceContext {
                    entry,
                    custom_img_nm,
                    build_args,
                    container_name,
                    grammars: &grammars,
                };
                if handle_user_choice(
                    cmd_helper,
                    images_already_processed,
                    &user_entered_val,
                    &context,
                )? {
                    break;
                }
            }
        }
    }
    Ok(())
}
