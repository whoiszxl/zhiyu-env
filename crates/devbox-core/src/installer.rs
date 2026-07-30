use crate::error::{DevBoxError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Condvar, Mutex,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

#[derive(Clone, Debug, Default)]
pub struct InstallCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl InstallCancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

thread_local! {
    static ACTIVE_INSTALL_REPORTER: RefCell<Option<InstallReporter>> = const { RefCell::new(None) };
    static ACTIVE_INSTALL_CANCELLATION: RefCell<Option<InstallCancellationToken>> = const { RefCell::new(None) };
}

pub fn with_install_reporter<T>(reporter: InstallReporter, operation: impl FnOnce() -> T) -> T {
    with_install_context(reporter, InstallCancellationToken::default(), operation)
}

pub fn with_install_context<T>(
    reporter: InstallReporter,
    cancellation: InstallCancellationToken,
    operation: impl FnOnce() -> T,
) -> T {
    ACTIVE_INSTALL_REPORTER.with(|active| {
        let previous = active.replace(Some(reporter));
        ACTIVE_INSTALL_CANCELLATION.with(|active_cancellation| {
            let previous_cancellation = active_cancellation.replace(Some(cancellation));
            let result = operation();
            active_cancellation.replace(previous_cancellation);
            active.replace(previous);
            result
        })
    })
}

pub fn check_install_cancelled() -> Result<()> {
    let cancelled = ACTIVE_INSTALL_CANCELLATION.with(|active| {
        active
            .borrow()
            .as_ref()
            .is_some_and(InstallCancellationToken::is_cancelled)
    });
    if cancelled {
        Err(DevBoxError::InstallCancelled)
    } else {
        Ok(())
    }
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
pub const INFLUXDB_SERIES: &str = "3.10";
pub const INFLUXDB_VERSION: &str = "3.10.5";
const INFLUXDB_ARCHIVE: &str = "influxdb3-core-3.10.5_darwin_arm64.tar.gz";
const INFLUXDB_URL: &str =
    "https://dl.influxdata.com/influxdb/releases/influxdb3-core-3.10.5_darwin_arm64.tar.gz";
const INFLUXDB_SHA256: &str = "4723dc749587f3afe9153fcff50ad27e46552115741fd45ff85a16b5187d10ac";
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
pub const ACTIVEMQ_SERIES: &str = "6.2";
pub const ACTIVEMQ_VERSION: &str = "6.2.8";
const ACTIVEMQ_ARCHIVE: &str = "apache-activemq-6.2.8-bin.tar.gz";
const ACTIVEMQ_URL: &str =
    "https://downloads.apache.org/activemq/6.2.8/apache-activemq-6.2.8-bin.tar.gz";
const ACTIVEMQ_SHA256: &str = "9d8751ba826983b1b7a4fe0e48a89fccaf499ed341cbefb9c1d7edb4ab239305";
pub const CADDY_SERIES: &str = "2.11";
pub const CADDY_VERSION: &str = "2.11.4";
const CADDY_ARCHIVE: &str = "caddy_2.11.4_mac_arm64.tar.gz";
const CADDY_URL: &str =
    "https://github.com/caddyserver/caddy/releases/download/v2.11.4/caddy_2.11.4_mac_arm64.tar.gz";
const CADDY_SHA256: &str = "9efb0af2d6cf09cfb5053c0e51721b9b3d4956d346234f39368d943d25a3c9a7";
pub const FTP_SERIES: &str = "2.7";
pub const FTP_VERSION: &str = "2.7.5";
const FTP_ARCHIVE: &str = "sftpgo_v2.7.5_macOS_arm64.tar.xz";
const FTP_URL: &str =
    "https://github.com/drakkan/sftpgo/releases/download/v2.7.5/sftpgo_v2.7.5_macOS_arm64.tar.xz";
const FTP_SHA256: &str = "3041e313048612fdef50d85277e7ebe04d0ce7b1187a31a241751c2f142185bf";

#[derive(Debug, Clone, Copy)]
pub struct VerifiedBinaryRelease {
    pub series: &'static str,
    pub version: &'static str,
    pub archive: &'static str,
    pub source_url: &'static str,
    pub sha256: &'static str,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
}

pub const MAILPIT_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "1.28",
        version: "1.28.4",
        archive: "mailpit-darwin-arm64-v1.28.4.tar.gz",
        source_url:
            "https://github.com/axllent/mailpit/releases/download/v1.28.4/mailpit-darwin-arm64.tar.gz",
        sha256: "eb3312168cb593c91b1de81d6f1c4ec134d5da7031c6c9f7d3b423d366293088",
        support_label: "历史稳定版",
        legacy: true,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: "1.29",
        version: "1.29.2",
        archive: "mailpit-darwin-arm64-v1.29.2.tar.gz",
        source_url:
            "https://github.com/axllent/mailpit/releases/download/v1.29.2/mailpit-darwin-arm64.tar.gz",
        sha256: "4cc025b6e4757020d030ab892e7b42dec3e6a10ba49bf8ed93677890d13e86e5",
        support_label: "兼容版本",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: MAILPIT_SERIES,
        version: MAILPIT_VERSION,
        archive: MAILPIT_ARCHIVE,
        source_url: MAILPIT_URL,
        sha256: MAILPIT_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const NATS_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "2.11",
        version: "2.11.17",
        archive: "nats-server-v2.11.17-darwin-arm64.tar.gz",
        source_url:
            "https://github.com/nats-io/nats-server/releases/download/v2.11.17/nats-server-v2.11.17-darwin-arm64.tar.gz",
        sha256: "e17d11c5e9cb0824e003013304ba3d96ca24adbff9998084c746869c0347be9b",
        support_label: "长期维护分支",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: "2.12",
        version: "2.12.8",
        archive: "nats-server-v2.12.8-darwin-arm64.tar.gz",
        source_url:
            "https://github.com/nats-io/nats-server/releases/download/v2.12.8/nats-server-v2.12.8-darwin-arm64.tar.gz",
        sha256: "4556a7f617eb532587790344629a6a95261099b236524b764d1f10d6c14aba1a",
        support_label: "稳定维护版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: NATS_SERIES,
        version: NATS_VERSION,
        archive: NATS_ARCHIVE,
        source_url: NATS_URL,
        sha256: NATS_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const ETCD_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "3.5",
        version: "3.5.21",
        archive: "etcd-v3.5.21-darwin-arm64.zip",
        source_url:
            "https://github.com/etcd-io/etcd/releases/download/v3.5.21/etcd-v3.5.21-darwin-arm64.zip",
        sha256: "d0ffb98e3671b1de1ca82b1fb6d7548661573519174470a33b6efccacb494f06",
        support_label: "长期维护分支",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: ETCD_SERIES,
        version: ETCD_VERSION,
        archive: ETCD_ARCHIVE,
        source_url: ETCD_URL,
        sha256: ETCD_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const CADDY_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "2.10",
        version: "2.10.2",
        archive: "caddy_2.10.2_mac_arm64.tar.gz",
        source_url:
            "https://github.com/caddyserver/caddy/releases/download/v2.10.2/caddy_2.10.2_mac_arm64.tar.gz",
        sha256: "cc9ad20742ea7bfee5dd1d435d42ab7fcf8592294f9ec43bf08fd21cbe448bc4",
        support_label: "兼容稳定版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: CADDY_SERIES,
        version: CADDY_VERSION,
        archive: CADDY_ARCHIVE,
        source_url: CADDY_URL,
        sha256: CADDY_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const MONGODB_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "7.0",
        version: "7.0.30",
        archive: "mongodb-macos-arm64-7.0.30.tgz",
        source_url: "https://fastdl.mongodb.org/osx/mongodb-macos-arm64-7.0.30.tgz",
        sha256: "6c6c6fcfc38e025d48e99c6f5892a9e6aa8012ccb08ce247787936423d634dd1",
        support_label: "长期维护分支",
        legacy: true,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: MONGODB_SERIES,
        version: MONGODB_VERSION,
        archive: MONGODB_ARCHIVE,
        source_url: MONGODB_URL,
        sha256: MONGODB_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const MEILISEARCH_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "1.45",
        version: "1.45.1",
        archive: "meilisearch-macos-apple-silicon-1.45.1",
        source_url:
            "https://github.com/meilisearch/meilisearch/releases/download/v1.45.1/meilisearch-macos-apple-silicon",
        sha256: "a5b311f90b84ea8df12854d28dd2c0704e5784df24130c90a62df2344cedbf4f",
        support_label: "兼容稳定版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: MEILISEARCH_SERIES,
        version: MEILISEARCH_VERSION,
        archive: MEILISEARCH_BINARY,
        source_url: MEILISEARCH_URL,
        sha256: MEILISEARCH_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const INFLUXDB_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "3.8",
        version: "3.8.3",
        archive: "influxdb3-core-3.8.3_darwin_arm64.tar.gz",
        source_url:
            "https://dl.influxdata.com/influxdb/releases/influxdb3-core-3.8.3_darwin_arm64.tar.gz",
        sha256: "b72c0d387bbdf8e15ccf61fca4445a0e06dd7faf2e83213e843032fba3384a1a",
        support_label: "兼容稳定版",
        legacy: true,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: "3.9",
        version: "3.9.0",
        archive: "influxdb3-core-3.9.0_darwin_arm64.tar.gz",
        source_url:
            "https://dl.influxdata.com/influxdb/releases/influxdb3-core-3.9.0_darwin_arm64.tar.gz",
        sha256: "b28d7856fb30cf72cbc19f50d55a41632498b669419fce5cc55deafd4f4953a9",
        support_label: "稳定版本",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: INFLUXDB_SERIES,
        version: INFLUXDB_VERSION,
        archive: INFLUXDB_ARCHIVE,
        source_url: INFLUXDB_URL,
        sha256: INFLUXDB_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const MINIO_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "2024",
        version: "2024-12-18",
        archive: "minio.RELEASE.2024-12-18T13-15-44Z",
        source_url:
            "https://dl.min.io/server/minio/release/darwin-arm64/archive/minio.RELEASE.2024-12-18T13-15-44Z",
        sha256: "af079f5c4e2cb855f8dd0c86eea57d1412c81c458bd7fcb8421bec34a4143fef",
        support_label: "2024 兼容版本",
        legacy: true,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: MINIO_SERIES,
        version: MINIO_VERSION,
        archive: MINIO_BINARY,
        source_url: MINIO_URL,
        sha256: MINIO_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const RUSTFS_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "1.0-beta.1",
        version: "1.0.0-beta.1",
        archive: "rustfs-macos-aarch64-v1.0.0-beta.1.zip",
        source_url:
            "https://github.com/rustfs/rustfs/releases/download/1.0.0-beta.1/rustfs-macos-aarch64-v1.0.0-beta.1.zip",
        sha256: "8d1c2f6340163f9876472d17328aa78527fbcc0c5c09b7b2860edfdfce5d40cf",
        support_label: "早期测试版",
        legacy: true,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: RUSTFS_SERIES,
        version: RUSTFS_VERSION,
        archive: RUSTFS_ARCHIVE,
        source_url: RUSTFS_URL,
        sha256: RUSTFS_SHA256,
        support_label: "当前测试版",
        legacy: false,
        recommended: true,
    },
];

