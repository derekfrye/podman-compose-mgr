use crate::interfaces::ReadInteractiveInputHelper;
use crate::read_interactive_input::{GrammarFragment, GrammarType};
use crate::secrets::error::Result;
use crate::secrets::file_details::{display_file_details, format_file_size, get_file_details};
use crate::utils::json_utils;
use serde_json::Value;

use crate::secrets::validation_prompt::{add_choice_options, display_validation_help, setup_validation_prompt};
use crate::secrets::upload_prompt::{add_upload_choice_options, display_upload_help, prompt_for_upload, prompt_for_upload_with_helper, setup_upload_prompt, UploadPromptConfig};
use crate::secrets::retrieve_prompt::setup_retrieve_prompt;

// This module re-exports the specific prompt setup and helper functions
// for validation, upload, and retrieve flows in the secrets subsystem.
pub use setup_validation_prompt;
pub use add_choice_options;
pub use display_validation_help;

pub use setup_upload_prompt;
pub use add_upload_choice_options;
pub use display_upload_help;
pub use UploadPromptConfig;
pub use prompt_for_upload;
pub use prompt_for_upload_with_helper;

pub use setup_retrieve_prompt;
