use crate::error::{DevBoxError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallUpdate {
    pub percent: Option<u8>,
    pub stage: String,
    pub message: String,
}

type InstallCallback = dyn Fn(InstallUpdate) + Send + Sync;

#[derive(Clone, Default)]
pub struct InstallReporter {
    callback: Option<Arc<InstallCallback>>,
}

impl fmt::Debug for InstallReporter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InstallReporter")
            .field("enabled", &self.callback.is_some())
            .finish()
    }
}

impl InstallReporter {
    pub fn new(callback: impl Fn(InstallUpdate) + Send + Sync + 'static) -> Self {
        Self {
            callback: Some(Arc::new(callback)),
        }
    }

    fn emit(&self, update: InstallUpdate) {
        if let Some(callback) = &self.callback {
            callback(update);
        }
    }
}

thread_local! {
    static ACTIVE_INSTALL_REPORTER: RefCell<Option<InstallReporter>> = const { RefCell::new(None) };
}

pub fn with_install_reporter<T>(reporter: InstallReporter, operation: impl FnOnce() -> T) -> T {
    ACTIVE_INSTALL_REPORTER.with(|active| {
        let previous = active.replace(Some(reporter));
        let result = operation();
        active.replace(previous);
        result
    })
}

pub fn report_install_progress(percent: u8, stage: &str, message: impl Into<String>) {
    report_install_update(Some(percent.min(100)), stage, message);
}

fn report_install_log(stage: &str, message: impl Into<String>) {
    report_install_update(None, stage, message);
}

fn report_install_update(percent: Option<u8>, stage: &str, message: impl Into<String>) {
    let update = InstallUpdate {
        percent,
        stage: stage.into(),
        message: message.into(),
    };
    ACTIVE_INSTALL_REPORTER.with(|active| {
        if let Some(reporter) = active.borrow().as_ref() {
            reporter.emit(update);
        }
    });
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostgresRelease {
    pub series: &'static str,
    pub version: &'static str,
    pub archive: &'static str,
    pub source_url: &'static str,
    pub sha256: &'static str,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
}

pub const POSTGRES_SERIES: &str = "17";
pub const POSTGRES_VERSION: &str = "17.10";

pub const POSTGRES_RELEASES: &[PostgresRelease] = &[
    PostgresRelease {
        series: "14",
        version: "14.23",
        archive: "postgresql-14.23.tar.bz2",
        source_url: "https://ftp.postgresql.org/pub/source/v14.23/postgresql-14.23.tar.bz2",
        sha256: "cc7216822b546330e29c2f91e123c8734a4c41795082145bb962aa712e8c94a5",
        support_label: "维护期至 2026-11",
        legacy: true,
        recommended: false,
    },
    PostgresRelease {
        series: "15",
        version: "15.18",
        archive: "postgresql-15.18.tar.bz2",
        source_url: "https://ftp.postgresql.org/pub/source/v15.18/postgresql-15.18.tar.bz2",
        sha256: "11df0df97fe3ea4ba9a791faaf39cee1d2fe571e78885b5b55d8517d27c323b4",
        support_label: "维护期至 2027-11",
        legacy: false,
        recommended: false,
    },
    PostgresRelease {
        series: "16",
        version: "16.14",
        archive: "postgresql-16.14.tar.bz2",
        source_url: "https://ftp.postgresql.org/pub/source/v16.14/postgresql-16.14.tar.bz2",
        sha256: "f6d077142737920858ce958ccdb75c6ee137a63b5b0853c70693d401ac7e3471",
        support_label: "维护期至 2028-11",
        legacy: false,
        recommended: false,
    },
    PostgresRelease {
        series: POSTGRES_SERIES,
        version: POSTGRES_VERSION,
        archive: "postgresql-17.10.tar.bz2",
        source_url: "https://ftp.postgresql.org/pub/source/v17.10/postgresql-17.10.tar.bz2",
        sha256: "078a03516dcdbdb705fecaf415ea3d13a956c589e46f09fed68a06fb00598c90",
        support_label: "维护期至 2029-11",
        legacy: false,
        recommended: true,
    },
    PostgresRelease {
        series: "18",
        version: "18.4",
        archive: "postgresql-18.4.tar.bz2",
        source_url: "https://ftp.postgresql.org/pub/source/v18.4/postgresql-18.4.tar.bz2",
        sha256: "81a81ec695fb0c7901407defaa1d2f7973617154cf27ba74e3a7ab8e64436094",
        support_label: "维护期至 2030-11",
        legacy: false,
        recommended: false,
    },
];

pub fn postgres_release(version_or_series: &str) -> Option<&'static PostgresRelease> {
    POSTGRES_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}
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
pub const NATS_SERIES: &str = "2.14";
pub const NATS_VERSION: &str = "2.14.2";
const NATS_ARCHIVE: &str = "nats-server-v2.14.2-darwin-arm64.tar.gz";
const NATS_URL: &str =
    "https://github.com/nats-io/nats-server/releases/download/v2.14.2/nats-server-v2.14.2-darwin-arm64.tar.gz";
