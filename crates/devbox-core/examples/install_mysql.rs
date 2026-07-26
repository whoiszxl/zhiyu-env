use devbox_core::{MysqlInstaller, MysqlService, ServiceConfig, ServiceKind, ServiceManager};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let root = root_argument("install_mysql");
    let instance = root.join("instances/mysql/default");
    let installer = MysqlInstaller::new(&root);
    if let Err(error) = installer.install().and_then(|outcome| {
        println!("{outcome:?}");
        let service = MysqlService::new(ServiceConfig {
            name: "MySQL".into(),
            kind: ServiceKind::Mysql,
            version: "8.4.10".into(),
            port: 3306,
            executable: installer.installation_dir().join("bin/mysqld"),
            arguments: vec![format!(
                "--defaults-file={}",
                instance.join("conf/my.cnf").display()
            )],
            environment: BTreeMap::new(),
            instance_dir: instance.clone(),
        })?;
        service.install()?;
        installer.initialize(&instance.join("data"))
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
