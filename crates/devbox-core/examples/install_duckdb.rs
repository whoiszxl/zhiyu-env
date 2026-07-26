use devbox_core::DuckdbInstaller;

fn main() {
    let home = std::env::var_os("HOME").expect("HOME is not set");
    let root = std::path::PathBuf::from(home).join(".devbox");
    match DuckdbInstaller::new(root).install() {
        Ok(outcome) => println!("{outcome:?}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