const NATS_SHA256: &str = "1027e634ef15c3be7befed6f6645c317cefea54a51d1ff3d312e220bac55ca21";
pub const KAFKA_SERIES: &str = "0.6";
pub const KAFKA_VERSION: &str = "0.6.0";
const KAFKA_ARCHIVE: &str = "tansu-aarch64-apple-darwin.tar.gz";
const KAFKA_URL: &str =
    "https://github.com/tansu-io/tansu/releases/download/v0.6.0/tansu-aarch64-apple-darwin.tar.gz";
const KAFKA_SHA256: &str = "1129783356e6712edd20b9e2d97cdfd9d9205113ad4f027a3aa3a3d6bcf40039";
pub const MEILISEARCH_SERIES: &str = "1.50";
pub const MEILISEARCH_VERSION: &str = "1.50.0";
const MEILISEARCH_BINARY: &str = "meilisearch-macos-apple-silicon-1.50.0";
const MEILISEARCH_URL: &str =
    "https://github.com/meilisearch/meilisearch/releases/download/v1.50.0/meilisearch-macos-apple-silicon";
const MEILISEARCH_SHA256: &str = "deccb8a992e8d24c3e67118fd1166a37ef11e5b092cb1e269f2ae8a7ac8d65c8";
pub const MINIO_SERIES: &str = "2025";
pub const MINIO_VERSION: &str = "2025-09-07";
const MINIO_BINARY: &str = "minio.RELEASE.2025-09-07T16-13-09Z";
const MINIO_URL: &str =
    "https://dl.min.io/server/minio/release/darwin-arm64/archive/minio.RELEASE.2025-09-07T16-13-09Z";
const MINIO_SHA256: &str = "7c3b3039b76e55a1b80935848ed83998d5e8d317374f87851f46a019ff5c0aa4";
pub const RUSTFS_SERIES: &str = "1.0";
pub const RUSTFS_VERSION: &str = "1.0.0-beta.2";
const RUSTFS_ARCHIVE: &str = "rustfs-macos-aarch64-v1.0.0-beta.2.zip";
const RUSTFS_URL: &str =
    "https://github.com/rustfs/rustfs/releases/download/1.0.0-beta.2/rustfs-macos-aarch64-v1.0.0-beta.2.zip";
const RUSTFS_SHA256: &str = "f57cd513fa53048410f194b34d81260f76eb57305d9183159f3bdf28b4c84df5";
pub const ETCD_SERIES: &str = "3.6";
pub const ETCD_VERSION: &str = "3.6.11";
const ETCD_ARCHIVE: &str = "etcd-v3.6.11-darwin-arm64.zip";
const ETCD_URL: &str =
    "https://github.com/etcd-io/etcd/releases/download/v3.6.11/etcd-v3.6.11-darwin-arm64.zip";
const ETCD_SHA256: &str = "9617bf71a0772dd26f9d0d88e34c668d99fd11677d01593d6b365e9a0f8f3d7e";
pub const CONSUL_SERIES: &str = "1.22";
pub const CONSUL_VERSION: &str = "1.22.3";
const CONSUL_ARCHIVE: &str = "consul_1.22.3_darwin_arm64.zip";
const CONSUL_URL: &str =
    "https://releases.hashicorp.com/consul/1.22.3/consul_1.22.3_darwin_arm64.zip";
const CONSUL_SHA256: &str = "b2881e2f9c6704fdac53d54dfb3957bf0d280600541a8e8f61d807e96ea7efa0";
pub const RNACOS_SERIES: &str = "0.8";
pub const RNACOS_VERSION: &str = "0.8.5";
const RNACOS_ARCHIVE: &str = "rnacos-aarch64-apple-darwin-v0.8.5.tar.gz";
const RNACOS_URL: &str =
    "https://github.com/nacos-group/r-nacos/releases/download/v0.8.5/rnacos-aarch64-apple-darwin-v0.8.5.tar.gz";
const RNACOS_SHA256: &str = "902a073fb9318d59cede8570377212dffec0b657b64c894c2e7d29ce6a0f25ef";
pub const RABBITMQ_SERIES: &str = "4.3";
pub const RABBITMQ_VERSION: &str = "4.3.1";
const RABBITMQ_ARCHIVE: &str = "rabbitmq-server-generic-unix-4.3.1.tar.xz";
const RABBITMQ_URL: &str =
    "https://github.com/rabbitmq/rabbitmq-server/releases/download/v4.3.1/rabbitmq-server-generic-unix-4.3.1.tar.xz";
const RABBITMQ_SHA256: &str = "fc65179276a5e929258caab98d5ad1f1b10b51ccc56a128c50a00ed06e518103";
const RABBITMQ_OTP_VERSION: &str = "27.3.4.6";
const RABBITMQ_OTP_ARCHIVE: &str = "otp-aarch64-apple-darwin-27.3.4.6.tar.gz";
const RABBITMQ_OTP_URL: &str =
    "https://github.com/erlef/otp_builds/releases/download/OTP-27.3.4.6/otp-aarch64-apple-darwin.tar.gz";
