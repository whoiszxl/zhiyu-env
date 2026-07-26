use devbox_core::RedisInstaller;
use std::path::PathBuf;

fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: install_redis <devbox-root>");

    match RedisInstaller::new(root).install() {
        Ok(outcome) => println!("{outcome:?}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
