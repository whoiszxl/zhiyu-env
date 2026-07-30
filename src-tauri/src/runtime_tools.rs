use devbox_core::{check_install_cancelled, report_install_progress};
use reqwest::header::RANGE;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKindInput {
    Go,
    Java,
    Rust,
    Python,
    Node,
}

impl RuntimeKindInput {
    fn id(self) -> &'static str {
        match self {
            Self::Go => "go",
            Self::Java => "java",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Node => "node",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Go => "Go",
            Self::Java => "Java",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Node => "Node.js",
        }
    }
}

#[derive(Clone, Copy)]
struct RuntimeRelease {
    kind: RuntimeKindInput,
    series: &'static str,
    version: &'static str,
    archive: &'static str,
    url: &'static str,
    sha256: &'static str,
    support_label: &'static str,
    recommended: bool,
}

const RELEASES: &[RuntimeRelease] = &[
    RuntimeRelease {
        kind: RuntimeKindInput::Go,
        series: "1.22",
        version: "1.22.12",
        archive: "go1.22.12.darwin-arm64.tar.gz",
        url: "https://go.dev/dl/go1.22.12.darwin-arm64.tar.gz",
        sha256: "416c35218edb9d20990b5d8fc87be655d8b39926f15524ea35c66ee70273050d",
        support_label: "已停止维护 · 仅兼容旧项目",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Go,
        series: "1.23",
        version: "1.23.12",
        archive: "go1.23.12.darwin-arm64.tar.gz",
        url: "https://go.dev/dl/go1.23.12.darwin-arm64.tar.gz",
        sha256: "5bfa117e401ae64e7ffb960243c448b535fe007e682a13ff6c7371f4a6f0ccaa",
        support_label: "已停止维护 · 仅兼容旧项目",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Go,
        series: "1.24",
        version: "1.24.13",
        archive: "go1.24.13.darwin-arm64.tar.gz",
        url: "https://go.dev/dl/go1.24.13.darwin-arm64.tar.gz",
        sha256: "f282d882c3353485e2fc6c634606d85caf36e855167d59b996dbeae19fa7629a",
        support_label: "已停止维护 · 仅兼容旧项目",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Go,
        series: "1.25",
        version: "1.25.12",
        archive: "go1.25.12.darwin-arm64.tar.gz",
        url: "https://go.dev/dl/go1.25.12.darwin-arm64.tar.gz",
        sha256: "fa2c88bbcf64bd3b2aef355f026cfec6d3a4a01c132f999c8f8c964eb767164f",
        support_label: "稳定版本",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Go,
        series: "1.26",
        version: "1.26.5",
        archive: "go1.26.5.darwin-arm64.tar.gz",
        url: "https://go.dev/dl/go1.26.5.darwin-arm64.tar.gz",
        sha256: "efb87ff28af9a188d0536ef5d42e63dd52ba8263cd7344a993cc48dd11dedb6a",
        support_label: "当前稳定版本",
        recommended: true,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Java,
        series: "8",
        version: "8.0.502+7",
        archive: "zulu8.96.0.19-ca-jdk8.0.502-macosx_aarch64.tar.gz",
        url: "https://cdn.azul.com/zulu/bin/zulu8.96.0.19-ca-jdk8.0.502-macosx_aarch64.tar.gz",
        sha256: "9f9e5038c638e415e507e8b5118a774822f553a56e76bdf4b042c3fbe7b69083",
        support_label: "LTS · 旧项目兼容 · Azul Zulu",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Java,
        series: "11",
        version: "11.0.31+11",
        archive: "OpenJDK11U-jdk_aarch64_mac_hotspot_11.0.31_11.tar.gz",
        url: "https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.31%2B11/OpenJDK11U-jdk_aarch64_mac_hotspot_11.0.31_11.tar.gz",
        sha256: "e3377bbe07f4396ba03adcfc5f3d71d151d6a7b858abdf1d0dd20ac4d8d709b0",
        support_label: "LTS · 旧项目兼容",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Java,
        series: "17",
        version: "17.0.19+10",
        archive: "OpenJDK17U-jdk_aarch64_mac_hotspot_17.0.19_10.tar.gz",
        url: "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.19%2B10/OpenJDK17U-jdk_aarch64_mac_hotspot_17.0.19_10.tar.gz",
        sha256: "8fa1eff40bb637a33613b2ccb8b12c70dc3661cc22cf8e784943715769a05336",
        support_label: "LTS · 长期支持",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Java,
        series: "21",
        version: "21.0.11+10",
        archive: "OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.11_10.tar.gz",
        url: "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11%2B10/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.11_10.tar.gz",
        sha256: "6ebcf221c9b41507b14c098e93c6ead6440b8d9bd154f8ec666c4c73abbdb201",
        support_label: "LTS · 推荐",
        recommended: true,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Java,
        series: "24",
        version: "24.0.2+12",
        archive: "OpenJDK24U-jdk_aarch64_mac_hotspot_24.0.2_12.tar.gz",
        url: "https://github.com/adoptium/temurin24-binaries/releases/download/jdk-24.0.2%2B12/OpenJDK24U-jdk_aarch64_mac_hotspot_24.0.2_12.tar.gz",
        sha256: "db2ba6f72c19ad8b742303a504f58474bceeb94174a185de5f095c1d45577f1c",
        support_label: "已停止维护 · Java 24 兼容",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Java,
        series: "25",
        version: "25.0.4+7",
        archive: "OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.4_7.tar.gz",
        url: "https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.4%2B7/OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.4_7.tar.gz",
        sha256: "5a101c54abf5a9f16c0f70d8c38ba99e6567c1ba213378f0bb04497284f051bd",
        support_label: "LTS · 最新长期支持",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Rust,
        series: "1.85",
        version: "1.85.1",
        archive: "rust-1.85.1-aarch64-apple-darwin.tar.gz",
        url: "https://static.rust-lang.org/dist/2025-03-18/rust-1.85.1-aarch64-apple-darwin.tar.gz",
        sha256: "64b0341a47e684d648c9b7defd0b7ff9d5397a64718cf803c1e114544f94bbe9",
        support_label: "Rust 2024 基线版本",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Rust,
        series: "1.88",
        version: "1.88.0",
        archive: "rust-1.88.0-aarch64-apple-darwin.tar.gz",
        url: "https://static.rust-lang.org/dist/2025-06-26/rust-1.88.0-aarch64-apple-darwin.tar.gz",
        sha256: "dee921b9a41b1c3fbb088ad31dcca3b232de2cb89c268db75f40912eeaa474db",
        support_label: "稳定兼容版本",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Rust,
        series: "1.92",
        version: "1.92.0",
        archive: "rust-1.92.0-aarch64-apple-darwin.tar.gz",
        url: "https://static.rust-lang.org/dist/2025-12-11/rust-1.92.0-aarch64-apple-darwin.tar.gz",
        sha256: "235a6cca2dd4881130a9ae61ad1149bbf28bba184dd4621700f0c98c97457716",
        support_label: "兼容版本",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Rust,
        series: "1.96",
        version: "1.96.1",
        archive: "rust-1.96.1-aarch64-apple-darwin.tar.gz",
        url: "https://static.rust-lang.org/dist/2026-06-30/rust-1.96.1-aarch64-apple-darwin.tar.gz",
        sha256: "c080e506af9cba3ca9472c17d989c2d8d5bcfc818eb5e196c77beee982788b50",
        support_label: "稳定版本",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Rust,
        series: "1.97",
        version: "1.97.1",
        archive: "rust-1.97.1-aarch64-apple-darwin.tar.gz",
        url: "https://static.rust-lang.org/dist/2026-07-16/rust-1.97.1-aarch64-apple-darwin.tar.gz",
        sha256: "cbd14c36f039f6f11f38148a6295d8234d18ddf20bea53031c86f119423a8b26",
        support_label: "当前稳定版本",
        recommended: true,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Python,
        series: "3.10",
        version: "3.10.20",
        archive: "cpython-3.10.20+20260718-aarch64-apple-darwin-install_only.tar.gz",
        url: "https://github.com/astral-sh/python-build-standalone/releases/download/20260718/cpython-3.10.20%2B20260718-aarch64-apple-darwin-install_only.tar.gz",
        sha256: "5ce056c4294bc7155cdd98ea35ee764bffebb08854c4910eb433cc5f4e45e9a5",
        support_label: "安全维护阶段 · Astral",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Python,
        series: "3.11",
        version: "3.11.15",
        archive: "cpython-3.11.15+20260718-aarch64-apple-darwin-install_only.tar.gz",
        url: "https://github.com/astral-sh/python-build-standalone/releases/download/20260718/cpython-3.11.15%2B20260718-aarch64-apple-darwin-install_only.tar.gz",
        sha256: "125587d03495bebdf30ec9e549a8469c97c0925d863ff401f24f157fd44d91d6",
        support_label: "稳定兼容版本 · Astral",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Python,
        series: "3.12",
        version: "3.12.13",
        archive: "cpython-3.12.13+20260718-aarch64-apple-darwin-install_only.tar.gz",
        url: "https://github.com/astral-sh/python-build-standalone/releases/download/20260718/cpython-3.12.13%2B20260718-aarch64-apple-darwin-install_only.tar.gz",
        sha256: "62aeee6161d57303a71a138b75fd5cc6fb8c89c4b1d9c7f0a052d89fa0b6652b",
        support_label: "稳定兼容版本 · Astral",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Python,
        series: "3.13",
        version: "3.13.14",
        archive: "cpython-3.13.14+20260718-aarch64-apple-darwin-install_only.tar.gz",
        url: "https://github.com/astral-sh/python-build-standalone/releases/download/20260718/cpython-3.13.14%2B20260718-aarch64-apple-darwin-install_only.tar.gz",
        sha256: "dca7c3bac21f023cf294705b27f4f3e9c70399c40790ebb81e8d0eff15b00770",
        support_label: "成熟稳定版本 · Astral",
        recommended: true,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Python,
        series: "3.14",
        version: "3.14.6",
        archive: "cpython-3.14.6+20260718-aarch64-apple-darwin-install_only.tar.gz",
        url: "https://github.com/astral-sh/python-build-standalone/releases/download/20260718/cpython-3.14.6%2B20260718-aarch64-apple-darwin-install_only.tar.gz",
        sha256: "5a234e405386bf486bab196018c01bc4577a4f0cc9fd5bc50f7a979fe4f5c59d",
        support_label: "当前稳定版本 · Astral",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Node,
        series: "18",
        version: "18.20.8",
        archive: "node-v18.20.8-darwin-arm64.tar.gz",
        url: "https://nodejs.org/dist/v18.20.8/node-v18.20.8-darwin-arm64.tar.gz",
        sha256: "bae4965d29d29bd32f96364eefbe3bca576a03e917ddbb70b9330d75f2cacd76",
        support_label: "已停止维护 · 仅兼容旧项目",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Node,
        series: "20",
        version: "20.20.2",
        archive: "node-v20.20.2-darwin-arm64.tar.gz",
        url: "https://nodejs.org/dist/v20.20.2/node-v20.20.2-darwin-arm64.tar.gz",
        sha256: "466e05f3477c20dfb723054dfebffe55bc74660ee77f612166fca121dacb65b6",
        support_label: "已停止维护 · 仅兼容旧项目",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Node,
        series: "22",
        version: "22.23.1",
        archive: "node-v22.23.1-darwin-arm64.tar.gz",
        url: "https://nodejs.org/dist/v22.23.1/node-v22.23.1-darwin-arm64.tar.gz",
        sha256: "ef28d8fab2c0e4314522d4bb1b7173270aa3937e93b92cb7de79c112ac1fa953",
        support_label: "LTS · Jod",
        recommended: false,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Node,
        series: "24",
        version: "24.18.0",
        archive: "node-v24.18.0-darwin-arm64.tar.gz",
        url: "https://nodejs.org/dist/v24.18.0/node-v24.18.0-darwin-arm64.tar.gz",
        sha256: "e1a97e14c99c803e96c7339403282ea05a499c32f8d83defe9ef5ec66f979ed1",
        support_label: "LTS · Krypton · 推荐",
        recommended: true,
    },
    RuntimeRelease {
        kind: RuntimeKindInput::Node,
        series: "26",
        version: "26.5.0",
        archive: "node-v26.5.0-darwin-arm64.tar.gz",
        url: "https://nodejs.org/dist/v26.5.0/node-v26.5.0-darwin-arm64.tar.gz",
        sha256: "ee920559aaa2391569cff4d737e3b83963430e3a14dedd91bfe0ff53171b5af9",
        support_label: "Current · 新特性版本",
        recommended: false,
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVersionInfo {
    kind: String,
    series: String,
    version: String,
    support_label: String,
    legacy: bool,
    recommended: bool,
    installed: bool,
    selected: bool,
    compatible: bool,
    platform_label: String,
    installation_path: String,
    executable_path: String,
    disk_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOverview {
    kind: String,
    name: String,
    selected_version: Option<String>,
    installed_count: u32,
    total_disk_bytes: u64,
    platform_label: String,
    compatible: bool,
    versions: Vec<RuntimeVersionInfo>,
    environment: Vec<RuntimeEnvironmentVariable>,
    go_proxy: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEnvironmentVariable {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnostic {
    success: bool,
    version: String,
    executable: String,
    output: String,
    environment: Vec<RuntimeEnvironmentVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProject {
    id: String,
    name: String,
    path: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    services: Vec<String>,
    go_version: Option<String>,
    java_version: Option<String>,
    #[serde(default)]
    rust_version: Option<String>,
    #[serde(default)]
    python_version: Option<String>,
    #[serde(default)]
    node_version: Option<String>,
    created_at_millis: u64,
    updated_at_millis: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RuntimeSettings {
    defaults: BTreeMap<String, String>,
    go_proxy: String,
}

impl RuntimeSettings {
    fn normalized(mut self) -> Self {
        if self.go_proxy.trim().is_empty() {
            self.go_proxy = "https://proxy.golang.org,direct".into();
        }
        self
    }
}

pub trait RuntimeManager {
    fn versions(&self) -> Result<Vec<RuntimeVersionInfo>, String>;
    fn install(&self, version: &str) -> Result<(), String>;
    fn uninstall(&self, version: &str) -> Result<(), String>;
    fn select_default(&self, version: &str) -> Result<(), String>;
    fn environment(&self, version: Option<&str>)
        -> Result<Vec<RuntimeEnvironmentVariable>, String>;
    fn diagnose(&self, version: Option<&str>) -> Result<RuntimeDiagnostic, String>;
}

struct ManagedRuntime {
    kind: RuntimeKindInput,
    root: PathBuf,
}

impl ManagedRuntime {
    fn new(kind: RuntimeKindInput) -> Result<Self, String> {
        Ok(Self {
            kind,
            root: crate::settings::devbox_root()?,
        })
    }

    fn release(&self, version: &str) -> Result<&'static RuntimeRelease, String> {
        RELEASES
            .iter()
            .find(|release| release.kind == self.kind && release.version == version)
            .ok_or_else(|| format!("不支持的 {} 版本：{version}", self.kind.name()))
    }

    fn releases(&self) -> Vec<&'static RuntimeRelease> {
        RELEASES
            .iter()
            .filter(|release| release.kind == self.kind)
            .collect()
    }

    fn installation_dir(&self, version: &str) -> PathBuf {
        self.root
            .join("runtimes")
            .join(self.kind.id())
            .join(version)
    }

    fn runtime_home(&self, version: &str) -> PathBuf {
        self.installation_dir(version).join("home")
    }

    fn executable(&self, version: &str) -> PathBuf {
        let relative = match self.kind {
            RuntimeKindInput::Go => "bin/go",
            RuntimeKindInput::Java => "bin/java",
            RuntimeKindInput::Rust => "bin/rustc",
            RuntimeKindInput::Python => "bin/python3",
            RuntimeKindInput::Node => "bin/node",
        };
        self.runtime_home(version).join(relative)
    }

    fn selected_version(&self) -> Option<String> {
        let selected = load_runtime_settings(&self.root)
            .defaults
            .get(self.kind.id())
            .cloned()?;
        self.release(&selected)
            .ok()
            .filter(|release| self.is_installed(release))
            .map(|_| selected)
    }

    fn resolve_version(&self, version: Option<&str>) -> Result<String, String> {
        let resolved = version
            .map(str::to_string)
            .or_else(|| self.selected_version())
            .ok_or_else(|| format!("请先选择一个 {} 运行版本", self.kind.name()))?;
        let release = self.release(&resolved)?;
        if !self.is_installed(release) {
            return Err(format!("{} {} 尚未安装", self.kind.name(), resolved));
        }
        Ok(resolved)
    }

    fn is_installed(&self, release: &RuntimeRelease) -> bool {
        runtime_diagnostic_commands(self.kind)
            .iter()
            .all(|(binary, _)| {
                self.runtime_home(release.version)
                    .join("bin")
                    .join(binary)
                    .is_file()
            })
            && read_manifest_sha(&self.installation_dir(release.version))
                .is_some_and(|sha| sha == release.sha256)
    }
}

impl RuntimeManager for ManagedRuntime {
    fn versions(&self) -> Result<Vec<RuntimeVersionInfo>, String> {
        let selected = self.selected_version();
        let (compatible, platform_label) = platform_compatibility();
        self.releases()
            .into_iter()
            .map(|release| {
                let installation = self.installation_dir(release.version);
                let installed = self.is_installed(release);
                Ok(RuntimeVersionInfo {
                    kind: self.kind.id().into(),
                    series: release.series.into(),
                    version: release.version.into(),
                    support_label: release.support_label.into(),
                    legacy: release_is_legacy(release),
                    recommended: release.recommended,
                    installed,
                    selected: selected.as_deref() == Some(release.version),
                    compatible,
                    platform_label: platform_label.clone(),
                    installation_path: installation.display().to_string(),
                    executable_path: self.executable(release.version).display().to_string(),
                    disk_bytes: if installed {
                        crate::commands::path_disk_size(&installation).unwrap_or_default()
                    } else {
                        0
                    },
                })
            })
            .collect()
    }

    fn install(&self, version: &str) -> Result<(), String> {
        let release = self.release(version)?;
        let (compatible, label) = platform_compatibility();
        if !compatible {
            return Err(format!("当前版本暂不支持 {label}"));
        }
        if self.is_installed(release) {
            report_install_progress(90, "已安装", "目标运行时已经安装");
            return Ok(());
        }

        report_install_progress(
            3,
            "准备安装",
            format!("准备安装 {} {}", self.kind.name(), release.version),
        );
        let archive = prepare_runtime_archive(&self.root, release)?;
        check_install_cancelled().map_err(|e| e.to_string())?;

        let work = self.root.join("tmp").join(format!(
            "runtime-{}-{}-{}-{}",
            self.kind.id(),
            release.version.replace('+', "_"),
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let stage = work.join("installation");
        let result: Result<(), String> = (|| {
            report_install_progress(55, "解压运行时", "正在解压官方运行时安装包");
            let status = Command::new("/usr/bin/tar")
                .args(["-xzf"])
                .arg(&archive)
                .arg("-C")
                .arg(&work)
                .status()
                .map_err(|e| format!("无法执行 tar: {e}"))?;
            if !status.success() {
                return Err("运行时压缩包解压失败".into());
            }
            check_install_cancelled().map_err(|e| e.to_string())?;
            fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
            let home = stage.join("home");
            if self.kind == RuntimeKindInput::Rust {
                install_rust_standalone(&work, &home, release)?;
            } else {
                let extracted_home = match self.kind {
                    RuntimeKindInput::Go => work.join("go"),
                    RuntimeKindInput::Java => find_java_home(&work)?,
                    RuntimeKindInput::Python => work.join("python"),
                    RuntimeKindInput::Node => {
                        work.join(release.archive.trim_end_matches(".tar.gz"))
                    }
                    RuntimeKindInput::Rust => unreachable!(),
                };
                if !extracted_home.is_dir() {
                    return Err("安装包结构不符合预期：没有找到运行时目录".into());
                }
                fs::rename(&extracted_home, &home).map_err(|e| e.to_string())?;
            }
            write_runtime_manifest(&stage, release)?;
            verify_runtime(self.kind, &home, release.version)?;
            report_install_progress(88, "写入版本", "正在原子写入运行时版本目录");
            replace_runtime_installation(&stage, &self.installation_dir(release.version))?;
            Ok(())
        })();
        let _ = fs::remove_dir_all(&work);
        result?;
        if self.selected_version().is_none() {
            self.select_default(release.version)?;
        }
        report_install_progress(94, "安装完成", "运行时已经可以使用");
        Ok(())
    }

    fn uninstall(&self, version: &str) -> Result<(), String> {
        self.release(version)?;
        let target = self.installation_dir(version);
        let runtime_root = self.root.join("runtimes").join(self.kind.id());
        if !target.starts_with(&runtime_root) {
            return Err("运行时卸载路径不安全".into());
        }
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|e| format!("无法卸载运行时: {e}"))?;
        }
        let mut settings = load_runtime_settings(&self.root);
        if settings.defaults.get(self.kind.id()).map(String::as_str) == Some(version) {
            settings.defaults.remove(self.kind.id());
            if let Some(next) = self
                .releases()
                .into_iter()
                .rev()
                .find(|release| self.is_installed(release))
            {
                settings
                    .defaults
                    .insert(self.kind.id().into(), next.version.into());
            }
            save_runtime_settings(&self.root, &settings)?;
        }
        Ok(())
    }

    fn select_default(&self, version: &str) -> Result<(), String> {
        let release = self.release(version)?;
        if !self.is_installed(release) {
            return Err(format!("请先安装 {} {version}", self.kind.name()));
        }
        let mut settings = load_runtime_settings(&self.root);
        settings
            .defaults
            .insert(self.kind.id().into(), version.into());
        save_runtime_settings(&self.root, &settings)
    }

    fn environment(
        &self,
        version: Option<&str>,
    ) -> Result<Vec<RuntimeEnvironmentVariable>, String> {
        let version = self.resolve_version(version)?;
        let home = self.runtime_home(&version);
        let mut values = match self.kind {
            RuntimeKindInput::Go => {
                let workspace = self.root.join("workspaces/go");
                vec![
                    env_var("GOROOT", &home),
                    env_var("GOPATH", &workspace),
                    env_var("GOBIN", workspace.join("bin")),
                    env_var("GOMODCACHE", workspace.join("pkg/mod")),
                    env_var("GOCACHE", self.root.join("caches/go-build")),
                    RuntimeEnvironmentVariable {
                        key: "GOPROXY".into(),
                        value: load_runtime_settings(&self.root).go_proxy,
                    },
                ]
            }
            RuntimeKindInput::Java => vec![env_var("JAVA_HOME", &home)],
            RuntimeKindInput::Rust => {
                let cargo_home = self.root.join("workspaces/rust/cargo");
                vec![
                    env_var("CARGO_HOME", &cargo_home),
                    RuntimeEnvironmentVariable {
                        key: "PATH".into(),
                        value: format!("{}/bin:{}/bin:$PATH", home.display(), cargo_home.display()),
                    },
                ]
            }
            RuntimeKindInput::Python => {
                let user_base = self.root.join("workspaces/python").join(&version);
                vec![
                    env_var("PYTHONUSERBASE", &user_base),
                    env_var("PIP_CACHE_DIR", self.root.join("caches/pip")),
                    RuntimeEnvironmentVariable {
                        key: "PATH".into(),
                        value: format!("{}/bin:{}/bin:$PATH", home.display(), user_base.display()),
                    },
                ]
            }
            RuntimeKindInput::Node => {
                let npm_prefix = self.root.join("workspaces/node").join(&version);
                vec![
                    env_var("NPM_CONFIG_PREFIX", &npm_prefix),
                    env_var("NPM_CONFIG_CACHE", self.root.join("caches/npm")),
                    RuntimeEnvironmentVariable {
                        key: "PATH".into(),
                        value: format!("{}/bin:{}/bin:$PATH", home.display(), npm_prefix.display()),
                    },
                ]
            }
        };
        if !values.iter().any(|variable| variable.key == "PATH") {
            values.push(RuntimeEnvironmentVariable {
                key: "PATH".into(),
                value: format!("{}/bin:$PATH", home.display()),
            });
        }
        Ok(values)
    }

    fn diagnose(&self, version: Option<&str>) -> Result<RuntimeDiagnostic, String> {
        let version = self.resolve_version(version)?;
        let executable = self.executable(&version);
        let mut success = true;
        let mut text = String::new();
        for (binary, argument) in runtime_diagnostic_commands(self.kind) {
            let path = self.runtime_home(&version).join("bin").join(binary);
            let output = Command::new(&path)
                .arg(argument)
                .output()
                .map_err(|e| format!("无法执行 {binary} 诊断: {e}"))?;
            success &= output.status.success();
            text.push_str(&format!("$ {binary} {argument}\n"));
            text.push_str(&String::from_utf8_lossy(&output.stdout));
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            text.push('\n');
        }
        let environment = self.environment(Some(&version))?;
        Ok(RuntimeDiagnostic {
            success: success && text.contains(&expected_version_fragment(self.kind, &version)),
            version,
            executable: executable.display().to_string(),
            output: text.trim().into(),
            environment,
        })
    }
}

fn version_series(version: &str) -> String {
    if version.starts_with("1.") {
        version.split('.').take(2).collect::<Vec<_>>().join(".")
    } else {
        version.split('.').next().unwrap_or(version).into()
    }
}

fn env_var(key: &str, value: impl AsRef<Path>) -> RuntimeEnvironmentVariable {
    RuntimeEnvironmentVariable {
        key: key.into(),
        value: value.as_ref().display().to_string(),
    }
}

fn platform_compatibility() -> (bool, String) {
    let label = format!("{} · {}", std::env::consts::OS, std::env::consts::ARCH);
    (
        cfg!(all(target_os = "macos", target_arch = "aarch64")),
        label,
    )
}

fn prepare_runtime_archive(root: &Path, release: &RuntimeRelease) -> Result<PathBuf, String> {
    let cache = root.join("runtime-cache");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let archive = cache.join(release.archive);
    if archive.is_file() && file_sha256(&archive)? == release.sha256 {
        report_install_progress(45, "复用缓存", "安装包缓存校验通过");
        return Ok(archive);
    }
    if archive.exists() {
        fs::remove_file(&archive).map_err(|e| e.to_string())?;
    }
    let settings = crate::settings::load_settings();
    let mut candidates = Vec::new();
    if !settings.download_mirror.trim().is_empty() {
        candidates.push((
            "自定义镜像",
            format!(
                "{}/{}",
                settings.download_mirror.trim_end_matches('/'),
                release.archive
            ),
        ));
    }
    if settings.public_github_mirror && release.url.starts_with("https://github.com/") {
        candidates.push((
            "GitHub 公共加速",
            format!("https://gh-proxy.com/{}", release.url),
        ));
    }
    candidates.push(("官方源", release.url.into()));
    let mut last_error = String::new();
    for (index, (label, url)) in candidates.iter().enumerate() {
        report_install_progress(
            8,
            "下载安装包",
            format!("尝试下载源 {}/{}（{label}）", index + 1, candidates.len()),
        );
        match download_with_resume(
            url,
            &archive,
            settings.download_timeout_seconds,
            release.archive,
        ) {
            Ok(()) => {
                report_install_progress(42, "校验安装包", "正在计算 SHA-256");
                if file_sha256(&archive)? == release.sha256 {
                    return Ok(archive);
                }
                last_error = format!("{label} 下载的安装包 SHA-256 不匹配");
                let _ = fs::remove_file(&archive);
            }
            Err(error) => last_error = error,
        }
        check_install_cancelled().map_err(|e| e.to_string())?;
    }
    Err(if last_error.is_empty() {
        "没有可用的运行时下载源".into()
    } else {
        last_error
    })
}

fn download_with_resume(
    url: &str,
    target: &Path,
    timeout_seconds: u64,
    archive_name: &str,
) -> Result<(), String> {
    let partial = target.with_extension(format!(
        "{}.part",
        target
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("download")
    ));
    let existing = partial.metadata().map(|meta| meta.len()).unwrap_or(0);
    let client = crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Download)?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(timeout_seconds.max(30)))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.get(url);
    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
    }
    let mut response = request.send().map_err(|e| format!("下载失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("下载返回 HTTP {}", response.status()));
    }
    let resumed = existing > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut file = if resumed {
        OpenOptions::new()
            .append(true)
            .open(&partial)
            .map_err(|e| e.to_string())?
    } else {
        File::create(&partial).map_err(|e| e.to_string())?
    };
    let start = if resumed { existing } else { 0 };
    let total = response.content_length().map(|size| size + start);
    let mut downloaded = start;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check_install_cancelled().map_err(|e| e.to_string())?;
        let count = response
            .read(&mut buffer)
            .map_err(|e| format!("读取下载内容失败: {e}"))?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|e| format!("写入下载缓存失败: {e}"))?;
        downloaded += count as u64;
        if let Some(total) = total {
            let progress = 8 + ((downloaded.saturating_mul(31) / total.max(1)) as u8).min(31);
            report_install_progress(
                progress,
                "下载安装包",
                format!(
                    "{archive_name} · {}%",
                    downloaded.saturating_mul(100) / total.max(1)
                ),
            );
        }
    }
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&partial, target).map_err(|e| e.to_string())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn find_java_home(work: &Path) -> Result<PathBuf, String> {
    for entry in fs::read_dir(work).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        for candidate in [path.join("Contents/Home"), path.clone()] {
            if candidate.join("bin/java").is_file() {
                return Ok(candidate);
            }
        }
    }
    Err("Temurin 安装包中没有找到 JAVA_HOME".into())
}

