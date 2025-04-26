use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::json_utils::write_json_output; // Changed import from utils to json_utils
use crate::secrets::models::JsonOutput;
use std::fs;

/// Write validation results to the output file
pub fn write_validation_results(args: &Args, json_outputs: &[JsonOutput]) -> Result<()> {
    if let Some(output_path) = args.output_json.as_ref() {
        if let Some(output_dir) = output_path.parent() {
            fs::create_dir_all(output_dir).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "Failed to create output directory: {}",
                    e
                ))
            })?;
        }

        // Convert the slice to owned Vec for write_json_output
        let outputs_vec = json_outputs.to_owned();

        // Call the correct write_json_output function, passing args
        write_json_output(outputs_vec, output_path, args)?;
    } else {
        return Err(Box::<dyn std::error::Error>::from(
            "Output JSON path is required",
        ));
    }

    Ok(())
}