const RABBITMQ_OTP_SHA256: &str =
    "82b1aa23f4a40f391e6b42cb4e9607e1e360bc2fdcb88d032b040795bb6d349f";

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
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Redis {}", self.release.version),
        );
        self.ensure_build_tools()?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/redis-server");
        if self.is_expected_version(&executable) {
            report_install_progress(90, "已安装", "目标版本已经安装，无需重复构建");
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
        report_install_progress(42, "解压源码", "正在解压 Redis 源码");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;

        let source_dir = work_dir.join(format!("redis-{}", self.release.version));
        report_install_progress(50, "准备编译", "正在应用 macOS 构建配置");
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
        report_install_progress(55, "编译程序", format!("使用 {jobs} 个并行任务编译 Redis"));
        run(&mut make, "make")?;

        report_install_progress(82, "整理文件", "正在整理 Redis 可执行程序");
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
        report_install_progress(90, "完成安装", "Redis 程序已写入版本目录");
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
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 MySQL {}", self.release.version),
        );
        ensure_macos_arm64("MySQL")?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mysqld");
        if binary_contains(&executable, &["--version"], self.release.version) {
            report_install_progress(90, "已安装", "目标版本已经安装");
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
            report_install_log("初始化数据", "MySQL 数据目录已经初始化");
            return Ok(());
        }
        report_install_progress(94, "初始化数据", "正在创建 MySQL 系统数据库");
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
        report_install_progress(45, "解压程序", "正在解压 MySQL 官方二进制包");
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
        report_install_progress(75, "整理文件", "正在写入 MySQL 版本目录");
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
        report_install_progress(90, "完成安装", "MySQL 程序安装完成");
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
        report_install_progress(3, "准备安装", format!("准备安装 DuckDB {DUCKDB_VERSION}"));
        ensure_macos_arm64("DuckDB")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/duckdb");
        if binary_contains(&executable, &["--version"], DUCKDB_VERSION) {
            report_install_progress(90, "已安装", "DuckDB 已经安装");
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
        report_install_progress(45, "解压程序", "正在解压 DuckDB");
        run(
            Command::new("/usr/bin/unzip")
                .args(["-q", "-o"])
                .arg(archive)
                .arg("-d")
                .arg(work_dir),
            "unzip",
        )?;

        let stage = work_dir.join("installation");
        report_install_progress(75, "整理文件", "正在写入 DuckDB 版本目录");
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
        report_install_progress(90, "完成安装", "DuckDB 安装完成");
        replace_installation(&stage, installation_dir)
    }
}

#[derive(Debug, Clone)]
pub struct NatsInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct KafkaInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MeilisearchInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MinioInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RustfsInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct EtcdInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ConsulInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RnacosInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RabbitmqInstaller {
    devbox_root: PathBuf,
}

impl RabbitmqInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 RabbitMQ {RABBITMQ_VERSION} 与 Erlang/OTP {RABBITMQ_OTP_VERSION}"),
        );
        ensure_macos_arm64("RabbitMQ")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("server/sbin/rabbitmq-server");
        if executable.is_file() && installation_dir.join("otp/bin/erl").is_file() {
            report_install_progress(90, "已安装", "RabbitMQ 与内置 Erlang 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "rabbitmq-{RABBITMQ_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let rabbit_archive = downloads_dir.join(RABBITMQ_ARCHIVE);
        prepare_archive(
            &rabbit_archive,
            RABBITMQ_ARCHIVE,
            RABBITMQ_URL,
            RABBITMQ_SHA256,
        )?;
        let otp_archive = downloads_dir.join(RABBITMQ_OTP_ARCHIVE);
        prepare_archive(
            &otp_archive,
            RABBITMQ_OTP_ARCHIVE,
            RABBITMQ_OTP_URL,
            RABBITMQ_OTP_SHA256,
        )?;

        let stage = work_dir.join("installation");
        let otp_dir = stage.join("otp");
        fs::create_dir_all(&otp_dir)?;
        report_install_progress(55, "解压运行时", "正在解压本地 Erlang/OTP");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(&otp_archive)
                .arg("-C")
                .arg(&otp_dir),
            "tar",
        )?;
        report_install_progress(72, "解压程序", "正在解压 RabbitMQ");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xJf"])
                .arg(&rabbit_archive)
                .arg("-C")
                .arg(&work_dir),
            "tar",
        )?;
        let server_source = work_dir.join(format!("rabbitmq_server-{RABBITMQ_VERSION}"));
        fs::rename(server_source, stage.join("server"))?;
        if !stage.join("server/sbin/rabbitmq-server").is_file()
            || !stage.join("otp/bin/erl").is_file()
        {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "verify RabbitMQ bundle".into(),
                message: "RabbitMQ 或 Erlang/OTP 压缩包结构不符合预期".into(),
            });
        }
        write_manifest(
            &stage,
            "rabbitmq",
            RABBITMQ_SERIES,
            RABBITMQ_VERSION,
            RABBITMQ_URL,
            RABBITMQ_SHA256,
            "official-binary-with-erlef-otp",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "RabbitMQ 与本地 Erlang/OTP 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/rabbitmq")
            .join(RABBITMQ_SERIES)
    }
}