pub const CONSUL_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "1.21",
        version: "1.21.5",
        archive: "consul_1.21.5_darwin_arm64.zip",
        source_url: "https://releases.hashicorp.com/consul/1.21.5/consul_1.21.5_darwin_arm64.zip",
        sha256: "36e141a33a3b34628ff02ec256528109a54712e9dacff3a3bd11d7e7d17d05f2",
        support_label: "兼容稳定版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: CONSUL_SERIES,
        version: CONSUL_VERSION,
        archive: CONSUL_ARCHIVE,
        source_url: CONSUL_URL,
        sha256: CONSUL_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const RNACOS_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "0.8.4",
        version: "0.8.4",
        archive: "rnacos-aarch64-apple-darwin-v0.8.4.tar.gz",
        source_url:
            "https://github.com/nacos-group/r-nacos/releases/download/v0.8.4/rnacos-aarch64-apple-darwin-v0.8.4.tar.gz",
        sha256: "5126b2a7968950130a2c1629858f92b1d758ddff4194440a998d4e53a90bc86c",
        support_label: "兼容稳定版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: RNACOS_SERIES,
        version: RNACOS_VERSION,
        archive: RNACOS_ARCHIVE,
        source_url: RNACOS_URL,
        sha256: RNACOS_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const RABBITMQ_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "4.2",
        version: "4.2.4",
        archive: "rabbitmq-server-generic-unix-4.2.4.tar.xz",
        source_url:
            "https://github.com/rabbitmq/rabbitmq-server/releases/download/v4.2.4/rabbitmq-server-generic-unix-4.2.4.tar.xz",
        sha256: "7cc2ce2dea3c35fc1cf9ad48bca4a534e394af9e6c77c71d94eeb07775ec7832",
        support_label: "兼容稳定版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: RABBITMQ_SERIES,
        version: RABBITMQ_VERSION,
        archive: RABBITMQ_ARCHIVE,
        source_url: RABBITMQ_URL,
        sha256: RABBITMQ_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

pub const ACTIVEMQ_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: ACTIVEMQ_SERIES,
        version: ACTIVEMQ_VERSION,
        archive: ACTIVEMQ_ARCHIVE,
        source_url: ACTIVEMQ_URL,
        sha256: ACTIVEMQ_SHA256,
        support_label: "稳定支持版 · 需要 Java 17 或 21",
        legacy: false,
        recommended: true,
    },
    VerifiedBinaryRelease {
        series: "6.3",
        version: "6.3.0",
        archive: "apache-activemq-6.3.0-bin.tar.gz",
        source_url: "https://downloads.apache.org/activemq/6.3.0/apache-activemq-6.3.0-bin.tar.gz",
        sha256: "a6ca29177d648b8961f66c323d33cfe2ca3774f943bf589f97371a3d868bb9e7",
        support_label: "最新特性版 · 需要 Java 25",
        legacy: false,
        recommended: false,
    },
];

pub const FTP_RELEASES: &[VerifiedBinaryRelease] = &[
    VerifiedBinaryRelease {
        series: "2.7.4",
        version: "2.7.4",
        archive: "sftpgo_v2.7.4_macOS_arm64.tar.xz",
        source_url:
            "https://github.com/drakkan/sftpgo/releases/download/v2.7.4/sftpgo_v2.7.4_macOS_arm64.tar.xz",
        sha256: "b2881d57bf77f77bcff7fc82cef72bc629b52efa082b4fcd7b0da0ba76e55166",
        support_label: "兼容稳定版",
        legacy: false,
        recommended: false,
    },
    VerifiedBinaryRelease {
        series: FTP_SERIES,
        version: FTP_VERSION,
        archive: FTP_ARCHIVE,
        source_url: FTP_URL,
        sha256: FTP_SHA256,
        support_label: "当前稳定版",
        legacy: false,
        recommended: true,
    },
];

macro_rules! verified_release_lookup {
    ($name:ident, $catalog:ident) => {
        pub fn $name(version_or_series: &str) -> Option<&'static VerifiedBinaryRelease> {
            $catalog.iter().find(|release| {
                release.version == version_or_series || release.series == version_or_series
            })
        }
    };
}

