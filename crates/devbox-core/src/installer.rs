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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedisRelease {
    pub series: &'static str,
    pub version: &'static str,
    pub archive: &'static str,
    pub source_url: &'static str,
    pub sha256: &'static str,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
}

pub const REDIS_RELEASES: &[RedisRelease] = &[
    RedisRelease {
        series: "5.0",
        version: "5.0.14",
        archive: "redis-5.0.14.tar.gz",
        source_url: "https://download.redis.io/releases/redis-5.0.14.tar.gz",
        sha256: "3ea5024766d983249e80d4aa9457c897a9f079957d0fb1f35682df233f997f32",
        support_label: "旧版 · 已停止维护",
        legacy: true,
        recommended: false,
    },
    RedisRelease {
        series: "6.0",
        version: "6.0.20",
        archive: "redis-6.0.20.tar.gz",
        source_url: "https://download.redis.io/releases/redis-6.0.20.tar.gz",
        sha256: "173d4c5f44b5d7186da96c4adc5cb20e8018b50ec3a8dfe0d191dbbab53952f0",
        support_label: "旧版 · 已停止维护",
        legacy: true,
        recommended: false,
    },
    RedisRelease {
        series: "6.2",
        version: "6.2.23",
        archive: "redis-6.2.23.tar.gz",
        source_url: "https://download.redis.io/releases/redis-6.2.23.tar.gz",
        sha256: "f06cffd69f4016986508017469cf64c16e25b1282927ea9360e7c2d1839eb8e7",
        support_label: "维护期至 2027-04",
        legacy: false,
        recommended: false,
    },
    RedisRelease {
        series: "7.0",
        version: "7.0.15",
        archive: "redis-7.0.15.tar.gz",
        source_url: "https://download.redis.io/releases/redis-7.0.15.tar.gz",
        sha256: "98066f5363504b26c34dd20fbcc3c957990d764cdf42576c836fc021073f4341",
        support_label: "旧版 · 已停止维护",
        legacy: true,
        recommended: false,
    },
    RedisRelease {
        series: REDIS_SERIES,
        version: REDIS_VERSION,
        archive: "redis-7.2.15.tar.gz",
        source_url: "https://download.redis.io/releases/redis-7.2.15.tar.gz",
        sha256: "7bf7975331511fdb788e85dae63964b128fccee1df026a10db57444babc9c9c4",
        support_label: "维护期至 2029-12",
        legacy: false,
        recommended: true,
    },
    RedisRelease {
        series: "7.4",
        version: "7.4.10",
        archive: "redis-7.4.10.tar.gz",
        source_url: "https://download.redis.io/releases/redis-7.4.10.tar.gz",
        sha256: "669ab6689b5e7d0c479e8d526ccbadae36b69a11370742ffe23822b9df8d85ba",
        support_label: "维护期至 2029-12",
        legacy: false,
        recommended: false,
    },
];

pub fn redis_release(version_or_series: &str) -> Option<&'static RedisRelease> {
    REDIS_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MysqlRelease {
    pub series: &'static str,
    pub version: &'static str,
    pub archive: &'static str,
    pub source_url: &'static str,
    pub sha256: &'static str,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
}

pub const MYSQL_SERIES: &str = "8.4";
pub const MYSQL_VERSION: &str = "8.4.10";

pub const MYSQL_RELEASES: &[MysqlRelease] = &[
    MysqlRelease {
        series: "8.0",
        version: "8.0.45",
        archive: "mysql-8.0.45-macos15-arm64.tar.gz",
        source_url: "https://cdn.mysql.com/Downloads/MySQL-8.0/mysql-8.0.45-macos15-arm64.tar.gz",
        sha256: "3b8bd89b839e663479775a99deb03ff6cde11c86a1c37c0aed7116a778e4a8bc",
        support_label: "旧版 · 已停止维护",
        legacy: true,
        recommended: false,
    },
    MysqlRelease {
        series: MYSQL_SERIES,
        version: MYSQL_VERSION,
        archive: "mysql-8.4.10-macos15-arm64.tar.gz",
        source_url: "https://cdn.mysql.com/Downloads/MySQL-8.4/mysql-8.4.10-macos15-arm64.tar.gz",
        sha256: "282618afd5cb662b94ac837f210b0ccb87ef156dd4c03eb88e094702a5c9ea1f",
        support_label: "维护期至 2029-04",
        legacy: false,
        recommended: true,
    },
    MysqlRelease {
        series: "9.7",
        version: "9.7.0",
        archive: "mysql-9.7.0-macos15-arm64.tar.gz",
        source_url: "https://cdn.mysql.com/Downloads/MySQL-9.7/mysql-9.7.0-macos15-arm64.tar.gz",
        sha256: "81d0c55227093e2ebdffb424452c458b0b4a39ddff76c5bcc25e93085ab7a912",
        support_label: "创新版本 · 无长期支持",
        legacy: false,
        recommended: false,
    },
];

