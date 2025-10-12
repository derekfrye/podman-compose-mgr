use crate::image_build::buildfile::start;
use crate::image_build::ui;
use crate::interfaces::{CommandHelper, ReadInteractiveInputHelper};
use crate::read_interactive_input::GrammarFragment;
use crate::utils::build_logger::{BuildLogLevel, BuildLogger};

use walkdir::DirEntry;

use super::grammar::build_rebuild_grammars;
use super::image_ops::pull_image;
use super::types::{Image, RebuildSelection};

pub struct UserChoiceContext<'a> {
    pub entry: &'a DirEntry,
    pub selection: RebuildSelection<'a>,
    pub build_args: &'a [String],
    pub grammars: &'a [GrammarFragment],
    pub logger: &'a dyn BuildLogger,
    pub no_cache: bool,
}

pub fn handle_user_choice<C: CommandHelper>(
    cmd_helper: &C,
    images_already_processed: &mut Vec<Image>,
    user_entered_val: &str,
    context: &UserChoiceContext,
) -> Result<bool, Box<dyn std::error::Error>> {
    match user_entered_val {
        "p" => {
            pull_selected_image(cmd_helper, context);
            Ok(true)
        }
        "N" => Ok(true),
        "d" => {
            display_image_details(cmd_helper, context);
            Ok(false)
        }
        "?" => {
            ui::display_help();
            Ok(false)
        }
        "b" => {
            start_selected_build(cmd_helper, context)?;
            Ok(true)
        }
        "s" => {
            mark_image_as_skipped(images_already_processed, context);
            Ok(true)
        }
        _ => {
            notify_invalid_choice(context.logger);
            Ok(false)
        }
    }
}

fn pull_selected_image<C: CommandHelper>(cmd_helper: &C, context: &UserChoiceContext<'_>) {
    if let Err(e) = pull_image(cmd_helper, context.selection.image) {
        context
            .logger
            .log(BuildLogLevel::Error, &format!("Error pulling image: {e}"));
    }
}

fn display_image_details<C: CommandHelper>(cmd_helper: &C, context: &UserChoiceContext<'_>) {
    ui::display_image_info(
        cmd_helper,
        context.selection.image,
        context.selection.container,
        context.entry,
        context.grammars,
    );
}

fn start_selected_build<C: CommandHelper>(
    cmd_helper: &C,
    context: &UserChoiceContext<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let build_args: Vec<&str> = context.build_args.iter().map(String::as_str).collect();
    start(
        cmd_helper,
        context.entry,
        context.selection.image,
        &build_args,
        context.logger,
        context.no_cache,
    )
    .map_err(Box::<dyn std::error::Error>::from)?;
    Ok(())
}

fn mark_image_as_skipped(images: &mut Vec<Image>, context: &UserChoiceContext<'_>) {
    images.push(Image {
        name: Some(context.selection.image.to_string()),
        container: Some(context.selection.container.to_string()),
        skipall_by_this_name: true,
    });
}

fn notify_invalid_choice(logger: &dyn BuildLogger) {
    logger.log(
        BuildLogLevel::Warn,
        "Invalid input. Please enter p/N/d/b/s/?: ",
    );
}

/// Read a value from the user and handle the action loop for rebuild.
///
/// # Errors
/// Returns an error if reading input or executing the selected action fails.
pub fn read_val_loop<C: CommandHelper, R: ReadInteractiveInputHelper>(
    cmd_helper: &C,
    read_val_helper: &R,
    images_already_processed: &mut Vec<Image>,
    entry: &DirEntry,
    selection: RebuildSelection<'_>,
    build_args: &[String],
    no_cache: bool,
    logger: &dyn BuildLogger,
) -> Result<(), Box<dyn std::error::Error>> {
    // use extracted helper to build grammars
    let mut grammars = build_rebuild_grammars(entry, selection.image, selection.container);

    loop {
        // Get the terminal width from the command helper instead of passing None
        let term_width = cmd_helper.get_terminal_display_width(None);
        let result =
            read_val_helper.read_val_from_cmd_line_and_proceed(&mut grammars, Some(term_width));

        match result.user_entered_val {
            None => {
                // Check if it's a Ctrl+C signal
                if result.was_interrupted {
                    logger.log(BuildLogLevel::Warn, "Operation cancelled by user");
                    std::process::exit(0);
                }
                break;
            }
            Some(user_entered_val) => {
                let context = UserChoiceContext {
                    entry,
                    selection,
                    build_args,
                    grammars: &grammars,
                    logger,
                    no_cache,
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
