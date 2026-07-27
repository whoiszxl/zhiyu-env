use devbox_core::{
    installer::MONGODB_VERSION, MongodbInstaller, MongodbService, ServiceConfig, ServiceKind,
    ServiceManager,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let root = root_argument("install_mongodb");
    let should_start = std::env::args_os().any(|argument| argument == "--start");
    let instance = root.join("instances/mongodb/default");
    let installer = MongodbInstaller::new(&root);
    if let Err(error) = installer.install().and_then(|outcome| {
        println!("{outcome:?}");
        let service = MongodbService::new(ServiceConfig {
            name: "MongoDB".into(),
            kind: ServiceKind::Mongodb,
            version: MONGODB_VERSION.into(),
            port: 27017,
            executable: installer.installation_dir().join("bin/mongod"),
            arguments: vec![
                "--config".into(),
                instance.join("conf/mongod.conf").display().to_string(),
            ],
            environment: BTreeMap::new(),
            instance_dir: instance,
            wait_for_port: true,
        })?;
        service.install()?;
        if should_start {
            let pid = service.start()?;
            println!("Started {{ pid: {pid} }}");
        }
        Ok(())
    }) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn root_argument(name: &str) -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("usage: {name} <devbox-root> [--start]"))
}
