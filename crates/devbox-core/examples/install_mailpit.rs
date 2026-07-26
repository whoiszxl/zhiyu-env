use devbox_core::{
    installer::MAILPIT_VERSION, MailpitInstaller, MailpitService, ServiceConfig, ServiceKind,
    ServiceManager,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let root = root_argument("install_mailpit");
    let should_start = std::env::args_os().any(|argument| argument == "--start");
    let instance = root.join("instances/mailpit/default");
    let installer = MailpitInstaller::new(&root);
    if let Err(error) = installer.install().and_then(|outcome| {
        println!("{outcome:?}");
        let service = MailpitService::new(ServiceConfig {
            name: "Mailpit".into(),
            kind: ServiceKind::Mailpit,
            version: MAILPIT_VERSION.into(),
            port: 1025,
            executable: installer.installation_dir().join("bin/mailpit"),
            arguments: Vec::new(),
            environment: environment(&instance),
            instance_dir: instance,
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

fn environment(instance: &std::path::Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("MP_SMTP_BIND_ADDR".into(), "127.0.0.1:1025".into()),
        ("MP_UI_BIND_ADDR".into(), "127.0.0.1:8025".into()),
        (
            "MP_DATABASE".into(),
            instance.join("data/mailpit.db").display().to_string(),
        ),
        ("MP_MAX_MESSAGES".into(), "500".into()),
        ("MP_MAX_MESSAGE_SIZE".into(), "10".into()),
        ("MP_DISABLE_VERSION_CHECK".into(), "true".into()),
        ("MP_BLOCK_REMOTE_CSS_AND_FONTS".into(), "true".into()),
        ("MP_QUIET".into(), "true".into()),
    ])
}

fn root_argument(name: &str) -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("usage: {name} <devbox-root> [--start]"))
}