verified_release_lookup!(mongodb_release, MONGODB_RELEASES);
verified_release_lookup!(meilisearch_release, MEILISEARCH_RELEASES);
verified_release_lookup!(influxdb_release, INFLUXDB_RELEASES);
verified_release_lookup!(minio_release, MINIO_RELEASES);
verified_release_lookup!(rustfs_release, RUSTFS_RELEASES);
verified_release_lookup!(consul_release, CONSUL_RELEASES);
verified_release_lookup!(rnacos_release, RNACOS_RELEASES);
verified_release_lookup!(rabbitmq_release, RABBITMQ_RELEASES);
verified_release_lookup!(activemq_release, ACTIVEMQ_RELEASES);
verified_release_lookup!(ftp_release, FTP_RELEASES);

pub fn mailpit_release(version_or_series: &str) -> Option<&'static VerifiedBinaryRelease> {
    MAILPIT_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

pub fn nats_release(version_or_series: &str) -> Option<&'static VerifiedBinaryRelease> {
    NATS_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

pub fn etcd_release(version_or_series: &str) -> Option<&'static VerifiedBinaryRelease> {
    ETCD_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

pub fn caddy_release(version_or_series: &str) -> Option<&'static VerifiedBinaryRelease> {
    CADDY_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

pub struct NginxRelease {
    pub series: &'static str,
    pub version: &'static str,
    pub archive: &'static str,
    pub source_url: &'static str,
    pub sha256: &'static str,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
}

pub const NGINX_SERIES: &str = "1.30";
pub const NGINX_VERSION: &str = "1.30.4";

pub const NGINX_RELEASES: &[NginxRelease] = &[
    NginxRelease {
        series: "1.28",
        version: "1.28.3",
        archive: "nginx-1.28.3.tar.gz",
        source_url: "https://nginx.org/download/nginx-1.28.3.tar.gz",
        sha256: "2c96a946bfb0882a21744ed429770a2123ae1828c7c48665092993ddee91a918",
        support_label: "旧版 · 维护期至 2027-05",
        legacy: true,
        recommended: false,
    },
    NginxRelease {
        series: NGINX_SERIES,
        version: NGINX_VERSION,
        archive: "nginx-1.30.4.tar.gz",
        source_url: "https://nginx.org/download/nginx-1.30.4.tar.gz",
        sha256: "4261dc90e9e47c1c4041276e9aaa3d48ebe2e664f728e14fa95ae6c67d57a08b",
        support_label: "最新稳定版",
        legacy: false,
        recommended: true,
    },
];

pub fn nginx_release(version_or_series: &str) -> Option<&'static NginxRelease> {
    NGINX_RELEASES
        .iter()
        .find(|release| release.version == version_or_series || release.series == version_or_series)
}

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
        ensure_macos("Redis")?;
        self.ensure_build_tools()?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/redis-server");
        if self.is_expected_version(&executable)
            && installation_manifest_matches(
                &installation_dir,
                "redis",
                self.release.version,
                self.release.sha256,
            )
        {
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
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);

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
        let installation_dir = self.installation_dir();
        self.is_expected_version(&installation_dir.join("bin/redis-server"))
            && installation_manifest_matches(
                &installation_dir,
                "redis",
                self.release.version,
                self.release.sha256,
            )
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
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "mysql",
                self.release.version,
                self.release.sha256,
            )
        {
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
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
        let installation_dir = self.installation_dir();
        binary_contains(
            &installation_dir.join("bin/mysqld"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation_dir,
            "mysql",
            self.release.version,
            self.release.sha256,
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
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct MailpitInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
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
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct KafkaInstaller {
    devbox_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MeilisearchInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct InfluxdbInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct FtpInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct MinioInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct RustfsInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct EtcdInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct ConsulInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct RnacosInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct RabbitmqInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

#[derive(Debug, Clone)]
pub struct ActivemqInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

impl ActivemqInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: activemq_release(ACTIVEMQ_VERSION)
                .expect("default ActiveMQ release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = activemq_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported ActiveMQ version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 ActiveMQ Classic {}", self.release.version),
        );
        ensure_macos_arm64("ActiveMQ")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("home/bin/activemq");
        if executable.is_file()
            && installation_manifest_matches(
                &installation_dir,
                "activemq",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "ActiveMQ 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads = self.devbox_root.join("downloads");
        let work = self.devbox_root.join("tmp").join(format!(
            "activemq-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads)?;
        fs::create_dir_all(&work)?;
        let _cleanup = WorkDirCleanup::new(&work);
        let archive = downloads.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        report_install_progress(55, "解压程序", "正在解压 ActiveMQ 官方分发包");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(&archive)
                .arg("-C")
                .arg(&work),
            "tar",
        )?;
        let source = work.join(format!("apache-activemq-{}", self.release.version));
        let stage = work.join("installation");
        fs::create_dir_all(&stage)?;
        fs::rename(source, stage.join("home"))?;
        let staged_executable = stage.join("home/bin/activemq");
        if !staged_executable.is_file() {
            return Err(DevBoxError::CommandFailed {
                command: "verify ActiveMQ bundle".into(),
                message: "ActiveMQ 压缩包结构不符合预期".into(),
            });
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&staged_executable, fs::Permissions::from_mode(0o755))?;
        }
        write_manifest(
            &stage,
            "activemq",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-java-distribution",
        )?;
        replace_installation(&stage, &installation_dir)?;
        report_install_progress(90, "完成安装", "ActiveMQ 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/activemq")
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        installation.join("home/bin/activemq").is_file()
            && installation_manifest_matches(
                &installation,
                "activemq",
                self.release.version,
                self.release.sha256,
            )
    }
}

pub struct NginxInstaller {
    devbox_root: PathBuf,
    release: &'static NginxRelease,
}

pub struct CaddyInstaller {
    devbox_root: PathBuf,
    release: &'static VerifiedBinaryRelease,
}

impl RabbitmqInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: rabbitmq_release(RABBITMQ_VERSION)
                .expect("default RabbitMQ release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = rabbitmq_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported RabbitMQ version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!(
                "准备安装 RabbitMQ {} 与 Erlang/OTP {RABBITMQ_OTP_VERSION}",
                self.release.version
            ),
        );
        ensure_macos_arm64("RabbitMQ")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("server/sbin/rabbitmq-server");
        if executable.is_file()
            && installation_dir.join("otp/bin/erl").is_file()
            && installation_manifest_matches(
                &installation_dir,
                "rabbitmq",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "RabbitMQ 与内置 Erlang 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "rabbitmq-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let rabbit_archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &rabbit_archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
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
        let server_source = work_dir.join(format!("rabbitmq_server-{}", self.release.version));
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
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        installation.join("server/sbin/rabbitmq-server").is_file()
            && installation.join("otp/bin/erl").is_file()
            && installation_manifest_matches(
                &installation,
                "rabbitmq",
                self.release.version,
                self.release.sha256,
            )
    }
}

