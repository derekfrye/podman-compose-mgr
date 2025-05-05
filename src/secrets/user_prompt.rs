// This module re-exports the specific prompt setup and helper functions
// for validation, upload, and retrieve flows in the secrets subsystem.

// Re-export validation prompt functions
pub use crate::secrets::validation_prompt::{
    add_choice_options, display_validation_help, setup_validation_prompt,
};

// Re-export upload prompt functions
pub use crate::secrets::upload_prompt::{
    UploadPromptConfig, add_upload_choice_options, display_upload_help, prompt_for_upload,
    prompt_for_upload_with_helper, setup_upload_prompt,
};

// Re-export retrieve prompt function
pub use crate::secrets::retrieve_prompt::setup_retrieve_prompt;
