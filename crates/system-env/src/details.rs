use crate::env::{SystemArch, SystemOS, SystemPlatform};
use crate::helpers::find_command_on_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

/// A Linux distribution, as defined by its `os-release` file.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(default)]
pub struct SystemDistro {
    /// Identifier of the distribution (the `ID` field), for example,
    /// `ubuntu`, `debian`, `fedora`, or `alpine`.
    pub id: String,

    /// Identifiers of the distributions that this distribution is derived from,
    /// or closely related to (the `ID_LIKE` field), ordered from the closest to
    /// the most distant, for example, `rhel`, `centos`, and `fedora` on Rocky
    /// Linux. Empty when the distribution doesn't define any.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub id_like: Vec<String>,

    /// Version of the distribution (the `VERSION_ID` field), for example,
    /// `24.04`, `12`, `40`, or `3.20.3`. Rolling releases, like Arch Linux,
    /// don't define a version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

impl SystemDistro {
    /// Detect the distribution from the `/etc/os-release` file, falling back to
    /// `/usr/lib/os-release`. Returns none if neither file exists, or the file
    /// doesn't define an identifier.
    pub fn detect() -> Option<Self> {
        Self::from_file("/etc/os-release").or_else(|| Self::from_file("/usr/lib/os-release"))
    }

    /// Load the distribution from the provided `os-release` file. Returns none
    /// if the file can't be read, or doesn't define an identifier.
    pub fn from_file(path: impl AsRef<Path>) -> Option<Self> {
        Self::parse_os_release(&fs::read_to_string(path).ok()?)
    }

    /// Parse the contents of an `os-release` file, which is a list of `KEY=value`
    /// lines, where the value may be wrapped in single or double quotes. The
    /// `ID_LIKE` field is a space-separated list. Returns none if the contents
    /// don't define an identifier (the `ID` field).
    pub fn parse_os_release(content: &str) -> Option<Self> {
        let mut id = None;
        let mut id_like = vec![];
        let mut version_id = None;

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let value = unquote_os_release_value(value.trim());

            if value.is_empty() {
                continue;
            }

            match key.trim() {
                "ID" => id = Some(value.to_lowercase()),
                "ID_LIKE" => {
                    id_like = value
                        .split_whitespace()
                        .map(|like| like.to_lowercase())
                        .collect()
                }
                "VERSION_ID" => version_id = Some(value),
                _ => {}
            };
        }

        Some(Self {
            id: id?,
            id_like,
            version_id,
        })
    }
}

fn unquote_os_release_value(value: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|inner| inner.strip_suffix(quote))
        {
            // Double quoted values may escape characters with a backslash
            if quote == '"' {
                let mut unescaped = String::with_capacity(inner.len());
                let mut chars = inner.chars();

                while let Some(ch) = chars.next() {
                    if ch == '\\'
                        && let Some(next) = chars.next()
                    {
                        unescaped.push(next);
                    } else {
                        unescaped.push(ch);
                    }
                }

                return unescaped;
            }

            return inner.to_owned();
        }
    }

    value.to_owned()
}

/// Details about the hardware and operating system of the system environment,
/// that further distinguish it from other machines with the same platform
/// (architecture, operating system, and libc). Each detail is only available
/// where it applies, and when it could be detected.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(default)]
pub struct SystemDetails {
    /// Version of the ARM architecture (for example, 6 or 7), on 32-bit ARM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_version: Option<u8>,

    /// The Linux distribution, on Linux.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distro: Option<SystemDistro>,

    /// The x86-64 microarchitecture level (1 to 4), on x64. Each level supports
    /// the instructions of the levels below it, for example, AVX2 is level 3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x64_level: Option<u8>,
}

impl SystemDetails {
    /// Detect the details of the current system environment. On 32-bit ARM,
    /// this requires executing the `uname -m` command.
    pub fn from_env() -> Self {
        Self::from_platform(&SystemPlatform::new(
            SystemOS::from_env(),
            SystemArch::from_env(),
        ))
    }

    /// Detect the details of the current system environment that apply to the
    /// provided platform, which must have already been detected for this machine
    /// (for example, with [`SystemPlatform::from_env`]), so that it isn't detected
    /// again. Details can't be detected for another platform, as they would
    /// describe this machine instead. On 32-bit ARM, this requires executing the
    /// `uname -m` command.
    pub fn from_platform(platform: &SystemPlatform) -> Self {
        Self {
            arm_version: if platform.arch == SystemArch::Arm {
                detect_arm_version()
            } else {
                None
            },
            distro: if platform.os.is_linux() {
                SystemDistro::detect()
            } else {
                None
            },
            x64_level: if platform.arch == SystemArch::X64 {
                detect_x64_level()
            } else {
                None
            },
        }
    }

    /// Parse the version of the ARM architecture from the machine hardware name,
    /// as returned by `uname -m`, for example, `armv6l` is 6, and `armv7l` is 7.
    /// Returns none for other architectures.
    pub fn parse_arm_version(machine: &str) -> Option<u8> {
        let digits = machine
            .trim()
            .strip_prefix("armv")?
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();

        digits.parse().ok()
    }

    /// Return the x86-64 microarchitecture level for the supported instructions,
    /// where each argument is whether all the instructions of that level are
    /// supported. A level is only reached when all levels below it are too.
    pub fn get_x64_level(v2: bool, v3: bool, v4: bool) -> u8 {
        if !v2 {
            1
        } else if !v3 {
            2
        } else if !v4 {
            3
        } else {
            4
        }
    }
}

fn detect_arm_version() -> Option<u8> {
    let output = Command::new(find_command_on_path("uname")?)
        .arg("-m")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    SystemDetails::parse_arm_version(&String::from_utf8_lossy(&output.stdout))
}

// https://gitlab.com/x86-psABIs/x86-64-ABI
#[cfg(target_arch = "x86_64")]
fn detect_x64_level() -> Option<u8> {
    use std::arch::is_x86_feature_detected as has;

    let v2 = has!("cmpxchg16b")
        && has!("popcnt")
        && has!("sse3")
        && has!("ssse3")
        && has!("sse4.1")
        && has!("sse4.2");

    let v3 = has!("avx")
        && has!("avx2")
        && has!("bmi1")
        && has!("bmi2")
        && has!("f16c")
        && has!("fma")
        && has!("lzcnt")
        && has!("movbe")
        && has!("xsave");

    let v4 = has!("avx512f")
        && has!("avx512bw")
        && has!("avx512cd")
        && has!("avx512dq")
        && has!("avx512vl");

    Some(SystemDetails::get_x64_level(v2, v3, v4))
}

#[cfg(not(target_arch = "x86_64"))]
fn detect_x64_level() -> Option<u8> {
    None
}