impl NginxInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: nginx_release(NGINX_VERSION).expect("default Nginx release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = nginx_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported Nginx version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static NginxRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备编译安装 Nginx {}", self.release.version),
        );
        ensure_macos_arm64("Nginx")?;
        ensure_tools(&[
            "/usr/bin/curl",
            "/usr/bin/tar",
            "/usr/bin/make",
            "/usr/bin/cc",
        ])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/nginx");
        if self.is_expected_version(&executable)
            && installation_manifest_matches(
                &installation_dir,
                "nginx",
                self.release.version,
                self.release.sha256,
            )
        {
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
                .expect("Nginx installation has a parent"),
        )?;

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;

        let work_dir = temp_root.join(format!(
            "nginx-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);

        let result = self.build_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/nginx")
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation_dir = self.installation_dir();
        self.is_expected_version(&installation_dir.join("bin/nginx"))
            && installation_manifest_matches(
                &installation_dir,
                "nginx",
                self.release.version,
                self.release.sha256,
            )
    }

    fn is_expected_version(&self, executable: &Path) -> bool {
        Command::new(executable)
            .arg("-v")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stderr).contains(self.release.version))
            .unwrap_or(false)
    }

    fn build_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        report_install_progress(42, "解压源码", "正在解压 Nginx 源码");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;

        let source_dir = work_dir.join(format!("nginx-{}", self.release.version));

        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        let modules_dir = stage.join("modules");
        fs::create_dir_all(&bin_dir)?;
        fs::create_dir_all(&modules_dir)?;

        let jobs = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(2)
            .min(8);

        report_install_progress(48, "配置编译", "正在运行 ./configure");
        let prefix_path = stage.display().to_string();
        run(
            Command::new(&source_dir.join("configure"))
                .args([
                    format!("--prefix={prefix_path}"),
                    format!("--sbin-path={prefix_path}/bin/nginx"),
                    format!("--modules-path={prefix_path}/modules"),
                    "--without-http_gzip_module".into(),
                    "--without-http_rewrite_module".into(),
                ])
                .current_dir(&source_dir),
            "configure",
        )?;

        report_install_progress(55, "编译程序", format!("使用 {jobs} 个并行任务编译 Nginx"));
        run(
            Command::new("/usr/bin/make")
                .arg(format!("-j{jobs}"))
                .current_dir(&source_dir),
            "make",
        )?;

        report_install_progress(75, "安装程序", "正在安装 Nginx 到版本目录");
        run(
            Command::new("/usr/bin/make")
                .arg("install")
                .current_dir(&source_dir),
            "make install",
        )?;

        if !self.is_expected_version(&bin_dir.join("nginx")) {
            return Err(DevBoxError::CommandFailed {
                command: "nginx -v".into(),
                message: format!("编译后的程序版本不是 Nginx {}", self.release.version),
            });
        }

        write_manifest(
            &stage,
            "nginx",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-source",
        )?;
        report_install_progress(90, "完成安装", "Nginx 程序已写入版本目录");
        replace_installation(&stage, installation_dir)
    }
}

impl CaddyInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: caddy_release(CADDY_VERSION).expect("default Caddy release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = caddy_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported Caddy version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static VerifiedBinaryRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Caddy {}", self.release.version),
        );
        ensure_macos_arm64("Caddy")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/caddy");
        if executable.is_file()
            && installation_manifest_matches(
                &installation_dir,
                "caddy",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "目标版本已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "caddy-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;

        report_install_progress(55, "解压程序", "正在解压 Caddy");
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
        fs::rename(work_dir.join("caddy"), bin_dir.join("caddy"))?;

        write_manifest(
            &stage,
            "caddy",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-binary",
        )?;
        replace_installation(&stage, &installation_dir)?;
        let _ = fs::remove_dir_all(&work_dir);
        report_install_progress(90, "完成安装", "Caddy 安装完成");
        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn installation_dir(&self) -> PathBuf {
        self.devbox_root
            .join("installations/caddy")
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation_dir = self.installation_dir();
        installation_dir.join("bin/caddy").is_file()
            && installation_manifest_matches(
                &installation_dir,
                "caddy",
                self.release.version,
                self.release.sha256,
            )
    }
}