impl RnacosInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(3, "准备安装", format!("准备安装 rnacos {RNACOS_VERSION}"));
        ensure_macos_arm64("rnacos")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/rnacos");
        if binary_contains(&executable, &["--version"], RNACOS_VERSION) {
            report_install_progress(90, "已安装", "rnacos 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "rnacos-{RNACOS_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let archive = downloads_dir.join(RNACOS_ARCHIVE);
        prepare_archive(&archive, RNACOS_ARCHIVE, RNACOS_URL, RNACOS_SHA256)?;
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(&archive)
                .arg("-C")
                .arg(&work_dir),
            "tar",
        )?;

        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 rnacos 版本目录");
        fs::copy(work_dir.join("rnacos"), bin_dir.join("rnacos"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("rnacos"), fs::Permissions::from_mode(0o755))?;
        }
        if !binary_contains(&bin_dir.join("rnacos"), &["--version"], RNACOS_VERSION) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "rnacos --version".into(),
                message: "downloaded binary is not the expected rnacos release".into(),
            });
        }
        write_manifest(
            &stage,
            "rnacos",
            RNACOS_SERIES,
            RNACOS_VERSION,
            RNACOS_URL,
            RNACOS_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "rnacos 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/rnacos")
            .join(RNACOS_SERIES)
    }
}

impl ConsulInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(3, "准备安装", format!("准备安装 Consul {CONSUL_VERSION}"));
        ensure_macos_arm64("Consul")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/consul");
        if binary_contains(&executable, &["version"], CONSUL_VERSION) {
            report_install_progress(90, "已安装", "Consul 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "consul-{CONSUL_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let archive = downloads_dir.join(CONSUL_ARCHIVE);
        prepare_archive(&archive, CONSUL_ARCHIVE, CONSUL_URL, CONSUL_SHA256)?;
        run(
            Command::new("/usr/bin/unzip")
                .args(["-q", "-o"])
                .arg(&archive)
                .arg("-d")
                .arg(&work_dir),
            "unzip",
        )?;

        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 Consul 版本目录");
        fs::copy(work_dir.join("consul"), bin_dir.join("consul"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("consul"), fs::Permissions::from_mode(0o755))?;
        }
        if !binary_contains(&bin_dir.join("consul"), &["version"], CONSUL_VERSION) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "consul version".into(),
                message: "downloaded binary is not the expected Consul release".into(),
            });
        }
        write_manifest(
            &stage,
            "consul",
            CONSUL_SERIES,
            CONSUL_VERSION,
            CONSUL_URL,
            CONSUL_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "Consul 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/consul")
            .join(CONSUL_SERIES)
    }
}

impl EtcdInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(3, "准备安装", format!("准备安装 etcd {ETCD_VERSION}"));
        ensure_macos_arm64("etcd")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/etcd");
        if binary_contains(&executable, &["--version"], ETCD_VERSION) {
            report_install_progress(90, "已安装", "etcd 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "etcd-{ETCD_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let archive = downloads_dir.join(ETCD_ARCHIVE);
        prepare_archive(&archive, ETCD_ARCHIVE, ETCD_URL, ETCD_SHA256)?;
        run(
            Command::new("/usr/bin/unzip")
                .args(["-q", "-o"])
                .arg(&archive)
                .arg("-d")
                .arg(&work_dir),
            "unzip",
        )?;

        let source = work_dir.join(format!("etcd-v{ETCD_VERSION}-darwin-arm64"));
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 etcd 版本目录");
        for binary in ["etcd", "etcdctl", "etcdutl"] {
            fs::copy(source.join(binary), bin_dir.join(binary))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(bin_dir.join(binary), fs::Permissions::from_mode(0o755))?;
            }
        }
        if !binary_contains(&bin_dir.join("etcd"), &["--version"], ETCD_VERSION) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "etcd --version".into(),
                message: "downloaded binary is not the expected etcd release".into(),
            });
        }
        write_manifest(
            &stage,
            "etcd",
            ETCD_SERIES,
            ETCD_VERSION,
            ETCD_URL,
            ETCD_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "etcd 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/etcd")
            .join(ETCD_SERIES)
    }
}

impl RustfsInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(3, "准备安装", format!("准备安装 RustFS {RUSTFS_VERSION}"));
        ensure_macos_arm64("RustFS")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/rustfs");
        if binary_contains(&executable, &["--version"], RUSTFS_VERSION) {
            report_install_progress(90, "已安装", "RustFS 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "rustfs-{RUSTFS_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let archive = downloads_dir.join(RUSTFS_ARCHIVE);
        prepare_archive(&archive, RUSTFS_ARCHIVE, RUSTFS_URL, RUSTFS_SHA256)?;
        run(
            Command::new("/usr/bin/unzip")
                .arg("-q")
                .arg(&archive)
                .arg("-d")
                .arg(&work_dir),
            "unzip",
        )?;

        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 RustFS 版本目录");
        fs::copy(work_dir.join("rustfs"), bin_dir.join("rustfs"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("rustfs"), fs::Permissions::from_mode(0o755))?;
        }
        if !binary_contains(&bin_dir.join("rustfs"), &["--version"], RUSTFS_VERSION) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "rustfs --version".into(),
                message: "downloaded binary is not the expected RustFS release".into(),
            });
        }
        write_manifest(
            &stage,
            "rustfs",
            RUSTFS_SERIES,
            RUSTFS_VERSION,
            RUSTFS_URL,
            RUSTFS_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "RustFS 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/rustfs")
            .join(RUSTFS_SERIES)
    }
}