pub fn mysql_release(version_or_series: &str) -> Option<&'static MysqlRelease> {
    MYSQL_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

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
pub const DUCKDB_SERIES: &str = "1.5";
pub const DUCKDB_VERSION: &str = "1.5.5";
const DUCKDB_ARCHIVE: &str = "duckdb_cli-osx-universal-1.5.5.zip";
const DUCKDB_URL: &str =
    "https://github.com/duckdb/duckdb/releases/download/v1.5.5/duckdb_cli-osx-universal.zip";
const DUCKDB_SHA256: &str = "7a4bc3a93f7f92f5b40cd09c21afaf98e415c6cb9d9170064993782e779f4115";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed { path: PathBuf },
    AlreadyInstalled { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct RedisInstaller {
    devbox_root: PathBuf,
    release: &'static RedisRelease,
}

impl RedisInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: redis_release(REDIS_VERSION).expect("default Redis release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = redis_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported Redis version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static RedisRelease {
        self.release
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

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;

        let work_dir = temp_root.join(format!(
            "redis-{}-{}-{}",
            self.release.version,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        self.is_expected_version(&self.installation_dir().join("bin/redis-server"))
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

        let source_dir = work_dir.join(format!("redis-{}", self.release.version));
        self.apply_build_compatibility(&source_dir)?;
        let jobs = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(2)
            .min(8);
        let mut make = Command::new("/usr/bin/make");
        make.arg("-C")
            .arg(&source_dir)
            .arg(format!("-j{jobs}"))
            .arg("BUILD_TLS=no")
            .arg("MALLOC=libc");
        if matches!(self.release.series, "5.0" | "6.0") {
            // Older Redis branches check for this legacy availability macro before
            // selecting fstat on macOS. Current SDKs no longer expose the macro,
            // even though fstat is correct and the old fstat64 alias was removed.
            make.arg("REDIS_CFLAGS=-DMAC_OS_X_VERSION_10_6=1060");
        }
        run(&mut make, "make")?;

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
                message: format!("built binary is not Redis {}", self.release.version),
            });
        }

        write_manifest(
            &stage,
            "redis",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-source",
        )?;
        replace_installation(&stage, installation_dir)
    }

    fn apply_build_compatibility(&self, source_dir: &Path) -> Result<()> {
        if self.release.series != "5.0" || !cfg!(all(target_os = "macos", target_arch = "aarch64"))
        {
            return Ok(());
        }

        // Redis 5 predates Apple Silicon. Its crash-reporting code assumes an
        // x86 register layout, so disable only the optional backtrace feature.
        let config_path = source_dir.join("src/config.h");
        let mut config = fs::read_to_string(&config_path)?;
        config.push_str(
            "\n#if defined(__APPLE__) && defined(__aarch64__)\n#undef HAVE_BACKTRACE\n#endif\n",
        );
        fs::write(config_path, config)?;
        Ok(())
    }

    fn is_expected_version(&self, executable: &Path) -> bool {
        Command::new(executable)
            .arg("--version")
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains(self.release.version)
            })
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct MysqlInstaller {
    devbox_root: PathBuf,
    release: &'static MysqlRelease,
}

