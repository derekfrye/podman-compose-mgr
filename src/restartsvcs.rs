use crate::args::Args;
use crate::helpers::cmd_helper_fns as cmd;

pub fn restart_services(args: &Args) {
    if args.verbose {
        println!("Checking for restart in path: {}", args.path.display());
    }
    let mut x = vec![];
    x.push("restart");
    x.push("-f");
    x.push("docker-compose.yml");
    
    cmd::exec_cmd("podman", x);
}