impl RnacosInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: rnacos_release(RNACOS_VERSION).expect("default rnacos release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = rnacos_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported rnacos version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 rnacos {}", self.release.version),
        );
        ensure_macos_arm64("rnacos")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/rnacos");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "rnacos",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "rnacos 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "rnacos-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
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
        if !binary_contains(
            &bin_dir.join("rnacos"),
            &["--version"],
            self.release.version,
        ) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "rnacos --version".into(),
                message: "downloaded binary is not the expected rnacos release".into(),
            });
        }
        write_manifest(
            &stage,
            "rnacos",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/rnacos"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "rnacos",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl ConsulInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: consul_release(CONSUL_VERSION).expect("default Consul release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = consul_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported Consul version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Consul {}", self.release.version),
        );
        ensure_macos_arm64("Consul")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/consul");
        if binary_contains(&executable, &["version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "consul",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "Consul 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "consul-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
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
        if !binary_contains(&bin_dir.join("consul"), &["version"], self.release.version) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "consul version".into(),
                message: "downloaded binary is not the expected Consul release".into(),
            });
        }
        write_manifest(
            &stage,
            "consul",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/consul"),
            &["version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "consul",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl EtcdInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: etcd_release(ETCD_VERSION).expect("default etcd release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = etcd_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported etcd version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static VerifiedBinaryRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 etcd {}", self.release.version),
        );
        ensure_macos_arm64("etcd")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/etcd");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "etcd",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "etcd 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "etcd-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        run(
            Command::new("/usr/bin/unzip")
                .args(["-q", "-o"])
                .arg(&archive)
                .arg("-d")
                .arg(&work_dir),
            "unzip",
        )?;

        let source = work_dir.join(format!("etcd-v{}-darwin-arm64", self.release.version));
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
        if !binary_contains(&bin_dir.join("etcd"), &["--version"], self.release.version) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "etcd --version".into(),
                message: "downloaded binary is not the expected etcd release".into(),
            });
        }
        write_manifest(
            &stage,
            "etcd",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation_dir = self.installation_dir();
        binary_contains(
            &installation_dir.join("bin/etcd"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation_dir,
            "etcd",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl RustfsInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: rustfs_release(RUSTFS_VERSION).expect("default RustFS release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = rustfs_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported RustFS version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 RustFS {}", self.release.version),
        );
        ensure_macos_arm64("RustFS")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/unzip"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/rustfs");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "rustfs",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "RustFS 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "rustfs-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
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
        if !binary_contains(
            &bin_dir.join("rustfs"),
            &["--version"],
            self.release.version,
        ) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "rustfs --version".into(),
                message: "downloaded binary is not the expected RustFS release".into(),
            });
        }
        write_manifest(
            &stage,
            "rustfs",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/rustfs"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "rustfs",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl MinioInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: minio_release(MINIO_VERSION).expect("default MinIO release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = minio_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported MinIO version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 MinIO {}", self.release.version),
        );
        ensure_macos_arm64("MinIO")?;
        ensure_tools(&["/usr/bin/curl"])?;
        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/minio");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "minio",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "MinIO 已经安装");
            return Ok(InstallOutcome::AlreadyInstalled {
                path: installation_dir,
            });
        }

        let downloads_dir = self.devbox_root.join("downloads");
        let work_dir = self.devbox_root.join("tmp").join(format!(
            "minio-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&downloads_dir)?;
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let download = downloads_dir.join(self.release.archive);
        prepare_archive(
            &download,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
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
        if !binary_contains(&bin_dir.join("minio"), &["--version"], self.release.version) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "minio --version".into(),
                message: "downloaded binary is not the expected MinIO release".into(),
            });
        }
        write_manifest(
            &stage,
            "minio",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/minio"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "minio",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl MeilisearchInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: meilisearch_release(MEILISEARCH_VERSION)
                .expect("default Meilisearch release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = meilisearch_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!(
                "unsupported Meilisearch version: {version_or_series}"
            ))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Meilisearch {}", self.release.version),
        );
        ensure_macos_arm64("Meilisearch")?;
        ensure_tools(&["/usr/bin/curl"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/meilisearch");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "meilisearch",
                self.release.version,
                self.release.sha256,
            )
        {
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

        let download = downloads_dir.join(self.release.archive);
        prepare_archive(
            &download,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "meilisearch-{}-{}-{}",
            self.release.version,
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
            self.release.version,
        ) {
            let _ = fs::remove_dir_all(&work_dir);
            return Err(DevBoxError::CommandFailed {
                command: "meilisearch --version".into(),
                message: format!(
                    "downloaded binary is not Meilisearch {}",
                    self.release.version
                ),
            });
        }
        write_manifest(
            &stage,
            "meilisearch",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/meilisearch"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "meilisearch",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl NatsInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: nats_release(NATS_VERSION).expect("default NATS release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = nats_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported NATS version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static VerifiedBinaryRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 NATS {}", self.release.version),
        );
        ensure_macos_arm64("NATS")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/nats-server");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "nats",
                self.release.version,
                self.release.sha256,
            )
        {
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

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "nats-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation_dir = self.installation_dir();
        binary_contains(
            &installation_dir.join("bin/nats-server"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation_dir,
            "nats",
            self.release.version,
            self.release.sha256,
        )
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
            .join(format!(
                "nats-server-v{}-darwin-arm64",
                self.release.version
            ))
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

        if !binary_contains(
            &bin_dir.join("nats-server"),
            &["--version"],
            self.release.version,
        ) {
            return Err(DevBoxError::CommandFailed {
                command: "nats-server --version".into(),
                message: format!("downloaded binary is not NATS {}", self.release.version),
            });
        }
        write_manifest(
            &stage,
            "nats",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
            release: mailpit_release(MAILPIT_VERSION)
                .expect("default Mailpit release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = mailpit_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported Mailpit version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static VerifiedBinaryRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 Mailpit {}", self.release.version),
        );
        ensure_macos_arm64("Mailpit")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mailpit");
        if binary_contains(&executable, &["version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "mailpit",
                self.release.version,
                self.release.sha256,
            )
        {
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

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "mailpit-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
            .join(self.release.series)
    }

    pub fn is_installed(&self) -> bool {
        let installation_dir = self.installation_dir();
        binary_contains(
            &installation_dir.join("bin/mailpit"),
            &["version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation_dir,
            "mailpit",
            self.release.version,
            self.release.sha256,
        )
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

        if !binary_contains(&bin_dir.join("mailpit"), &["version"], self.release.version) {
            return Err(DevBoxError::CommandFailed {
                command: "mailpit version".into(),
                message: format!("downloaded binary is not Mailpit {}", self.release.version),
            });
        }
        write_manifest(
            &stage,
            "mailpit",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
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
            release: mongodb_release(MONGODB_VERSION)
                .expect("default MongoDB release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = mongodb_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported MongoDB version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 MongoDB {}", self.release.version),
        );
        ensure_macos_arm64("MongoDB")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/mongod");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "mongodb",
                self.release.version,
                self.release.sha256,
            )
        {
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

        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "mongodb-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
            .join(self.release.series)
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
                            name.starts_with("mongodb-macos-")
                                && name.ends_with(self.release.version)
                        })
            })
            .ok_or_else(|| DevBoxError::CommandFailed {
                command: "tar".into(),
                message: "MongoDB archive does not contain the expected directory".into(),
            })?;
        let stage = work_dir.join("installation");
        report_install_progress(75, "整理文件", "正在写入 MongoDB 版本目录");
        fs::rename(source, &stage)?;

        if !binary_contains(
            &stage.join("bin/mongod"),
            &["--version"],
            self.release.version,
        ) {
            return Err(DevBoxError::CommandFailed {
                command: "mongod --version".into(),
                message: format!("downloaded binary is not MongoDB {}", self.release.version),
            });
        }
        write_manifest(
            &stage,
            "mongodb",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-binary",
        )?;
        report_install_progress(90, "完成安装", "MongoDB 安装完成");
        replace_installation(&stage, installation_dir)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/mongod"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "mongodb",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl InfluxdbInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: influxdb_release(INFLUXDB_VERSION)
                .expect("default InfluxDB release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = influxdb_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported InfluxDB version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 InfluxDB {}", self.release.version),
        );
        ensure_macos_arm64("InfluxDB")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/influxdb3");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "influxdb",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "InfluxDB 已经安装");
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
                .expect("InfluxDB installation has a parent"),
        )?;
        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "influxdb-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
            .join("influxdb")
            .join(self.release.series)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        report_install_progress(45, "解压程序", "正在解压 InfluxDB");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source = find_named_file(work_dir, "influxdb3", 3).ok_or_else(|| {
            DevBoxError::CommandFailed {
                command: "tar".into(),
                message: "InfluxDB archive does not contain influxdb3".into(),
            }
        })?;
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        report_install_progress(75, "整理文件", "正在写入 InfluxDB 版本目录");
        fs::create_dir_all(&bin_dir)?;
        fs::copy(source, bin_dir.join("influxdb3"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("influxdb3"), fs::Permissions::from_mode(0o755))?;
        }
        if !binary_contains(
            &bin_dir.join("influxdb3"),
            &["--version"],
            self.release.version,
        ) {
            return Err(DevBoxError::CommandFailed {
                command: "influxdb3 --version".into(),
                message: format!("downloaded binary is not InfluxDB {}", self.release.version),
            });
        }
        write_manifest(
            &stage,
            "influxdb",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "official-binary",
        )?;
        report_install_progress(90, "完成安装", "InfluxDB 安装完成");
        replace_installation(&stage, installation_dir)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/influxdb3"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "influxdb",
            self.release.version,
            self.release.sha256,
        )
    }
}

impl FtpInstaller {
    pub fn new(devbox_root: impl Into<PathBuf>) -> Self {
        Self {
            devbox_root: devbox_root.into(),
            release: ftp_release(FTP_VERSION).expect("default FTP release is registered"),
        }
    }

    pub fn for_version(devbox_root: impl Into<PathBuf>, version_or_series: &str) -> Result<Self> {
        let release = ftp_release(version_or_series).ok_or_else(|| {
            DevBoxError::InvalidConfig(format!("unsupported FTP version: {version_or_series}"))
        })?;
        Ok(Self {
            devbox_root: devbox_root.into(),
            release,
        })
    }

    pub fn release(&self) -> &'static VerifiedBinaryRelease {
        self.release
    }

    pub fn install(&self) -> Result<InstallOutcome> {
        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 FTP Server {}", self.release.version),
        );
        ensure_macos_arm64("FTP Server")?;
        ensure_tools(&["/usr/bin/curl", "/usr/bin/tar"])?;

        let installation_dir = self.installation_dir();
        let executable = installation_dir.join("bin/sftpgo");
        if binary_contains(&executable, &["--version"], self.release.version)
            && installation_manifest_matches(
                &installation_dir,
                "ftp",
                self.release.version,
                self.release.sha256,
            )
        {
            report_install_progress(90, "已安装", "FTP Server 已经安装");
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
                .expect("FTP installation has a parent"),
        )?;
        let archive = downloads_dir.join(self.release.archive);
        prepare_archive(
            &archive,
            self.release.archive,
            self.release.source_url,
            self.release.sha256,
        )?;
        let work_dir = temp_root.join(format!(
            "ftp-{}-{}-{}",
            self.release.version,
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&work_dir)?;
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
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
            .join("ftp")
            .join(self.release.series)
    }

    fn extract_and_commit(
        &self,
        archive: &Path,
        work_dir: &Path,
        installation_dir: &Path,
    ) -> Result<()> {
        report_install_progress(45, "解压程序", "正在解压 FTP Server");
        run(
            Command::new("/usr/bin/tar")
                .args(["-xJf"])
                .arg(archive)
                .arg("-C")
                .arg(work_dir),
            "tar",
        )?;
        let source =
            find_named_file(work_dir, "sftpgo", 2).ok_or_else(|| DevBoxError::CommandFailed {
                command: "tar".into(),
                message: "FTP Server archive does not contain sftpgo".into(),
            })?;
        let stage = work_dir.join("installation");
        let bin_dir = stage.join("bin");
        fs::create_dir_all(&bin_dir)?;
        report_install_progress(75, "整理文件", "正在写入 FTP Server 版本目录");
        fs::copy(source, bin_dir.join("sftpgo"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin_dir.join("sftpgo"), fs::Permissions::from_mode(0o755))?;
        }
        if !binary_contains(
            &bin_dir.join("sftpgo"),
            &["--version"],
            self.release.version,
        ) {
            return Err(DevBoxError::CommandFailed {
                command: "sftpgo --version".into(),
                message: format!(
                    "downloaded binary is not FTP Server {}",
                    self.release.version
                ),
            });
        }
        write_manifest(
            &stage,
            "ftp",
            self.release.series,
            self.release.version,
            self.release.source_url,
            self.release.sha256,
            "sftpgo-official-binary",
        )?;
        report_install_progress(90, "完成安装", "FTP Server 安装完成");
        replace_installation(&stage, installation_dir)
    }

    pub fn is_installed(&self) -> bool {
        let installation = self.installation_dir();
        binary_contains(
            &installation.join("bin/sftpgo"),
            &["--version"],
            self.release.version,
        ) && installation_manifest_matches(
            &installation,
            "ftp",
            self.release.version,
            self.release.sha256,
        )
    }
}