impl MysqlInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: mysql_release(MYSQL_VERSION).expect("default MySQL release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = mysql_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported MySQL version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static MysqlRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        ensure_macos_arm64("MySQL")?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mysqld");
        if binary_contains(&executable, &["--version"], self.release.version) {
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

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "mysql-{}-{}-{}",
            self.release.version,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        binary_contains(
            &self.installation_dir().join("bin/mysqld"),
            &["--version"],
            self.release.version,
        )
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
        // MySQL 官方压缩包内的顶层目录名总是等于压缩包文件名去掉 .tar.gz 后缀，
        // 用这种方式推导比硬编码某个 macOS SDK 标签更稳妥。
        let archive_stem = self
            .release
            .archive
            .strip_suffix(".tar.gz")
            .expect("MySQL release archive name ends with .tar.gz");
        let source = work_dir.join(archive_stem);
        let stage = work_dir.join("installation");
        fs::rename(source, &stage)?;

        if !binary_contains(
            &stage.join("bin/mysqld"),
            &["--version"],
            self.release.version,
        ) {
            return Err(DevBoxError::CommandFailed {
                command: "mysqld --version".into(),
                message: format!("downloaded binary is not MySQL {}", self.release.version),
            });
        }
        write_manifest(
            &stage,
            "mysql",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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

#[derive(Debug, Clone)]
pub struct DuckdbInstaller {
    devbox_root: PathBuf,
}

impl DuckdbInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        ensure_macos_arm64("DuckDB")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/duckdb");
        if binary_contains(&executable, &["--version"], DUCKDB_VERSION) {
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
                .expect("DuckDB installation has a parent"),
        )?;

        let archive = downloads_dir.join(DUCKDB_ARCHIVE);
        prepare_archive(&archive, DUCKDB_ARCHIVE, DUCKDB_URL, DUCKDB_SHA256)?;
        let work_dir = temp_root.join(format!(
            "duckdb-{DUCKDB_VERSION}-{}-{}",
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
            .join("duckdb")
            .join(DUCKDB_SERIES)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        run(
            Command::new("/usr/bin/unzip")
                .args(["-q", "-o"])
                .arg(archive)
                .arg("-d")
                .arg(work_dir),
            "unzip",
        )?;

        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        fs::copy(work_dir.join("duckdb"), bin_dir.join("duckdb"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("duckdb"), fs::Permissions::from_mode(0o755))?;
        }

        if !binary_contains(&bin_dir.join("duckdb"), &["--version"], DUCKDB_VERSION) {
            return Err(DevBoxError::CommandFailed {
                command: "duckdb --version".into(),
                message: format!("downloaded binary is not DuckDB {DUCKDB_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "duckdb",
            DUCKDB_SERIES,
            DUCKDB_VERSION,
            DUCKDB_URL,
            DUCKDB_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, installation_dir)
    }
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
        assert_eq!(REDIS_RELEASES.len(), 6);
        for release in REDIS_RELEASES {
            assert!(release.source_url.ends_with(release.archive));
            assert!(release.archive.contains(release.version));
            assert_eq!(release.sha256.len(), 64);
            assert_eq!(redis_release(release.version), Some(release));
            assert_eq!(redis_release(release.series), Some(release));
        }
        assert!(redis_release("4.0").is_none());
        assert!(redis_release(REDIS_VERSION).is_some_and(|release| release.recommended));
    }

    #[test]
    fn redis_installers_use_independent_series_directories() {
        let root = Path::new("/tmp/zhiyu-redis-versions");
        let redis_5 = RedisInstaller::for_version(root, "5.0.14").unwrap();
        let redis_6_0 = RedisInstaller::for_version(root, "6.0").unwrap();
        let redis_6 = RedisInstaller::for_version(root, "6.2").unwrap();
        let redis_7_0 = RedisInstaller::for_version(root, "7.0").unwrap();
        let redis_7 = RedisInstaller::new(root);
        let redis_7_4 = RedisInstaller::for_version(root, "7.4").unwrap();

        assert!(redis_5.installation_dir().ends_with("redis/5.0"));
        assert!(redis_6_0.installation_dir().ends_with("redis/6.0"));
        assert!(redis_6.installation_dir().ends_with("redis/6.2"));
        assert!(redis_7_0.installation_dir().ends_with("redis/7.0"));
        assert!(redis_7.installation_dir().ends_with("redis/7.2"));
        assert!(redis_7_4.installation_dir().ends_with("redis/7.4"));
        assert!(RedisInstaller::for_version(root, "4.0").is_err());
    }

    #[test]
    fn database_installer_constants_target_matching_releases() {
        assert!(POSTGRES_URL.ends_with(POSTGRES_ARCHIVE));
        assert!(POSTGRES_ARCHIVE.contains(POSTGRES_VERSION));
        assert_eq!(POSTGRES_SHA256.len(), 64);
        assert!(MONGODB_URL.ends_with(MONGODB_ARCHIVE));
        assert!(MONGODB_ARCHIVE.contains(MONGODB_VERSION));
        assert_eq!(MONGODB_SHA256.len(), 64);
        assert!(MAILPIT_URL.ends_with(MAILPIT_ARCHIVE));
        assert!(MAILPIT_ARCHIVE.contains("arm64"));
        assert_eq!(MAILPIT_SHA256.len(), 64);
        assert!(DUCKDB_URL.ends_with("duckdb_cli-osx-universal.zip"));
        assert!(DUCKDB_ARCHIVE.contains(DUCKDB_VERSION));
        assert_eq!(DUCKDB_SHA256.len(), 64);
    }

    #[test]
    fn mysql_constants_target_the_same_release() {
        assert_eq!(MYSQL_RELEASES.len(), 3);
        for release in MYSQL_RELEASES {
            assert!(release.source_url.ends_with(release.archive));
            assert!(release.archive.contains(release.version));
            assert_eq!(release.sha256.len(), 64);
            assert_eq!(mysql_release(release.version), Some(release));
            assert_eq!(mysql_release(release.series), Some(release));
        }
        assert!(mysql_release("7.0").is_none());
        assert!(mysql_release(MYSQL_VERSION).is_some_and(|release| release.recommended));
    }

    #[test]
    fn mysql_installers_use_independent_series_directories() {
        let root = Path::new("/tmp/zhiyu-mysql-versions");
        let mysql_80 = MysqlInstaller::for_version(root, "8.0.45").unwrap();
        let mysql_84 = MysqlInstaller::new(root);
        let mysql_97 = MysqlInstaller::for_version(root, "9.7").unwrap();

        assert!(mysql_80.installation_dir().ends_with("mysql/8.0"));
        assert!(mysql_84.installation_dir().ends_with("mysql/8.4"));
        assert!(mysql_97.installation_dir().ends_with("mysql/9.7"));
        assert!(MysqlInstaller::for_version(root, "7.0").is_err());
    }
}
