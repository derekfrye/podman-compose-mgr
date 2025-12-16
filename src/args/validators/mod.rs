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
    if args.one_shot.dry_run && !args.one_shot.one_shot {
        return Err("--dry-run requires --one-shot".to_string());
    }
    if args.tui_simulate.is_some() && !args.one_shot.is_dry_run() {
        return Err("--tui-simulate requires --dry-run (and typically --one-shot)".to_string());
    }
    if args.tui_simulate_podman_input_json.is_some() && args.tui_simulate.is_none() {
        return Err("--tui-simulate-podman-input-json requires --tui-simulate".to_string());
    }
    Ok(())
}