fn install_rust_standalone(
    work: &Path,
    home: &Path,
    release: &RuntimeRelease,
) -> Result<(), String> {
    let source = work.join(release.archive.trim_end_matches(".tar.gz"));
    let installer = source.join("install.sh");
    if !installer.is_file() {
        return Err("Rust 官方安装包中没有找到 install.sh".into());
    }
    check_install_cancelled().map_err(|e| e.to_string())?;
    let status = Command::new("/bin/sh")
        .arg(installer)
        .arg(format!("--prefix={}", home.display()))
        .arg("--disable-ldconfig")
        .status()
        .map_err(|e| format!("无法执行 Rust standalone 安装器: {e}"))?;
    check_install_cancelled().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Rust standalone 安装器执行失败".into());
    }
    Ok(())
}

fn runtime_diagnostic_commands(kind: RuntimeKindInput) -> &'static [(&'static str, &'static str)] {
    match kind {
        RuntimeKindInput::Go => &[("go", "version")],
        RuntimeKindInput::Java => &[("java", "-version"), ("javac", "-version")],
        RuntimeKindInput::Rust => &[("rustc", "--version"), ("cargo", "--version")],
        RuntimeKindInput::Python => &[("python3", "--version"), ("pip3", "--version")],
        RuntimeKindInput::Node => &[("node", "--version"), ("npm", "--version")],
    }
}

