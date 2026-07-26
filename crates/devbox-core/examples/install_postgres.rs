use devbox_core::{PostgresInstaller, PostgresService, ServiceConfig, ServiceKind, ServiceManager};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let root = root_argument("install_postgres");
    let instance = root.join("instances/postgres/default");
    let installer = PostgresInstaller::new(&root);
    if let Err(error) = installer.install().and_then(|outcome| {
        println!("{outcome:?}");
        let data_dir = instance.join("data").join(installer.release().series);
        let service = PostgresService::new(ServiceConfig {
            name: "PostgreSQL".into(),
            kind: ServiceKind::Postgres,
            version: installer.release().version.into(),
            port: 5432,
            executable: installer.installation_dir().join("bin/postgres"),
            arguments: vec![
                "-D".into(),
                data_dir.display().to_string(),
                "-c".into(),
                format!(
                    "config_file={}",
                    instance.join("conf/postgresql.conf").display()
                ),
                "-c".into(),
                format!("data_directory={}", data_dir.display()),
            ],
            environment: BTreeMap::new(),
            instance_dir: instance.clone(),
        })?;
        service.install()?;
        installer.initialize(&data_dir)
    }) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn root_argument(name: &str) -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("usage: {name} <devbox-root>"))
}