impl MinioInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(3, "准备安装", format!("准备安装 MinIO {MINIO_VERSION}"));
        ensure_macos_arm64("MinIO")?;
        ensure_tools(&["/usr/bin/curl"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/minio");
        if binary_contains(&executable, &["--version"], "RELEASE.2025-09-07") {
            report_install_progress(90, "已安装", "MinIO 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "minio-{MINIO_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let download = downloads_dir.join(MINIO_BINARY);
        prepare_archive(&download, MINIO_BINARY, MINIO_URL, MINIO_SHA256)?;
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 MinIO 版本目录");
        fs::copy(download, bin_dir.join("minio"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("minio"), fs::Permissions::from_mode(0o755))?;
        }
        if !binary_contains(&bin_dir.join("minio"), &["--version"], "RELEASE.2025-09-07") {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "minio --version".into(),
                message: "downloaded binary is not the expected MinIO release".into(),
            });
        }
        write_manifest(
            &stage,
            "minio",
            MINIO_SERIES,
            MINIO_VERSION,
            MINIO_URL,
            MINIO_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "MinIO 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/minio")
            .join(MINIO_SERIES)
    }
}

impl MeilisearchInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Meilisearch {MEILISEARCH_VERSION}"),
        );
        ensure_macos_arm64("Meilisearch")?;
        ensure_tools(&["/usr/bin/curl"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/meilisearch");
        if binary_contains(&executable, &["--version"], MEILISEARCH_VERSION) {
            report_install_progress(90, "已安装", "Meilisearch 已经安装");
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
                .expect("Meilisearch installation has a parent"),
        )?;

        let download = downloads_dir.join(MEILISEARCH_BINARY);
        prepare_archive(
            &download,
            MEILISEARCH_BINARY,
            MEILISEARCH_URL,
            MEILISEARCH_SHA256,
        )?;
        let work_dir = temp_root.join(format!(
            "meilisearch-{MEILISEARCH_VERSION}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 Meilisearch 版本目录");
        fs::copy(download, bin_dir.join("meilisearch"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                bin_dir.join("meilisearch"),
                fs::Permissions::from_mode(0o755),
            )?;
        }

        if !binary_contains(
            &bin_dir.join("meilisearch"),
            &["--version"],
            MEILISEARCH_VERSION,
        ) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "meilisearch --version".into(),
                message: format!("downloaded binary is not Meilisearch {MEILISEARCH_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "meilisearch",
            MEILISEARCH_SERIES,
            MEILISEARCH_VERSION,
            MEILISEARCH_URL,
            MEILISEARCH_SHA256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "Meilisearch 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations")
            .join("meilisearch")
            .join(MEILISEARCH_SERIES)
    }
}

impl NatsInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(3, "准备安装", format!("准备安装 NATS {NATS_VERSION}"));
        ensure_macos_arm64("NATS")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/nats-server");
        if binary_contains(&executable, &["--version"], NATS_VERSION) {
            report_install_progress(90, "已安装", "NATS 已经安装");
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
                .expect("NATS installation has a parent"),
        )?;

        let archive = downloads_dir.join(NATS_ARCHIVE);
        prepare_archive(&archive, NATS_ARCHIVE, NATS_URL, NATS_SHA256)?;
        let work_dir = temp_root.join(format!(
            "nats-{NATS_VERSION}-{}-{}",
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
            .join("nats")
            .join(NATS_SERIES)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        report_install_progress(45, "解压程序", "正在解压 NATS 官方二进制包");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;

        let source = work_dir
            .join(format!("nats-server-v{NATS_VERSION}-darwin-arm64"))
            .join("nats-server");
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        report_install_progress(75, "整理文件", "正在写入 NATS 版本目录");
        fs::create_dir_all(&bin_dir)?;
        fs::copy(source, bin_dir.join("nats-server"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                bin_dir.join("nats-server"),
                fs::Permissions::from_mode(0o755),
            )?;
        }

        if !binary_contains(&bin_dir.join("nats-server"), &["--version"], NATS_VERSION) {
            return Err(DevBoxError::CommandFailed {
                command: "nats-server --version".into(),
                message: format!("downloaded binary is not NATS {NATS_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "nats",
            NATS_SERIES,
            NATS_VERSION,
            NATS_URL,
            NATS_SHA256,
            "official-binary",
        )?;
        report_install_progress(90, "完成安装", "NATS 安装完成");
        replace_installation(&stage, installation_dir)
    }
}

impl KafkaInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
        }
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Kafka Sandbox (Tansu {KAFKA_VERSION})"),
        );
        ensure_macos_arm64("Kafka Sandbox")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/tansu");
        if binary_contains(&executable, &["--version"], KAFKA_VERSION) {
            report_install_progress(90, "已安装", "Kafka Sandbox 已经安装");
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
                .expect("Kafka installation has a parent"),
        )?;

        let archive = downloads_dir.join(KAFKA_ARCHIVE);
        prepare_archive(&archive, KAFKA_ARCHIVE, KAFKA_URL, KAFKA_SHA256)?;
        let work_dir = temp_root.join(format!(
            "kafka-{KAFKA_VERSION}-{}-{}",
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
            .join("kafka")
            .join(KAFKA_SERIES)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        report_install_progress(45, "解压程序", "正在解压 Tansu 官方二进制包");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;

        let source = work_dir.join("bin/tansu");
        if !source.is_file() {
            return Err(DevBoxError::CommandFailed {
                command: "tar".into(),
                message: "安装包中未找到 bin/tansu".into(),
            });
        }
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        report_install_progress(75, "整理文件", "正在写入 Kafka Sandbox 版本目录");
        fs::create_dir_all(&bin_dir)?;
        fs::copy(source, bin_dir.join("tansu"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("tansu"), fs::Permissions::from_mode(0o755))?;
        }

        if !binary_contains(&bin_dir.join("tansu"), &["--version"], KAFKA_VERSION) {
            return Err(DevBoxError::CommandFailed {
                command: "tansu --version".into(),
                message: format!("downloaded binary is not Tansu {KAFKA_VERSION}"),
            });
        }
        write_manifest(
            &stage,
            "kafka",
            KAFKA_SERIES,
            KAFKA_VERSION,
            KAFKA_URL,
            KAFKA_SHA256,
            "tansu-official-binary",
        )?;
        report_install_progress(90, "完成安装", "Kafka Sandbox 安装完成");
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
        report_install_progress(3, "准备安装", format!("准备安装 Mailpit {MAILPIT_VERSION}"));
        ensure_macos_arm64("Mailpit")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mailpit");
        if binary_contains(&executable, &["version"], MAILPIT_VERSION) {
            report_install_progress(90, "已安装", "Mailpit 已经安装");
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
        report_install_progress(45, "解压程序", "正在解压 Mailpit");
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
        report_install_progress(75, "整理文件", "正在写入 Mailpit 版本目录");
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
        report_install_progress(90, "完成安装", "Mailpit 安装完成");
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
        report_install_progress(3, "准备安装", format!("准备安装 MongoDB {MONGODB_VERSION}"));
        ensure_macos_arm64("MongoDB")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mongod");
        if binary_contains(&executable, &["--version"], MONGODB_VERSION) {
            report_install_progress(90, "已安装", "MongoDB 已经安装");
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
        report_install_progress(45, "解压程序", "正在解压 MongoDB");
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
        report_install_progress(75, "整理文件", "正在写入 MongoDB 版本目录");
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
        report_install_progress(90, "完成安装", "MongoDB 安装完成");
        replace_installation(&stage, installation_dir)
    }
}

#[derive(Debug, Clone)]
pub struct PostgresInstaller {
    devbox_root: PathBuf,
    release: &'static PostgresRelease,
}

impl PostgresInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: postgres_release(POSTGRES_VERSION)
                .expect("default PostgreSQL release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = postgres_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!(
                "unsupported PostgreSQL version: {version_or_series}"
            ))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static PostgresRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 PostgreSQL {}", self.release.version),
        );
        ensure_macos_arm64("PostgreSQL")?;
        ensure_tools(&[
            "/usr/bin/curl",
            "/usr/bin/tar",
            "/usr/bin/make",
            "/usr/bin/cc",
        ])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/postgres");
        if binary_contains(&executable, &["--version"], self.release.version)
            && binary_contains(
                &installation_dir.join("bin/initdb"),
                &["--version"],
                self.release.version,
            )
        {
            report_install_progress(90, "已安装", "目标版本已经安装，无需重复编译");
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

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "postgres-{}-{}-{}",
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

    pub fn initialize(&self, data_dir: &Path) -> Result<()> {
        if data_dir.join("PG_VERSION").is_file() {
            report_install_log("初始化数据", "PostgreSQL 数据目录已经初始化");
            return Ok(());
        }
        report_install_progress(94, "初始化数据", "正在执行 initdb 创建数据库集群");
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation_dir = self.installation_dir();
        binary_contains(
            &installation_dir.join("bin/postgres"),
            &["--version"],
            self.release.version,
        ) && binary_contains(
            &installation_dir.join("bin/initdb"),
            &["--version"],
            self.release.version,
        )
    }

    fn build_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        report_install_progress(42, "解压源码", "正在解压 PostgreSQL 源码");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xjf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source = work_dir.join(format!("postgresql-{}", self.release.version));
        let destination_root = work_dir.join("destination");
        let relative_installation = installation_dir.strip_prefix("/").map_err(|_| {
            DevBoxError::InvalidConfig("PostgreSQL installation path must be absolute".into())
        })?;
        let stage = destination_root.join(relative_installation);
        report_install_progress(50, "配置构建", "正在检查编译环境并生成构建配置");
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
        report_install_progress(
            58,
            "编译程序",
            format!("使用 {jobs} 个并行任务编译 PostgreSQL"),
        );
        run(
            Command::new("/usr/bin/make")
                .arg("-C")
                .arg(&source)
                .arg(format!("-j{jobs}")),
            "make",
        )?;
        report_install_progress(80, "安装程序", "正在整理 PostgreSQL 编译产物");
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
            self.release.version,
        ) || !binary_contains(
            &stage.join("bin/initdb"),
            &["--version"],
            self.release.version,
        ) {
            return Err(DevBoxError::CommandFailed {
                command: "postgres --version".into(),
                message: format!("built binary is not PostgreSQL {}", self.release.version),
            });
        }
        write_manifest(
            &stage,
            "postgres",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-source",
        )?;
        report_install_progress(90, "完成安装", "PostgreSQL 程序已写入版本目录");
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
    report_install_progress(8, "检查缓存", format!("检查安装包缓存：{archive_name}"));
    if archive.is_file() {
        if sha256(archive)? == expected_sha256 {
            report_install_progress(35, "使用缓存", format!("安装包校验通过：{archive_name}"));
            return Ok(());
        }
        report_install_log("检查缓存", "缓存校验失败，将重新下载");
        fs::remove_file(archive)?;
    }

    let partial = archive.with_file_name(format!("{archive_name}.partial"));
    let devbox_root = archive
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new(""));
    let settings = installer_download_settings(devbox_root);
    let configured_mirror = configured_download_mirror(devbox_root, &settings);
    let candidates = download_candidates(
        source_url,
        archive_name,
        configured_mirror.as_deref(),
        settings.public_github_mirror && std::env::var_os("ZHIYU_DISABLE_PUBLIC_MIRROR").is_none(),
    );
    let _download_permit = DownloadPermit::acquire(settings.download_concurrency);
    let mut failures = Vec::new();

    for (index, candidate) in candidates.iter().enumerate() {
        let _ = fs::remove_file(&partial);
        report_install_progress(
            12,
            "下载安装包",
            format!(
                "尝试下载源 {}/{}（{}）：{}",
                index + 1,
                candidates.len(),
                candidate.label,
                candidate.url
            ),
        );
        let mut command = Command::new("/usr/bin/curl");
        command
            .args(["--fail", "--location", "--silent", "--show-error"])
            .arg("--connect-timeout")
            .arg(settings.download_timeout_seconds.min(15).to_string())
            .arg("--max-time")
            .arg(settings.download_timeout_seconds.to_string())
            .args(["--retry", "1"])
            .arg("--output")
            .arg(&partial);
        if !candidate.official {
            command.args(["--speed-time", "15", "--speed-limit", "16384"]);
        }
        let output = command.arg(&candidate.url).output()?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            failures.push(format!("{}: {}", candidate.label, message));
            report_install_log(
                "切换下载源",
                format!("{}不可用，自动尝试下一个下载源", candidate.label),
            );
            continue;
        }

        report_install_progress(30, "校验安装包", "下载完成，正在计算 SHA-256");
        let actual = sha256(&partial)?;
        if actual != expected_sha256 {
            failures.push(format!("{}: SHA-256 校验不一致", candidate.label));
            report_install_log(
                "切换下载源",
                format!("{}返回的文件校验失败，已丢弃并切换", candidate.label),
            );
            continue;
        }
        fs::rename(&partial, archive)?;
        report_install_progress(
            35,
            "安装包就绪",
            format!("通过{}下载并校验成功：{archive_name}", candidate.label),
        );
        return Ok(());
    }
    let _ = fs::remove_file(&partial);
    Err(DevBoxError::CommandFailed {
        command: "curl".into(),
        message: format!("所有下载源均失败：{}", failures.join("；")),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadCandidate {
    label: String,
    url: String,
    official: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct InstallerDownloadSettings {
    download_mirror: Option<String>,
    public_github_mirror: bool,
    download_concurrency: usize,
    download_timeout_seconds: u64,
}

impl Default for InstallerDownloadSettings {
    fn default() -> Self {
        Self {
            download_mirror: None,
            public_github_mirror: true,
            download_concurrency: 2,
            download_timeout_seconds: 180,
        }
    }
}

fn installer_download_settings(devbox_root: &Path) -> InstallerDownloadSettings {
    let mut settings: InstallerDownloadSettings =
        fs::read(devbox_root.join("installer-settings.json"))
            .ok()
            .and_then(|contents| serde_json::from_slice(&contents).ok())
            .unwrap_or_default();
    settings.download_concurrency = settings.download_concurrency.clamp(1, 4);
    settings.download_timeout_seconds = settings.download_timeout_seconds.clamp(15, 600);
    settings
}

fn configured_download_mirror(
    devbox_root: &Path,
    settings: &InstallerDownloadSettings,
) -> Option<String> {
    std::env::var("ZHIYU_DOWNLOAD_MIRROR")
        .ok()
        .or_else(|| settings.download_mirror.clone())
        .or_else(|| fs::read_to_string(devbox_root.join("download-mirror.txt")).ok())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| value.starts_with("https://"))
}

static ACTIVE_DOWNLOADS: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

struct DownloadPermit;

impl DownloadPermit {
    fn acquire(limit: usize) -> Self {
        let (lock, ready) = &ACTIVE_DOWNLOADS;
        let mut active = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while *active >= limit {
            active = ready
                .wait(active)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *active += 1;
        Self
    }
}

impl Drop for DownloadPermit {
    fn drop(&mut self) {
        let (lock, ready) = &ACTIVE_DOWNLOADS;
        let mut active = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *active = active.saturating_sub(1);
        ready.notify_one();
    }
}

fn download_candidates(
    source_url: &str,
    archive_name: &str,
    configured_mirror: Option<&str>,
    public_mirror_enabled: bool,
) -> Vec<DownloadCandidate> {
    let mut candidates = Vec::new();
    if let Some(mirror) = configured_mirror {
        candidates.push(DownloadCandidate {
            label: "自定义镜像".into(),
            url: format!("{}/{archive_name}", mirror.trim_end_matches('/')),
            official: false,
        });
    }
    if public_mirror_enabled && source_url.starts_with("https://github.com/") {
        candidates.push(DownloadCandidate {
            label: "GitHub 公共加速".into(),
            url: format!("https://gh-proxy.com/{source_url}"),
            official: false,
        });
    }
    candidates.push(DownloadCandidate {
        label: "官方源".into(),
        url: source_url.into(),
        official: true,
    });
    candidates
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
    if let Some(parent) = installation_dir.parent() {
        fs::create_dir_all(parent)?;
    }
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
    report_install_log("执行命令", format!("开始执行：{name}"));
    let output = command.output()?;
    if output.status.success() {
        report_install_log("执行命令", format!("执行完成：{name}"));
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
    let message = tail_chars(&message, 32 * 1024);
    report_install_log("命令失败", format!("{name}：{message}"));
    Err(DevBoxError::CommandFailed {
        command: name.into(),
        message,
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
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;

    #[test]
    fn install_reporter_preserves_progress_and_log_updates() {
        let updates = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&updates);
        let reporter = InstallReporter::new(move |update| {
            captured.lock().unwrap().push(update);
        });

        with_install_reporter(reporter, || {
            report_install_progress(25, "下载", "正在下载安装包");
            report_install_log("执行命令", "开始执行：tar");
        });

        assert_eq!(
            *updates.lock().unwrap(),
            vec![
                InstallUpdate {
                    percent: Some(25),
                    stage: "下载".into(),
                    message: "正在下载安装包".into(),
                },
                InstallUpdate {
                    percent: None,
                    stage: "执行命令".into(),
                    message: "开始执行：tar".into(),
                },
            ]
        );
    }

    #[test]
    fn github_downloads_prefer_configured_and_public_mirrors() {
        let source =
            "https://github.com/example/project/releases/download/v1.0/project-v1.0.tar.gz";
        let candidates = download_candidates(
            source,
            "project-v1.0.tar.gz",
            Some("https://mirror.example.com/zhiyu/"),
            true,
        );

        assert_eq!(
            candidates,
            vec![
                DownloadCandidate {
                    label: "自定义镜像".into(),
                    url: "https://mirror.example.com/zhiyu/project-v1.0.tar.gz".into(),
                    official: false,
                },
                DownloadCandidate {
                    label: "GitHub 公共加速".into(),
                    url: format!("https://gh-proxy.com/{source}"),
                    official: false,
                },
                DownloadCandidate {
                    label: "官方源".into(),
                    url: source.into(),
                    official: true,
                },
            ]
        );
    }

    #[test]
    fn public_mirror_only_applies_to_github_and_can_be_disabled() {
        let non_github = download_candidates(
            "https://cdn.example.com/project.tar.gz",
            "project.tar.gz",
            None,
            true,
        );
        assert_eq!(non_github.len(), 1);
        assert!(non_github[0].official);

        let github = download_candidates(
            "https://github.com/example/project/releases/download/v1/project.tar.gz",
            "project.tar.gz",
            None,
            false,
        );
        assert_eq!(github.len(), 1);
        assert!(github[0].official);
    }

    #[test]
    fn installer_download_settings_are_loaded_and_bounded() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("installer-settings.json"),
            r#"{
                "downloadMirror": "https://mirror.example.com/packages",
                "publicGithubMirror": false,
                "downloadConcurrency": 9,
                "downloadTimeoutSeconds": 5
            }"#,
        )
        .unwrap();

        let settings = installer_download_settings(temp.path());

        assert_eq!(
            settings.download_mirror.as_deref(),
            Some("https://mirror.example.com/packages")
        );
        assert!(!settings.public_github_mirror);
        assert_eq!(settings.download_concurrency, 4);
        assert_eq!(settings.download_timeout_seconds, 15);
    }

    #[test]
    fn replacing_an_installation_creates_the_service_parent_directory() {
        let temp = TempDir::new().unwrap();
        let stage = temp.path().join("tmp/installation");
        let executable = stage.join("bin/service");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"binary").unwrap();

        let installation = temp.path().join("installations/service/1.0");
        assert!(!installation.parent().unwrap().exists());

        replace_installation(&stage, &installation).unwrap();

        assert_eq!(
            fs::read(installation.join("bin/service")).unwrap(),
            b"binary"
        );
        assert!(!stage.exists());
    }

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
    fn postgres_constants_target_the_same_release() {
        assert_eq!(POSTGRES_RELEASES.len(), 5);
        for release in POSTGRES_RELEASES {
            assert!(release.source_url.ends_with(release.archive));
            assert!(release.archive.contains(release.version));
            assert_eq!(release.sha256.len(), 64);
            assert_eq!(postgres_release(release.version), Some(release));
            assert_eq!(postgres_release(release.series), Some(release));
        }
        assert!(postgres_release("13").is_none());
        assert!(postgres_release(POSTGRES_VERSION).is_some_and(|release| release.recommended));
    }

    #[test]
    fn postgres_installers_use_independent_series_directories() {
        let root = Path::new("/tmp/zhiyu-postgres-versions");
        let postgres_14 = PostgresInstaller::for_version(root, "14.23").unwrap();
        let postgres_17 = PostgresInstaller::new(root);
        let postgres_18 = PostgresInstaller::for_version(root, "18").unwrap();

        assert!(postgres_14.installation_dir().ends_with("postgres/14"));
        assert!(postgres_17.installation_dir().ends_with("postgres/17"));
        assert!(postgres_18.installation_dir().ends_with("postgres/18"));
        assert!(PostgresInstaller::for_version(root, "13").is_err());
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
