use crate::args::Args;
use crate::secrets::error::Result;
use crate::secrets::models::JsonOutput;
use crate::secrets::utils::write_json_output;
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

        let output_str = output_path
            .to_str()
            .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid UTF-8 in output path"))?;

        write_json_output(json_outputs, output_str)?;
    } else {
        return Err(Box::<dyn std::error::Error>::from(
            "Output JSON path is required",
        ));
    }

    Ok(())
}