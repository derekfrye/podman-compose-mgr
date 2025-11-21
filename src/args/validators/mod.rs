use super::types::Args;

/// Validate the args for rebuild mode
///
/// # Errors
///
/// Returns an error if the arguments are invalid for rebuild mode.
pub fn validate(args: &Args) -> Result<(), String> {
    if args.rebuild_view_line_buffer_max == 0 {
        return Err("--rebuild-view-line-buffer-max must be at least 1".to_string());
    }
    if args.dry_run && !args.one_shot {
        return Err("--dry-run requires --one-shot".to_string());
    }
    Ok(())
}
