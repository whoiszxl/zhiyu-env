use devbox_core::{MysqlInstaller, MysqlService, ServiceConfig, ServiceKind, ServiceManager};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let root = root_argument("install_mysql");
    let instance = root.join("instances/mysql/default");
    let installer = MysqlInstaller::new(&root);
    if let Err(error) = installer.install().and_then(|outcome| {
        println!("{outcome:?}");
        let installation = installer.installation_dir();
        let data_dir = instance.join("data").join(installer.release().series);
        let service = MysqlService::new(ServiceConfig {
            name: "MySQL".into(),
            kind: ServiceKind::Mysql,
            version: installer.release().version.into(),
            port: 3306,
            executable: installation.join("bin/mysqld"),
            arguments: vec![
                format!("--defaults-file={}", instance.join("conf/my.cnf").display()),
                format!("--basedir={}", installer.installation_dir().display()),
                format!("--datadir={}", data_dir.display()),
            ],
            environment: BTreeMap::new(),
            instance_dir: instance.clone(),
            wait_for_port: true,
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
