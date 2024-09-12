use crate::args::Args;
use crate::image_cmd as cmd;

pub fn restart_services(args: &Args) {
    let mut x = vec![];
    x.push("restart");
    x.push("-f");
    x.push("docker-compose.yml");
    
    cmd::exec_cmd("podman", x);
}