fn find_named_file(root: &Path, name: &str, remaining_depth: usize) -> Option<PathBuf> {
    for entry in fs::read_dir(root).ok()?.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|value| value.to_str()) == Some(name) {
            return Some(path);
        }
        if remaining_depth > 0 && path.is_dir() {
            if let Some(found) = find_named_file(&path, name, remaining_depth - 1) {
                return Some(found);
            }
        }
    }
    None
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
        ensure_macos("PostgreSQL")?;
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
            && installation_manifest_matches(
                &installation_dir,
                "postgres",
                self.release.version,
                self.release.sha256,
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
        let _work_dir_cleanup = WorkDirCleanup::new(&work_dir);
        let result = self.build_and_commit(&archive, &work_dir, &installation_dir);
        let _ = fs::remove_dir_all(&work_dir);
        result?;

        Ok(InstallOutcome::Installed {
            path: installation_dir,
        })
    }

    pub fn initialize(&self, data_dir: &Path) -> Result<()> {
        fs::create_dir_all(data_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(data_dir, fs::Permissions::from_mode(0o700))?;
        }
        if data_dir.join("PG_VERSION").is_file() {
            report_install_log("初始化数据", "PostgreSQL 数据目录已经初始化");
            return Ok(());
        }
        report_install_progress(94, "初始化数据", "正在执行 initdb 创建数据库集群");
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
        ) && installation_manifest_matches(
            &installation_dir,
            "postgres",
            self.release.version,
            self.release.sha256,
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
    check_install_cancelled()?;
    report_install_progress(8, "检查缓存", format!("检查安装包缓存：{archive_name}"));
    if archive.is_file() {
        check_install_cancelled()?;
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
    let _download_permit = DownloadPermit::acquire(settings.download_concurrency)?;
    let mut failures = Vec::new();

    for (index, candidate) in candidates.iter().enumerate() {
        check_install_cancelled()?;
        let resuming = partial.is_file();
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
        if resuming {
            report_install_log(
                "断点续传",
                format!("检测到未完成下载，将从已有文件继续：{archive_name}"),
            );
        }
        if let Err(error) = download_candidate(&partial, candidate, &settings, resuming) {
            if matches!(error, DevBoxError::InstallCancelled) {
                report_install_log("安装取消", "下载已停止，未完成文件将用于下次断点续传");
                return Err(error);
            }
            failures.push(format!("{}: {}", candidate.label, error));
            report_install_log(
                "切换下载源",
                format!("{}不可用，自动尝试下一个下载源", candidate.label),
            );
            continue;
        }

        check_install_cancelled()?;
        report_install_progress(30, "校验安装包", "下载完成，正在计算 SHA-256");
        let actual = sha256(&partial)?;
        if actual != expected_sha256 {
            failures.push(format!("{}: SHA-256 校验不一致", candidate.label));
            let _ = fs::remove_file(&partial);
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
    Err(DevBoxError::CommandFailed {
        command: "curl".into(),
        message: format!("所有下载源均失败：{}", failures.join("；")),
    })
}

fn download_candidate(
    partial: &Path,
    candidate: &DownloadCandidate,
    settings: &InstallerDownloadSettings,
    resume: bool,
) -> Result<()> {
    let mut command = download_command(partial, candidate, settings, resume);
    match run(&mut command, "curl") {
        Ok(()) => Ok(()),
        Err(DevBoxError::InstallCancelled) => Err(DevBoxError::InstallCancelled),
        Err(_error) if resume => {
            report_install_log(
                "断点续传",
                format!("{}不支持当前断点，正在从头重新下载", candidate.label),
            );
            fs::remove_file(partial)?;
            run(
                &mut download_command(partial, candidate, settings, false),
                "curl",
            )
        }
        Err(error) => Err(error),
    }
}

fn download_command(
    partial: &Path,
    candidate: &DownloadCandidate,
    settings: &InstallerDownloadSettings,
    resume: bool,
) -> Command {
    let mut command = Command::new("/usr/bin/curl");
    command
        .args(["--fail", "--location", "--silent", "--show-error"])
        .arg("--connect-timeout")
        .arg(settings.download_timeout_seconds.min(15).to_string())
        .arg("--max-time")
        .arg(settings.download_timeout_seconds.to_string())
        .args(["--retry", "1"]);
    if resume {
        command.args(["--continue-at", "-"]);
    }
    if !candidate.official {
        command.args(["--speed-time", "15", "--speed-limit", "16384"]);
    }
    if !settings.download_proxy_enabled || settings.proxy_mode == "disabled" {
        command.args(["--noproxy", "*"]);
    } else if settings.proxy_mode == "manual" {
        if let Some(proxy_url) = settings.proxy_url.as_deref() {
            command
                .args(["--proxy", proxy_url])
                .args(["--noproxy", "localhost,127.0.0.1,::1,0.0.0.0,*.local"]);
        }
    } else if !has_proxy_environment() {
        #[cfg(target_os = "macos")]
        if let Some(proxy_url) = macos_system_proxy_url() {
            command
                .args(["--proxy", &proxy_url])
                .args(["--noproxy", "localhost,127.0.0.1,::1,0.0.0.0,*.local"]);
        }
        #[cfg(target_os = "windows")]
        if let Some(proxy_url) = windows_system_proxy_url() {
            command
                .args(["--proxy", &proxy_url])
                .args(["--noproxy", "localhost,127.0.0.1,::1,0.0.0.0,*.local"]);
        }
    }
    command.arg("--output").arg(partial).arg(&candidate.url);
    command
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
    proxy_mode: String,
    proxy_url: Option<String>,
    download_proxy_enabled: bool,
}

impl Default for InstallerDownloadSettings {
    fn default() -> Self {
        Self {
            download_mirror: None,
            public_github_mirror: true,
            download_concurrency: 2,
            download_timeout_seconds: 180,
            proxy_mode: "system".into(),
            proxy_url: None,
            download_proxy_enabled: true,
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
    if !matches!(
        settings.proxy_mode.as_str(),
        "system" | "manual" | "disabled"
    ) {
        settings.proxy_mode = "system".into();
    }
    settings
}

fn has_proxy_environment() -> bool {
    [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ]
    .iter()
    .any(|key| std::env::var(key).is_ok_and(|value| !value.trim().is_empty()))
}

#[cfg(target_os = "macos")]
fn macos_system_proxy_url() -> Option<String> {
    let output = Command::new("/usr/sbin/scutil")
        .arg("--proxy")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| parse_macos_system_proxy_url(&String::from_utf8_lossy(&output.stdout)))
        .flatten()
}

#[cfg(target_os = "macos")]
fn parse_macos_system_proxy_url(output: &str) -> Option<String> {
    fn value<'a>(output: &'a str, key: &str) -> Option<&'a str> {
        output.lines().find_map(|line| {
            let (name, value) = line.trim().split_once(':')?;
            (name.trim() == key).then_some(value.trim())
        })
    }
    for (prefix, scheme) in [("HTTPS", "http"), ("HTTP", "http"), ("SOCKS", "socks5h")] {
        if value(output, &format!("{prefix}Enable")) != Some("1") {
            continue;
        }
        let host = value(output, &format!("{prefix}Proxy"))?;
        let port = value(output, &format!("{prefix}Port"))?;
        if !host.is_empty() && port.parse::<u16>().is_ok() {
            return Some(format!(
                "{scheme}://{}:{port}",
                host.trim_matches(['[', ']'])
            ));
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn windows_system_proxy_url() -> Option<String> {
    fn registry_value(name: &str) -> Option<String> {
        let output = Command::new("reg.exe")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                "/v",
                name,
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.contains(name))
            .and_then(|line| line.split_whitespace().last())
            .map(str::to_string)
    }
    if registry_value("ProxyEnable").as_deref() != Some("0x1") {
        return None;
    }
    let server = registry_value("ProxyServer")?;
    if !server.contains('=') {
        return Some(format!("http://{server}"));
    }
    for preferred in ["https", "http", "socks", "socks5"] {
        for entry in server.split(';') {
            let Some((scheme, endpoint)) = entry.split_once('=') else {
                continue;
            };
            if scheme.trim().eq_ignore_ascii_case(preferred) && !endpoint.trim().is_empty() {
                let proxy_scheme = if preferred.starts_with("socks") {
                    "socks5h"
                } else {
                    "http"
                };
                return Some(format!("{proxy_scheme}://{}", endpoint.trim()));
            }
        }
    }
    None
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
    fn acquire(limit: usize) -> Result<Self> {
        let (lock, ready) = &ACTIVE_DOWNLOADS;
        let mut active = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while *active >= limit {
            check_install_cancelled()?;
            let (next, _) = ready
                .wait_timeout(active, Duration::from_millis(100))
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            active = next;
        }
        *active += 1;
        Ok(Self)
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

fn installation_manifest_matches(
    installation_dir: &Path,
    service: &str,
    version: &str,
    source_sha256: &str,
) -> bool {
    let value: serde_json::Value = match fs::read(installation_dir.join("manifest.json"))
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
    {
        Some(value) => value,
        None => return false,
    };
    value.get("service").and_then(|value| value.as_str()) == Some(service)
        && value.get("version").and_then(|value| value.as_str()) == Some(version)
        && value.get("source_sha256").and_then(|value| value.as_str()) == Some(source_sha256)
}

fn replace_installation(stage: &Path, installation_dir: &Path) -> Result<()> {
    check_install_cancelled()?;
    if let Some(parent) = installation_dir.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup = installation_dir.with_file_name(format!(
        ".{}-previous-{}-{}",
        installation_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("installation"),
        std::process::id(),
        unique_suffix()
    ));
    if installation_dir.exists() {
        fs::rename(installation_dir, &backup)?;
    }
    if let Err(error) = fs::rename(stage, installation_dir) {
        if backup.exists() {
            let _ = fs::rename(&backup, installation_dir);
        }
        return Err(error.into());
    }
    if backup.exists() {
        if let Err(error) = fs::remove_dir_all(&backup) {
            report_install_log(
                "清理旧版本",
                format!("新版本已安装，但旧目录清理失败：{error}"),
            );
        }
    }
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

fn ensure_macos(service: &str) -> Result<()> {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(DevBoxError::UnsupportedPlatform(format!(
            "{service} automatic installation currently supports macOS"
        )))
    }
}

fn sha256(path: &Path) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_install_cancelled()?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn run(command: &mut Command, name: &str) -> Result<()> {
    check_install_cancelled()?;
    report_install_log("执行命令", format!("开始执行：{name}"));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().map(|mut output| {
        thread::spawn(move || {
            let mut contents = Vec::new();
            let _ = output.read_to_end(&mut contents);
            contents
        })
    });
    let stderr = child.stderr.take().map(|mut output| {
        thread::spawn(move || {
            let mut contents = Vec::new();
            let _ = output.read_to_end(&mut contents);
            contents
        })
    });

    let status = loop {
        if check_install_cancelled().is_err() {
            terminate_install_process(&mut child);
            let _ = child.wait();
            join_output(stdout);
            join_output(stderr);
            report_install_log("安装取消", format!("已终止命令：{name}"));
            return Err(DevBoxError::InstallCancelled);
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(Duration::from_millis(100));
    };
    let stdout = join_output(stdout);
    let stderr = join_output(stderr);
    if status.success() {
        report_install_log("执行命令", format!("执行完成：{name}"));
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&stdout).trim().to_string();
    let message = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => format!("process exited with {status}"),
    };
    let message = tail_chars(&message, 32 * 1024);
    report_install_log("命令失败", format!("{name}：{message}"));
    Err(DevBoxError::CommandFailed {
        command: name.into(),
        message,
    })
}

#[cfg(unix)]
fn terminate_install_process(child: &mut std::process::Child) {
    let process_group = -(child.id() as i32);
    unsafe {
        libc::kill(process_group, libc::SIGTERM);
    }
    thread::sleep(Duration::from_millis(100));
    unsafe {
        libc::kill(process_group, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn terminate_install_process(child: &mut std::process::Child) {
    let _ = child.kill();
}

fn join_output(handle: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

struct WorkDirCleanup(PathBuf);

impl WorkDirCleanup {
    fn new(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

impl Drop for WorkDirCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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
    use std::time::{Duration, Instant};
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
    fn valid_archive_cache_is_reused_without_downloading() {
        let temp = TempDir::new().unwrap();
        let downloads = temp.path().join("downloads");
        fs::create_dir_all(&downloads).unwrap();
        let archive = downloads.join("example.tar.gz");
        fs::write(&archive, b"abc").unwrap();
        let updates = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&updates);

        let result = with_install_reporter(
            InstallReporter::new(move |update| captured.lock().unwrap().push(update)),
            || {
                prepare_archive(
                    &archive,
                    "example.tar.gz",
                    "https://invalid.example/example.tar.gz",
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                )
            },
        );

        assert!(result.is_ok());
        assert!(updates
            .lock()
            .unwrap()
            .iter()
            .any(|update| update.stage == "使用缓存"));
    }

    #[test]
    fn resumed_download_uses_curl_continue_at() {
        let candidate = DownloadCandidate {
            label: "测试源".into(),
            url: "https://example.com/archive.tar.gz".into(),
            official: true,
        };
        let settings = InstallerDownloadSettings::default();
        let command = download_command(
            Path::new("/tmp/archive.tar.gz.partial"),
            &candidate,
            &settings,
            true,
        );
        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(arguments
            .windows(2)
            .any(|pair| pair == ["--continue-at", "-"]));
    }

    #[test]
    fn download_proxy_policy_is_forwarded_to_curl() {
        let candidate = DownloadCandidate {
            label: "测试源".into(),
            url: "https://example.com/archive.tar.gz".into(),
            official: true,
        };
        let manual = InstallerDownloadSettings {
            proxy_mode: "manual".into(),
            proxy_url: Some("http://127.0.0.1:7890".into()),
            ..InstallerDownloadSettings::default()
        };
        let manual_command = download_command(
            Path::new("/tmp/archive.tar.gz.partial"),
            &candidate,
            &manual,
            false,
        );
        let manual_arguments = manual_command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(manual_arguments
            .windows(2)
            .any(|pair| pair == ["--proxy", "http://127.0.0.1:7890"]));
        assert!(manual_arguments
            .windows(2)
            .any(|pair| pair[0] == "--noproxy" && pair[1].contains("localhost")));

        let direct = InstallerDownloadSettings {
            download_proxy_enabled: false,
            ..InstallerDownloadSettings::default()
        };
        let direct_command = download_command(
            Path::new("/tmp/archive.tar.gz.partial"),
            &candidate,
            &direct,
            false,
        );
        let direct_arguments = direct_command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(direct_arguments
            .windows(2)
            .any(|pair| pair == ["--noproxy", "*"]));
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_interrupts_a_running_command() {
        let token = InstallCancellationToken::default();
        let cancellation = token.clone();
        let canceller = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            cancellation.cancel();
        });
        let started = Instant::now();

        let result = with_install_context(InstallReporter::default(), token, || {
            run(Command::new("/bin/sleep").arg("10"), "sleep")
        });
        canceller.join().unwrap();

        assert!(matches!(result, Err(DevBoxError::InstallCancelled)));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn cancellation_never_replaces_an_existing_installation() {
        let temp = TempDir::new().unwrap();
        let installation = temp.path().join("installations/service/1.0");
        let stage = temp.path().join("tmp/installation");
        fs::create_dir_all(&installation).unwrap();
        fs::create_dir_all(&stage).unwrap();
        fs::write(installation.join("marker"), b"stable").unwrap();
        fs::write(stage.join("marker"), b"new").unwrap();
        let token = InstallCancellationToken::default();
        token.cancel();

        let result = with_install_context(InstallReporter::default(), token, || {
            replace_installation(&stage, &installation)
        });

        assert!(matches!(result, Err(DevBoxError::InstallCancelled)));
        assert_eq!(fs::read(installation.join("marker")).unwrap(), b"stable");
        assert_eq!(fs::read(stage.join("marker")).unwrap(), b"new");

        let retry = with_install_context(
            InstallReporter::default(),
            InstallCancellationToken::default(),
            || replace_installation(&stage, &installation),
        );
        assert!(retry.is_ok());
        assert_eq!(fs::read(installation.join("marker")).unwrap(), b"new");
    }

    #[test]
    fn work_directory_is_removed_when_an_install_step_returns_early() {
        let temp = TempDir::new().unwrap();
        let work_dir = temp.path().join("tmp/service-work");
        fs::create_dir_all(&work_dir).unwrap();
        {
            let _cleanup = WorkDirCleanup::new(&work_dir);
            fs::write(work_dir.join("partial-output"), b"incomplete").unwrap();
        }
        assert!(!work_dir.exists());
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
    fn installation_manifest_must_match_service_version_and_checksum() {
        let temp = TempDir::new().unwrap();
        let installation = temp.path().join("installation");
        fs::create_dir_all(&installation).unwrap();
        write_manifest(
            &installation,
            "redis",
            "7.2",
            "7.2.15",
            "https://example.com/redis.tar.gz",
            "abc123",
            "official-source",
        )
        .unwrap();

        assert!(installation_manifest_matches(
            &installation,
            "redis",
            "7.2.15",
            "abc123"
        ));
        assert!(!installation_manifest_matches(
            &installation,
            "redis",
            "7.2.14",
            "abc123"
        ));
        assert!(!installation_manifest_matches(
            &installation,
            "redis",
            "7.2.15",
            "different"
        ));
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

    #[test]
    fn verified_binary_catalogs_have_valid_unique_releases() {
        for releases in [
            MAILPIT_RELEASES,
            NATS_RELEASES,
            ETCD_RELEASES,
            CADDY_RELEASES,
            MONGODB_RELEASES,
            MEILISEARCH_RELEASES,
            INFLUXDB_RELEASES,
            MINIO_RELEASES,
            RUSTFS_RELEASES,
            CONSUL_RELEASES,
            RNACOS_RELEASES,
            RABBITMQ_RELEASES,
            FTP_RELEASES,
        ] {
            assert!(releases.len() >= 2);
            assert_eq!(
                releases
                    .iter()
                    .filter(|release| release.recommended)
                    .count(),
                1
            );
            for (index, release) in releases.iter().enumerate() {
                assert_eq!(release.sha256.len(), 64);
                assert!(release.source_url.starts_with("https://"));
                assert!(release.source_url.contains(release.version));
                assert!(releases[..index]
                    .iter()
                    .all(|known| known.series != release.series));
            }
        }
        assert!(mailpit_release("0.0").is_none());
        assert!(nats_release("1.0").is_none());
        assert!(etcd_release("2.0").is_none());
        assert!(caddy_release("1.0").is_none());
        assert!(mongodb_release("6.0").is_none());
        assert!(meilisearch_release("1.0").is_none());
        assert!(influxdb_release("2.0").is_none());
        assert!(minio_release("2023").is_none());
        assert!(rustfs_release("0.9").is_none());
        assert!(consul_release("1.20").is_none());
        assert!(rnacos_release("0.7").is_none());
        assert!(rabbitmq_release("3.13").is_none());
        assert!(activemq_release("5.18").is_none());
        assert!(ftp_release("2.6").is_none());
    }

    #[test]
    fn verified_binary_installers_use_independent_series_directories() {
        let root = Path::new("/tmp/zhiyu-verified-binary-versions");

        assert!(MailpitInstaller::for_version(root, "1.28.4")
            .unwrap()
            .installation_dir()
            .ends_with("mailpit/1.28"));
        assert!(MailpitInstaller::new(root)
            .installation_dir()
            .ends_with(format!("mailpit/{MAILPIT_SERIES}")));
        assert!(NatsInstaller::for_version(root, "2.11.17")
            .unwrap()
            .installation_dir()
            .ends_with("nats/2.11"));
        assert!(NatsInstaller::new(root)
            .installation_dir()
            .ends_with(format!("nats/{NATS_SERIES}")));
        assert!(EtcdInstaller::for_version(root, "3.5.21")
            .unwrap()
            .installation_dir()
            .ends_with("etcd/3.5"));
        assert!(EtcdInstaller::new(root)
            .installation_dir()
            .ends_with(format!("etcd/{ETCD_SERIES}")));
        assert!(CaddyInstaller::for_version(root, "2.10.2")
            .unwrap()
            .installation_dir()
            .ends_with("caddy/2.10"));
        assert!(CaddyInstaller::new(root)
            .installation_dir()
            .ends_with(format!("caddy/{CADDY_SERIES}")));
        assert!(FtpInstaller::for_version(root, "2.7.4")
            .unwrap()
            .installation_dir()
            .ends_with("ftp/2.7.4"));
        assert!(FtpInstaller::new(root)
            .installation_dir()
            .ends_with(format!("ftp/{FTP_SERIES}")));
        assert!(MongodbInstaller::for_version(root, "7.0")
            .unwrap()
            .installation_dir()
            .ends_with("mongodb/7.0"));
        assert!(InfluxdbInstaller::for_version(root, "3.8")
            .unwrap()
            .installation_dir()
            .ends_with("influxdb/3.8"));
        assert!(InfluxdbInstaller::new(root)
            .installation_dir()
            .ends_with(format!("influxdb/{INFLUXDB_SERIES}")));
        assert!(MeilisearchInstaller::for_version(root, "1.45")
            .unwrap()
            .installation_dir()
            .ends_with("meilisearch/1.45"));
        assert!(MinioInstaller::for_version(root, "2024")
            .unwrap()
            .installation_dir()
            .ends_with("minio/2024"));
        assert!(RustfsInstaller::for_version(root, "1.0.0-beta.1")
            .unwrap()
            .installation_dir()
            .ends_with("rustfs/1.0-beta.1"));
        assert!(ConsulInstaller::for_version(root, "1.21")
            .unwrap()
            .installation_dir()
            .ends_with("consul/1.21"));
        assert!(RnacosInstaller::for_version(root, "0.8.4")
            .unwrap()
            .installation_dir()
            .ends_with("rnacos/0.8.4"));
        assert!(RabbitmqInstaller::for_version(root, "4.2")
            .unwrap()
            .installation_dir()
            .ends_with("rabbitmq/4.2"));
        assert!(ActivemqInstaller::for_version(root, "6.3")
            .unwrap()
            .installation_dir()
            .ends_with("activemq/6.3"));
        assert!(ActivemqInstaller::new(root)
            .installation_dir()
            .ends_with(format!("activemq/{ACTIVEMQ_SERIES}")));
    }
}
