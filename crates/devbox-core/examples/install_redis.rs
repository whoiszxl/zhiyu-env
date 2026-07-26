use devbox_core::RedisInstaller;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .expect("usage: install_redis <devbox-root> [5.0.14|6.0.20|6.2.23|7.0.15|7.2.15|7.4.10]");
    let version = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "7.2.15".to_string());

    let installer = RedisInstaller::for_version(root, &version)
        .unwrap_or_else(|error| panic!("invalid Redis version {version}: {error}"));
    match installer.install() {
        Ok(outcome) => println!("{outcome:?}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
