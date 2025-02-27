use crate::args::Args;
use crate::helpers::cmd_helper_fns as cmd;

pub fn restart_services(args: &Args) {
    if args.verbose {
        println!("Starting {}...", args.path.display());
    }
    let x = ["restart", "-f", "docker-compose.yml"];

    cmd::exec_cmd("podman", &x);
}
