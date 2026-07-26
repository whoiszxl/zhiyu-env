use crate::error::{DevBoxError, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const REDIS_SERIES: &str = "7.2";
pub const REDIS_VERSION: &str = "7.2.15";
const REDIS_ARCHIVE: &str = "redis-7.2.15.tar.gz";
const REDIS_URL: &str = "https://download.redis.io/releases/redis-7.2.15.tar.gz";
const REDIS_SHA256: &str = "7bf7975331511fdb788e85dae63964b128fccee1df026a10db57444babc9c9c4";
pub const MYSQL_SERIES: &str = "8.4";
pub const MYSQL_VERSION: &str = "8.4.10";
const MYSQL_ARCHIVE: &str = "mysql-8.4.10-macos15-arm64.tar.gz";
const MYSQL_URL: &str =
    "https://cdn.mysql.com/Downloads/MySQL-8.4/mysql-8.4.10-macos15-arm64.tar.gz";
const MYSQL_SHA256: &str = "282618afd5cb662b94ac837f210b0ccb87ef156dd4c03eb88e094702a5c9ea1f";
pub const POSTGRES_SERIES: &str = "17";
pub const POSTGRES_VERSION: &str = "17.10";
const POSTGRES_ARCHIVE: &str = "postgresql-17.10.tar.bz2";
const POSTGRES_URL: &str = "https://ftp.postgresql.org/pub/source/v17.10/postgresql-17.10.tar.bz2";
const POSTGRES_SHA256: &str = "078a03516dcdbdb705fecaf415ea3d13a956c589e46f09fed68a06fb00598c90";
pub const MONGODB_SERIES: &str = "8.0";
pub const MONGODB_VERSION: &str = "8.0.26";
const MONGODB_ARCHIVE: &str = "mongodb-macos-arm64-8.0.26.tgz";
const MONGODB_URL: &str = "https://fastdl.mongodb.org/osx/mongodb-macos-arm64-8.0.26.tgz";
const MONGODB_SHA256: &str = "49f93af3935632d8ca460584166e07fb7d00fb1c2173544851df453edfdb14a3";
pub const MAILPIT_SERIES: &str = "1.30";
pub const MAILPIT_VERSION: &str = "1.30.5";
const MAILPIT_ARCHIVE: &str = "mailpit-darwin-arm64.tar.gz";
const MAILPIT_URL: &str =
    "https://github.com/axllent/mailpit/releases/download/v1.30.5/mailpit-darwin-arm64.tar.gz";
const MAILPIT_SHA256: &str = "8713a5665dc6ba16e08f2b6b70a6ddd490ade23407d18ee59e5a9fceee47f171";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed { path: PathBuf },
    AlreadyInstalled { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct RedisInstaller {
    devbox_root: PathBuf,
}

impl RedisInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        self.ensure_build_tools()?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/redis-server");
        if self.is_expected_version(&executable) {
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let temp_root = self.devbox_root.join("tmp");
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&temp_root)?;
        fs::create_dir_all(
            installation_dir
                .parent()
                .expect("Redis installation has a parent"),
        )?;

        let archive = downloads_dir.join(REDIS_ARCHIVE);
        self.prepare_archive(&archive)?;

        let work_dir = temp_root.join(format!(
            "redis-{REDIS_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;

        let result = self.build_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations")
            .join("redis")
            .join(REDIS_SERIES)
    }

    fn ensure_build_tools(&self) -> Result<()> {
        for tool in [
            "/usr/bin/curl",
            "/usr/bin/tar",
            "/usr/bin/make",
            "/usr/bin/cc",
        ] {
            if !Path::new(tool).is_file() {
                return Err(DevBoxError::CommandFailed {
                    command: tool.into(),
                    message: "macOS Command Line Tools are required".into(),
                });
            }
        }
        Ok(())
    }

    fn prepare_archive(&self, archive: &Path) -> Result<()> {
        if archive.is_file() {
            if sha256(archive)? == REDIS_SHA256 {
                return Ok(());
            }
            fs::remove_file(archive)?;
        }

        let partial = archive.with_file_name(format!("{REDIS_ARCHIVE}.partial"));
        let _ = fs::remove_file(&partial);
        run(
            Command::new("/usr/bin/curl")
                .args(["--fail", "--location", "--silent", "--show-error"])
                .arg("--output")
                .arg(&partial)
                .arg(REDIS_URL),
            "curl",
        )?;

        let actual = sha256(&partial)?;
        if actual != REDIS_SHA256 {
            let _ = fs::remove_file(&partial);
            return Err(DevBoxError::IntegrityMismatch {
                expected: REDIS_SHA256.into(),
                actual,
            });
        }

        fs::rename(partial, archive)?;
        Ok(())
    }

    fn build_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;

        let source_dir = work_dir.join(format!("redis-{REDIS_VERSION}"));
        let jobs = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(2)
            .min(8);
        run(
            Command::new("/usr/bin/make")
                .arg("-C")
                .arg(&source_dir)
                .arg(format!("-j{jobs}"))
                .arg("BUILD_TLS=no")
                .arg("MALLOC=libc"),
            "make",
        )?;

        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;

        for binary in [
            "redis-server",
            "redis-cli",
            "redis-benchmark",
            "redis-check-aof",
            "redis-check-rdb",
            "redis-sentinel",
        ] {
            fs::copy(source_dir.join("src").join(binary), bin_dir.join(binary))?;
        }

        if !self.is_expected_version(&bin_dir.join("redis-server")) {
            return Err(DevBoxError::CommandFailed {
                command: "redis-server --version".into(),
                message: format!("built binary is not Redis {REDIS_VERSION}"),
            });
        }

        let manifest = RedisManifest {
            service: "redis",
            series: REDIS_SERIES,
            version: REDIS_VERSION,
            source_url: REDIS_URL,
            source_sha256: REDIS_SHA256,
            build: "official-source",
        };
        fs::write(
            stage.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;

        if installation_dir.exists() {
            fs::remove_dir_all(installation_dir)?;
        }
        fs::rename(stage, installation_dir)?;
        Ok(())
    }

    fn is_expected_version(&self, executable: &Path) -> bool {
        Command::new(executable)
            .arg("--version")
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains(REDIS_VERSION)
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct MysqlInstaller {
    devbox_root: PathBuf,
}

impl MysqlInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        ensure_macos_arm64("MySQL")?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mysqld");
        if binary_contains(&executable, &["--version"], MYSQL_VERSION) {
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let temp_root = self.devbox_root.join("tmp");
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&temp_root)?;
        fs::create_dir_all(
            installation_dir
                .parent()
                .expect("MySQL installation has a parent"),
        )?;

        let archive = downloads_dir.join(MYSQL_ARCHIVE);
        prepare_archive(&archive, MYSQL_ARCHIVE, MYSQL_URL, MYSQL_SHA256)?;
        let work_dir = temp_root.join(format!(
            "mysql-{MYSQL_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let result = self.extract_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn initialize(&self, data_dir: &Path) -> Result<()> {
        if data_dir.join("mysql").is_dir() {
            return Ok(());
        }
        fs::create_dir_all(data_dir)?;
        run(
            Command::new(self.installation_dir().join("bin/mysqld"))
                .arg("--no-defaults")
                .arg("--initialize-insecure")
                .arg(format!("--basedir={}", self.installation_dir().display()))
                .arg(format!("--datadir={}", data_dir.display())),
            "mysqld --initialize-insecure",
        )
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations")
            .join("mysql")
            .join(MYSQL_SERIES)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source = work_dir.join(format!("mysql-{MYSQL_VERSION}-macos15-arm64"));
        let stage = work_dir.join("installation");
        fs::rename(source, &stage)?;

        if !binary_contains(&stage.join("bin/mysqld"), &["--version"], MYSQL_VERSION) {
            return Err(DevBoxError::CommandFailed {
                command: "mysqld --version".into(),
                message: format!("downloaded binary is not MySQL {MYSQL_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "mysql",
            MYSQL_SERIES,
            MYSQL_VERSION,
            MYSQL_URL,
            MYSQL_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, installation_dir)
    }
}

#[derive(Debug, Clone)]
pub struct MongodbInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MailpitInstaller {
    devbox_root: PathBuf,
}

impl MailpitInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        ensure_macos_arm64("Mailpit")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mailpit");
        if binary_contains(&executable, &["version"], MAILPIT_VERSION) {
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let temp_root = self.devbox_root.join("tmp");
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&temp_root)?;
        fs::create_dir_all(
            installation_dir
                .parent()
                .expect("Mailpit installation has a parent"),
        )?;

        let archive = downloads_dir.join(MAILPIT_ARCHIVE);
        prepare_archive(&archive, MAILPIT_ARCHIVE, MAILPIT_URL, MAILPIT_SHA256)?;
        let work_dir = temp_root.join(format!(
            "mailpit-{MAILPIT_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let result = self.extract_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations")
            .join("mailpit")
            .join(MAILPIT_SERIES)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source = work_dir.join("mailpit");
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        fs::copy(source, bin_dir.join("mailpit"))?;

        if !binary_contains(&bin_dir.join("mailpit"), &["version"], MAILPIT_VERSION) {
            return Err(DevBoxError::CommandFailed {
                command: "mailpit version".into(),
                message: format!("downloaded binary is not Mailpit {MAILPIT_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "mailpit",
            MAILPIT_SERIES,
            MAILPIT_VERSION,
            MAILPIT_URL,
            MAILPIT_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, installation_dir)
    }
}

impl MongodbInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        ensure_macos_arm64("MongoDB")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mongod");
        if binary_contains(&executable, &["--version"], MONGODB_VERSION) {
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let temp_root = self.devbox_root.join("tmp");
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&temp_root)?;
        fs::create_dir_all(
            installation_dir
                .parent()
                .expect("MongoDB installation has a parent"),
        )?;

        let archive = downloads_dir.join(MONGODB_ARCHIVE);
        prepare_archive(&archive, MONGODB_ARCHIVE, MONGODB_URL, MONGODB_SHA256)?;
        let work_dir = temp_root.join(format!(
            "mongodb-{MONGODB_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let result = self.extract_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations")
            .join("mongodb")
            .join(MONGODB_SERIES)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source = fs::read_dir(work_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with("mongodb-macos-") && name.ends_with(MONGODB_VERSION)
                        })
            })
            .ok_or_else(|| DevBoxError::CommandFailed {
                command: "tar".into(),
                message: "MongoDB archive does not contain the expected directory".into(),
            })?;
        let stage = work_dir.join("installation");
        fs::rename(source, &stage)?;

        if !binary_contains(&stage.join("bin/mongod"), &["--version"], MONGODB_VERSION) {
            return Err(DevBoxError::CommandFailed {
                command: "mongod --version".into(),
                message: format!("downloaded binary is not MongoDB {MONGODB_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "mongodb",
            MONGODB_SERIES,
            MONGODB_VERSION,
            MONGODB_URL,
            MONGODB_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, installation_dir)
    }
}

#[derive(Debug, Clone)]
pub struct PostgresInstaller {
    devbox_root: PathBuf,
}

impl PostgresInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        ensure_macos_arm64("PostgreSQL")?;
        ensure_tools(&[
            "/usr/bin/curl",
            "/usr/bin/tar",
            "/usr/bin/make",
            "/usr/bin/cc",
        ])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/postgres");
        if binary_contains(&executable, &["--version"], POSTGRES_VERSION)
            && binary_contains(
                &installation_dir.join("bin/initdb"),
                &["--version"],
                POSTGRES_VERSION,
            )
        {
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let temp_root = self.devbox_root.join("tmp");
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&temp_root)?;
        fs::create_dir_all(
            installation_dir
                .parent()
                .expect("PostgreSQL installation has a parent"),
        )?;

        let archive = downloads_dir.join(POSTGRES_ARCHIVE);
        prepare_archive(&archive, POSTGRES_ARCHIVE, POSTGRES_URL, POSTGRES_SHA256)?;
        let work_dir = temp_root.join(format!(
            "postgres-{POSTGRES_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let result = self.build_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn initialize(&self, data_dir: &Path) -> Result<()> {
        if data_dir.join("PG_VERSION").is_file() {
            return Ok(());
        }
        fs::create_dir_all(data_dir)?;
        run(
            Command::new(self.installation_dir().join("bin/initdb"))
                .arg("-D")
                .arg(data_dir)
                .args([
                    "--username=postgres",
                    "--auth=trust",
                    "--encoding=UTF8",
                    "--no-locale",
                ]),
            "initdb",
        )
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations")
            .join("postgres")
            .join(POSTGRES_SERIES)
    }

    fn build_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        run(
            Command::new("/usr/bin/tar")
                .args(["-xjf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source = work_dir.join(format!("postgresql-{POSTGRES_VERSION}"));
        let destination_root = work_dir.join("destination");
        let relative_installation = installation_dir.strip_prefix("/").map_err(|_| {
            DevBoxError::InvalidConfig("PostgreSQL installation path must be absolute".into())
        })?;
        let stage = destination_root.join(relative_installation);
        let mut configure = Command::new(source.join("configure"));
        configure
            .current_dir(&source)
            .arg(format!("--prefix={}", installation_dir.display()))
            .args(["--without-icu", "--without-readline", "--without-zlib"]);
        run(&mut configure, "configure")?;
        let jobs = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(2)
            .min(8);
        run(
            Command::new("/usr/bin/make")
                .arg("-C")
                .arg(&source)
                .arg(format!("-j{jobs}")),
            "make",
        )?;
        run(
            Command::new("/usr/bin/make")
                .arg("-C")
                .arg(&source)
                .arg("install")
                .arg(format!("DESTDIR={}", destination_root.display())),
            "make install",
        )?;

        if !binary_contains(
            &stage.join("bin/postgres"),
            &["--version"],
            POSTGRES_VERSION,
        ) || !binary_contains(&stage.join("bin/initdb"), &["--version"], POSTGRES_VERSION)
        {
            return Err(DevBoxError::CommandFailed {
                command: "postgres --version".into(),
                message: format!("built binary is not PostgreSQL {POSTGRES_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "postgres",
            POSTGRES_SERIES,
            POSTGRES_VERSION,
            POSTGRES_URL,
            POSTGRES_SHA256,
            "official-source",
        )?;
        replace_installation(&stage, installation_dir)
    }
}

#[derive(Serialize)]
struct RedisManifest<'a> {
    service: &'a str,
    series: &'a str,
    version: &'a str,
    source_url: &'a str,
    source_sha256: &'a str,
    build: &'a str,
}

#[derive(Serialize)]
struct ServiceManifest<'a> {
    service: &'a str,
    series: &'a str,
    version: &'a str,
    source_url: &'a str,
    source_sha256: &'a str,
    build: &'a str,
}

fn prepare_archive(
    archive: &Path,
    archive_name: &str,
    source_url: &str,
    expected_sha256: &str,
) -> Result<()> {
    if archive.is_file() {
        if sha256(archive)? == expected_sha256 {
            return Ok(());
        }
        fs::remove_file(archive)?;
    }

    let partial = archive.with_file_name(format!("{archive_name}.partial"));
    let _ = fs::remove_file(&partial);
    run(
        Command::new("/usr/bin/curl")
            .args(["--fail", "--location", "--silent", "--show-error"])
            .arg("--output")
            .arg(&partial)
            .arg(source_url),
        "curl",
    )?;
    let actual = sha256(&partial)?;
    if actual != expected_sha256 {
        let _ = fs::remove_file(&partial);
        return Err(DevBoxError::IntegrityMismatch {
            expected: expected_sha256.into(),
            actual,
        });
    }
    fs::rename(partial, archive)?;
    Ok(())
}

fn write_manifest(
    stage: &Path,
    service: &str,
    series: &str,
    version: &str,
    source_url: &str,
    source_sha256: &str,
    build: &str,
) -> Result<()> {
    let manifest = ServiceManifest {
        service,
        series,
        version,
        source_url,
        source_sha256,
        build,
    };
    fs::write(
        stage.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

fn replace_installation(stage: &Path, installation_dir: &Path) -> Result<()> {
    if installation_dir.exists() {
        fs::remove_dir_all(installation_dir)?;
    }
    fs::rename(stage, installation_dir)?;
    Ok(())
}

fn binary_contains(executable: &Path, arguments: &[&str], expected: &str) -> bool {
    Command::new(executable)
        .args(arguments)
        .output()
        .map(|output| {
            output.status.success()
                && (String::from_utf8_lossy(&output.stdout).contains(expected)
                    || String::from_utf8_lossy(&output.stderr).contains(expected))
        })
        .unwrap_or(false)
}

fn ensure_tools(tools: &[&str]) -> Result<()> {
    for tool in tools {
        if !Path::new(tool).is_file() {
            return Err(DevBoxError::CommandFailed {
                command: (*tool).into(),
                message: "macOS Command Line Tools are required".into(),
            });
        }
    }
    Ok(())
}

fn ensure_macos_arm64(service: &str) -> Result<()> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Ok(())
    } else {
        Err(DevBoxError::UnsupportedPlatform(format!(
            "{service} automatic installation currently supports macOS Apple Silicon"
        )))
    }
}

fn sha256(path: &Path) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn run(command: &mut Command, name: &str) -> Result<()> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let message = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => format!("process exited with {}", output.status),
    };
    Err(DevBoxError::CommandFailed {
        command: name.into(),
        message: tail_chars(&message, 32 * 1024),
    })
}

fn tail_chars(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        value.into()
    } else {
        format!(
            "…earlier output omitted…\n{}",
            value.chars().skip(count - max_chars).collect::<String>()
        )
    }
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_constants_target_the_same_release() {
        assert!(REDIS_URL.ends_with(REDIS_ARCHIVE));
        assert!(REDIS_ARCHIVE.contains(REDIS_VERSION));
        assert_eq!(REDIS_SHA256.len(), 64);
    }

    #[test]
    fn database_installer_constants_target_matching_releases() {
        assert!(MYSQL_URL.ends_with(MYSQL_ARCHIVE));
        assert!(MYSQL_ARCHIVE.contains(MYSQL_VERSION));
        assert_eq!(MYSQL_SHA256.len(), 64);
        assert!(POSTGRES_URL.ends_with(POSTGRES_ARCHIVE));
        assert!(POSTGRES_ARCHIVE.contains(POSTGRES_VERSION));
        assert_eq!(POSTGRES_SHA256.len(), 64);
        assert!(MONGODB_URL.ends_with(MONGODB_ARCHIVE));
        assert!(MONGODB_ARCHIVE.contains(MONGODB_VERSION));
        assert_eq!(MONGODB_SHA256.len(), 64);
        assert!(MAILPIT_URL.ends_with(MAILPIT_ARCHIVE));
        assert!(MAILPIT_ARCHIVE.contains("arm64"));
        assert_eq!(MAILPIT_SHA256.len(), 64);
    }
}