fn expected_version_fragment(kind: RuntimeKindInput, version: &str) -> String {
    match kind {
        RuntimeKindInput::Java => version_series(version),
        RuntimeKindInput::Python => version.split('+').next().unwrap_or(version).into(),
        RuntimeKindInput::Go | RuntimeKindInput::Rust | RuntimeKindInput::Node => version.into(),
    }
}

fn verify_runtime(kind: RuntimeKindInput, home: &Path, version: &str) -> Result<(), String> {
    let mut text = String::new();
    for (binary, argument) in runtime_diagnostic_commands(kind) {
        let executable = home.join("bin").join(binary);
        let output = Command::new(&executable)
            .arg(argument)
            .output()
            .map_err(|e| format!("{binary} 验证失败: {e}"))?;
        if !output.status.success() {
            return Err(format!("下载的 {} 缺少可用的 {binary}", kind.name()));
        }
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    if !text.contains(&expected_version_fragment(kind, version)) {
        return Err(format!("下载的程序不是预期的 {} {version}", kind.name()));
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct RuntimeManifest {
    kind: String,
    series: String,
    version: String,
    source_url: String,
    sha256: String,
    installed_at_millis: u64,
}

fn write_runtime_manifest(stage: &Path, release: &RuntimeRelease) -> Result<(), String> {
    let manifest = RuntimeManifest {
        kind: release.kind.id().into(),
        series: release.series.into(),
        version: release.version.into(),
        source_url: release.url.into(),
        sha256: release.sha256.into(),
        installed_at_millis: now_millis(),
    };
    fs::write(
        stage.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn read_manifest_sha(installation: &Path) -> Option<String> {
    fs::read(installation.join("manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<RuntimeManifest>(&bytes).ok())
        .map(|manifest| manifest.sha256)
}

fn replace_runtime_installation(stage: &Path, target: &Path) -> Result<(), String> {
    let parent = target
        .parent()
        .ok_or_else(|| "运行时安装目录无效".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let backup = parent.join(format!(".backup-{}-{}", std::process::id(), now_millis()));
    if target.exists() {
        fs::rename(target, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(error) = fs::rename(stage, target) {
        if backup.exists() {
            let _ = fs::rename(&backup, target);
        }
        return Err(format!("无法提交运行时安装目录: {error}"));
    }
    if backup.exists() {
        let _ = fs::remove_dir_all(backup);
    }
    Ok(())
}

fn runtime_settings_path(root: &Path) -> PathBuf {
    root.join("runtime-profiles/settings.json")
}

fn load_runtime_settings(root: &Path) -> RuntimeSettings {
    fs::read(runtime_settings_path(root))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<RuntimeSettings>(&bytes).ok())
        .unwrap_or_default()
        .normalized()
}

fn save_runtime_settings(root: &Path, settings: &RuntimeSettings) -> Result<(), String> {
    let path = runtime_settings_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

fn projects_path(root: &Path) -> PathBuf {
    root.join("runtime-profiles/projects.json")
}

fn load_projects(root: &Path) -> Vec<RuntimeProject> {
    fs::read(projects_path(root))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<RuntimeProject>>(&bytes).ok())
        .unwrap_or_default()
}

fn save_projects(root: &Path, projects: &[RuntimeProject]) -> Result<(), String> {
    let path = projects_path(root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(projects).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[tauri::command]
pub fn runtime_overview(kind: RuntimeKindInput) -> Result<RuntimeOverview, String> {
    let manager = ManagedRuntime::new(kind)?;
    let versions = manager.versions()?;
    let selected_version = manager.selected_version();
    let environment = manager.environment(None).unwrap_or_default();
    let settings = load_runtime_settings(&manager.root);
    let (compatible, platform_label) = platform_compatibility();
    Ok(RuntimeOverview {
        kind: kind.id().into(),
        name: kind.name().into(),
        selected_version,
        installed_count: versions.iter().filter(|item| item.installed).count() as u32,
        total_disk_bytes: versions.iter().map(|item| item.disk_bytes).sum(),
        platform_label,
        compatible,
        versions,
        environment,
        go_proxy: settings.go_proxy,
    })
}

#[tauri::command]
pub async fn runtime_install(
    app: AppHandle,
    kind: RuntimeKindInput,
    version: String,
    operation_id: String,
) -> Result<RuntimeOverview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::commands::run_install_task(
            app,
            operation_id,
            format!("runtime-{}", kind.id()),
            || {
                ManagedRuntime::new(kind)?.install(&version)?;
                runtime_overview(kind)
            },
        )
    })
    .await
    .map_err(|e| format!("运行时安装任务异常结束: {e}"))?
}

#[tauri::command]
pub fn runtime_select(kind: RuntimeKindInput, version: String) -> Result<RuntimeOverview, String> {
    ManagedRuntime::new(kind)?.select_default(&version)?;
    runtime_overview(kind)
}

#[tauri::command]
pub fn runtime_uninstall(
    kind: RuntimeKindInput,
    version: String,
) -> Result<RuntimeOverview, String> {
    ManagedRuntime::new(kind)?.uninstall(&version)?;
    runtime_overview(kind)
}

#[tauri::command]
pub fn runtime_diagnose(
    kind: RuntimeKindInput,
    version: Option<String>,
) -> Result<RuntimeDiagnostic, String> {
    ManagedRuntime::new(kind)?.diagnose(version.as_deref())
}

#[tauri::command]
pub fn runtime_go_proxy_set(proxy: String) -> Result<RuntimeOverview, String> {
    let proxy = proxy.trim();
    if proxy.is_empty() || proxy.contains('\n') || proxy.contains('\r') {
        return Err("GOPROXY 配置无效".into());
    }
    let root = crate::settings::devbox_root()?;
    let mut settings = load_runtime_settings(&root);
    settings.go_proxy = proxy.into();
    save_runtime_settings(&root, &settings)?;
    runtime_overview(RuntimeKindInput::Go)
}

#[tauri::command]
pub fn runtime_projects_list() -> Result<Vec<RuntimeProject>, String> {
    Ok(load_projects(&crate::settings::devbox_root()?))
}

#[tauri::command]
pub fn runtime_project_save(mut project: RuntimeProject) -> Result<Vec<RuntimeProject>, String> {
    let root = crate::settings::devbox_root()?;
    let canonical = PathBuf::from(&project.path)
        .canonicalize()
        .map_err(|_| "项目目录不存在".to_string())?;
    if !canonical.is_dir() {
        return Err("项目路径必须是目录".into());
    }
    project.name = project.name.trim().to_string();
    project.description = project.description.trim().to_string();
    if project.name.is_empty() {
        return Err("项目名称不能为空".into());
    }
    if project.name.chars().count() > 80 {
        return Err("项目名称不能超过 80 个字符".into());
    }
    project.services.sort();
    project.services.dedup();
    if let Some(invalid) = project
        .services
        .iter()
        .find(|kind| !is_supported_project_service(kind))
    {
        return Err(format!("不支持的项目服务：{invalid}"));
    }
    if let Some(version) = project.go_version.as_deref() {
        validate_project_runtime(RuntimeKindInput::Go, version)?;
    }
    if let Some(version) = project.java_version.as_deref() {
        validate_project_runtime(RuntimeKindInput::Java, version)?;
    }
    if let Some(version) = project.rust_version.as_deref() {
        validate_project_runtime(RuntimeKindInput::Rust, version)?;
    }
    if let Some(version) = project.python_version.as_deref() {
        validate_project_runtime(RuntimeKindInput::Python, version)?;
    }
    if let Some(version) = project.node_version.as_deref() {
        validate_project_runtime(RuntimeKindInput::Node, version)?;
    }
    let path = canonical.display().to_string();
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    project.id = format!("{:x}", hasher.finalize())[..16].into();
    project.path = path;
    let now = now_millis();
    let mut projects = load_projects(&root);
    if let Some(existing) = projects.iter_mut().find(|item| item.id == project.id) {
        project.created_at_millis = existing.created_at_millis;
        project.updated_at_millis = now;
        *existing = project;
    } else {
        project.created_at_millis = now;
        project.updated_at_millis = now;
        projects.push(project);
    }
    save_projects(&root, &projects)?;
    Ok(projects)
}

fn is_supported_project_service(kind: &str) -> bool {
    matches!(
        kind,
        "redis"
            | "mysql"
            | "postgres"
            | "mongodb"
            | "mailpit"
            | "nats"
            | "kafka"
            | "meilisearch"
            | "minio"
            | "rustfs"
            | "etcd"
            | "consul"
            | "rnacos"
            | "rabbitmq"
            | "nginx"
            | "caddy"
    )
}

#[tauri::command]
pub fn runtime_project_manifest_export(id: String) -> Result<String, String> {
    let root = crate::settings::devbox_root()?;
    let project = load_projects(&root)
        .into_iter()
        .find(|project| project.id == id)
        .ok_or_else(|| "项目工作区不存在".to_string())?;
    let directory = PathBuf::from(&project.path);
    if !directory.is_dir() {
        return Err("项目目录不存在".into());
    }
    let path = directory.join(".zhiyu-env.json");
    let temp = directory.join(".zhiyu-env.json.tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(&project).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temp, &path).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn runtime_project_manifest_import(path: String) -> Result<Vec<RuntimeProject>, String> {
    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| "项目目录不存在".to_string())?;
    if !canonical.is_dir() {
        return Err("项目路径必须是目录".into());
    }
    let manifest = canonical.join(".zhiyu-env.json");
    let bytes =
        fs::read(&manifest).map_err(|_| format!("未找到项目清单：{}", manifest.display()))?;
    let mut project: RuntimeProject =
        serde_json::from_slice(&bytes).map_err(|e| format!("项目清单格式错误：{e}"))?;
    project.path = canonical.display().to_string();
    runtime_project_save(project)
}

fn validate_project_runtime(kind: RuntimeKindInput, version: &str) -> Result<(), String> {
    let manager = ManagedRuntime::new(kind)?;
    let release = manager.release(version)?;
    if !manager.is_installed(release) {
        return Err(format!("{} {version} 尚未安装", kind.name()));
    }
    Ok(())
}

#[tauri::command]
pub fn runtime_project_delete(id: String) -> Result<Vec<RuntimeProject>, String> {
    let root = crate::settings::devbox_root()?;
    let mut projects = load_projects(&root);
    projects.retain(|project| project.id != id);
    save_projects(&root, &projects)?;
    Ok(projects)
}

#[tauri::command]
pub fn runtime_open_terminal(
    kind: RuntimeKindInput,
    project_path: Option<String>,
    version: Option<String>,
) -> Result<(), String> {
    let manager = ManagedRuntime::new(kind)?;
    let environment = manager.environment(version.as_deref())?;
    let shell_dir = manager.root.join("runtime-shells");
    fs::create_dir_all(&shell_dir).map_err(|e| e.to_string())?;
    let script = shell_dir.join(format!("{}-{}.command", kind.id(), now_millis()));
    let mut body = String::from("#!/bin/zsh\n");
    for variable in environment {
        if variable.key == "PATH" {
            let prefix = variable.value.trim_end_matches(":$PATH");
            body.push_str(&format!("export PATH={}:\"$PATH\"\n", shell_quote(prefix)));
        } else {
            body.push_str(&format!(
                "export {}={}\n",
                variable.key,
                shell_quote(&variable.value)
            ));
        }
    }
    if let Some(path) = project_path {
        let canonical = PathBuf::from(path)
            .canonicalize()
            .map_err(|_| "项目目录不存在".to_string())?;
        body.push_str(&format!(
            "cd {}\n",
            shell_quote(&canonical.display().to_string())
        ));
    }
    body.push_str("echo \"Zhiyu ");
    body.push_str(kind.name());
    body.push_str(" Development Shell\"\nexec \"${SHELL:-/bin/zsh}\" -l\n");
    fs::write(&script, body).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("/usr/bin/open")
            .args(["-a", "Terminal"])
            .arg(&script)
            .spawn()
            .map_err(|e| format!("无法打开开发终端: {e}"))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = script;
        Err("当前版本的开发终端暂只支持 macOS".into())
    }
}

fn release_is_legacy(release: &RuntimeRelease) -> bool {
    matches!(
        (release.kind, release.series),
        (RuntimeKindInput::Go, "1.22" | "1.23" | "1.24")
            | (RuntimeKindInput::Java, "24")
            | (RuntimeKindInput::Node, "18" | "20")
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_catalog_has_at_least_five_verified_versions_for_every_runtime() {
        for kind in [
            RuntimeKindInput::Go,
            RuntimeKindInput::Java,
            RuntimeKindInput::Rust,
            RuntimeKindInput::Python,
            RuntimeKindInput::Node,
        ] {
            let releases: Vec<_> = RELEASES
                .iter()
                .filter(|release| release.kind == kind)
                .collect();
            assert!(releases.len() >= 5, "{} catalog", kind.name());
            assert_eq!(
                releases
                    .iter()
                    .filter(|release| release.recommended)
                    .count(),
                1,
                "{} recommendation",
                kind.name()
            );
            assert!(releases.iter().all(|release| {
                release.sha256.len() == 64
                    && release.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && release.url.starts_with("https://")
            }));
        }
    }

    #[test]
    fn shell_values_are_safely_quoted() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("it's"), "'it'\"'\"'s'");
    }

    #[test]
    fn version_series_matches_runtime_output_prefixes() {
        assert_eq!(version_series("1.26.5"), "1.26");
        assert_eq!(version_series("21.0.11+10"), "21");
        assert_eq!(
            expected_version_fragment(RuntimeKindInput::Python, "3.14.6+20260718"),
            "3.14.6"
        );
        assert_eq!(
            expected_version_fragment(RuntimeKindInput::Node, "24.18.0"),
            "24.18.0"
        );
    }

    #[test]
    fn legacy_project_profiles_load_without_new_runtime_fields() {
        let project: RuntimeProject = serde_json::from_value(serde_json::json!({
            "id": "demo",
            "name": "Demo",
            "path": "/tmp/demo",
            "goVersion": "1.26.5",
            "javaVersion": null,
            "createdAtMillis": 1,
            "updatedAtMillis": 2
        }))
        .unwrap();
        assert_eq!(project.go_version.as_deref(), Some("1.26.5"));
        assert!(project.rust_version.is_none());
        assert!(project.python_version.is_none());
        assert!(project.node_version.is_none());
        assert!(project.description.is_empty());
        assert!(project.services.is_empty());
    }

    #[test]
    fn project_service_allowlist_rejects_unknown_processes() {
        assert!(is_supported_project_service("redis"));
        assert!(is_supported_project_service("postgres"));
        assert!(!is_supported_project_service("docker"));
        assert!(!is_supported_project_service("../redis"));
    }
}
