use crate::error::Error;
use crate::helpers::find_command_on_path;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::env::consts;
use std::fmt;
use std::process::Command;
use std::str::FromStr;

/// Architecture of the system environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemArch {
    X86,
    #[serde(alias = "x86_64")]
    X64,
    Arm,
    #[serde(alias = "aarch64")]
    Arm64,
    #[serde(alias = "loongarch64")]
    LongArm64,
    M68k,
    Mips,
    Mips64,
    Powerpc,
    Powerpc64,
    Riscv64,
    S390x,
    Sparc64,
}

impl SystemArch {
    /// Return an instance derived from [`std::env::consts::ARCH`].
    pub fn from_env() -> SystemArch {
        serde_json::from_value(Value::String(consts::ARCH.to_owned()))
            .expect("Unknown architecture!")
    }

    /// Convert to a [`std::env::consts::ARCH`] compatible string.
    pub fn to_rust_arch(&self) -> String {
        match self {
            Self::X64 => "x86_64".into(),
            Self::Arm64 => "aarch64".into(),
            Self::LongArm64 => "loongarch64".into(),
            _ => self.to_string(),
        }
    }
}

impl Default for SystemArch {
    #[cfg(target_arch = "wasm32")]
    fn default() -> Self {
        SystemArch::X64
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn default() -> Self {
        SystemArch::from_env()
    }
}

impl fmt::Display for SystemArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Arm => "arm",
            Self::Arm64 => "arm64",
            Self::LongArm64 => "longarm64",
            Self::M68k => "m68k",
            Self::Mips => "mips",
            Self::Mips64 => "mips64",
            Self::Powerpc => "powerpc",
            Self::Powerpc64 => "powerpc64",
            Self::Riscv64 => "riscv64",
            Self::S390x => "s390x",
            Self::Sparc64 => "sparc64",
            Self::X64 => "x64",
            Self::X86 => "x86",
        })
    }
}

/// Operating system of the current environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemOS {
    Android,
    Dragonfly,
    FreeBSD,
    IOS,
    Linux,
    #[serde(alias = "mac")]
    MacOS,
    NetBSD,
    OpenBSD,
    Solaris,
    Windows,
}

impl SystemOS {
    /// Return an instance derived from [`std::env::consts::OS`].
    pub fn from_env() -> SystemOS {
        serde_json::from_value(Value::String(consts::OS.to_owned()))
            .expect("Unknown operating system!")
    }

    /// Return either a Unix or Windows value based on the current native system.
    pub fn for_native<'value, T: AsRef<str> + ?Sized>(
        &self,
        unix: &'value T,
        windows: &'value T,
    ) -> &'value str {
        if self.is_windows() {
            windows.as_ref()
        } else {
            unix.as_ref()
        }
    }

    /// Return the provided name as a system formatted file name for executables.
    /// On Windows this will append an ".exe" extension. On Unix, no extension.
    pub fn get_exe_name(&self, name: impl AsRef<str>) -> String {
        self.get_file_name(name, "exe")
    }

    /// Return the provided file name formatted with the extension (without dot)
    /// when on Windows. On Unix, returns the name as-is.
    pub fn get_file_name(&self, name: impl AsRef<str>, windows_ext: impl AsRef<str>) -> String {
        let name = name.as_ref();
        let ext = windows_ext.as_ref();

        if self.is_windows() && !name.ends_with(ext) {
            format!("{name}.{ext}")
        } else {
            name.to_owned()
        }
    }

    /// Return true if in the BSD family.
    pub fn is_bsd(&self) -> bool {
        matches!(
            self,
            Self::Dragonfly | Self::FreeBSD | Self::NetBSD | Self::OpenBSD
        )
    }

    /// Return true if Linux.
    pub fn is_linux(&self) -> bool {
        matches!(self, Self::Linux)
    }

    /// Return true if MacOS.
    pub fn is_mac(&self) -> bool {
        matches!(self, Self::MacOS)
    }

    /// Return true if a Unix based OS.
    pub fn is_unix(&self) -> bool {
        self.is_bsd() || matches!(self, Self::Linux | Self::MacOS)
    }

    /// Return true if Windows.
    pub fn is_windows(&self) -> bool {
        matches!(self, Self::Windows)
    }

    /// Convert to a [`std::env::consts::OS`] compatible string.
    pub fn to_rust_os(&self) -> String {
        self.to_string()
    }
}

impl Default for SystemOS {
    #[cfg(target_arch = "wasm32")]
    fn default() -> Self {
        SystemOS::Linux
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn default() -> Self {
        SystemOS::from_env()
    }
}

impl fmt::Display for SystemOS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Android => "android",
            Self::Dragonfly => "dragonfly",
            Self::FreeBSD => "freebsd",
            Self::IOS => "ios",
            Self::Linux => "linux",
            Self::MacOS => "macos",
            Self::NetBSD => "netbsd",
            Self::OpenBSD => "openbsd",
            Self::Solaris => "solaris",
            Self::Windows => "windows",
        })
    }
}

/// Libc being used in the system environment.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemLibc {
    /// Android
    // Bionic,

    /// GNU C Library
    #[serde(alias = "glibc")]
    Gnu,

    /// macOS & iOS
    // #[serde(alias = "macos")]
    // LibSystem,

    /// Alpine Linux
    Musl,

    /// Microsoft Visual C++ / UCRT (Universal C Runtime)
    // #[serde(alias = "ucrt")]
    // Msvc,

    #[default]
    Unknown,
}

impl SystemLibc {
    /// Detect the libc type from the current system environment.
    pub fn detect(os: SystemOS) -> Self {
        match os {
            // SystemOS::Android => Self::Bionic,
            // SystemOS::IOS | SystemOS::MacOS => Self::LibSystem,
            // SystemOS::Windows => Self::Msvc,
            SystemOS::Android => Self::Unknown,
            SystemOS::IOS | SystemOS::MacOS => Self::Unknown,
            SystemOS::Windows => Self::Unknown,
            _ => {
                if Self::is_musl() {
                    Self::Musl
                } else {
                    Self::Gnu
                }
            }
        }
    }

    /// Check if musl is available on the current machine, by running the
    /// `ldd --version` command, or the `uname` command. This will return false
    /// on systems that have neither of those commands.
    pub fn is_musl() -> bool {
        let mut command = if let Some(ldd_path) = find_command_on_path("ldd") {
            let mut cmd = Command::new(ldd_path);
            cmd.arg("--version");
            cmd
        } else if let Some(uname_path) = find_command_on_path("uname") {
            Command::new(uname_path)
        } else {
            return false;
        };

        if let Ok(result) = command.output() {
            let output = if result.status.success() {
                String::from_utf8_lossy(&result.stdout).to_lowercase()
            } else {
                // ldd on apline returns stderr with a 1 exit code
                String::from_utf8_lossy(&result.stderr).to_lowercase()
            };

            return output.contains("musl") || output.contains("alpine");
        }

        false
    }

    /// Return true if the libc appears in a Rust target triple.
    pub fn appears_in_triple(&self) -> bool {
        matches!(self, Self::Gnu | Self::Musl) //  | Self::Msvc)
    }
}

impl fmt::Display for SystemLibc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            // Self::Bionic => "bionic",
            Self::Gnu => "gnu",
            // Self::LibSystem => "libsystem",
            Self::Musl => "musl",
            // Self::Msvc => "msvc",
            Self::Unknown => "unknown",
        })
    }
}

/// ABI (Application Binary Interface) of a target triple.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemABI {
    Eabi,
    Eabihf,
    Llvm,

    #[default]
    Unknown,
}

impl fmt::Display for SystemABI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Eabi => "eabi",
            Self::Eabihf => "eabihf",
            Self::Llvm => "llvm",
            Self::Unknown => "unknown",
        })
    }
}

/// A platform target, composed of an architecture, operating system, and libc.
/// Is formatted as `<arch>-<os>` or `<arch>-<os>-<libc>`, for example,
/// `arm64-macos`, `x64-linux-gnu`, or `arm64-linux-musl`.
///
/// When parsing, Rust target triples (`<arch>-<vendor>-<os>-<env>`) are also
/// supported, for example, `aarch64-apple-darwin`, `x86_64-unknown-linux-musl`,
/// or `aarch64-linux-android`. Each part supports the same aliases as their
/// respective types (`mac`, `x86_64`, `aarch64`, `glibc`, etc), and the Rust
/// variants of each (`i686`, `armv7`, `darwin`, `gnueabihf`, etc). If the libc
/// is not provided, it defaults to GNU for Linux, and unknown for everything else.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct SystemPlatform {
    pub os: SystemOS,
    pub arch: SystemArch,
    pub libc: SystemLibc,
}

impl SystemPlatform {
    /// Create a new platform with the default libc for the operating system.
    pub fn new(os: SystemOS, arch: SystemArch) -> Self {
        Self {
            os,
            arch,
            libc: Self::default_libc(os),
        }
    }

    /// Return an instance derived from the current system environment.
    /// The libc is detected, which requires executing a command on Linux.
    pub fn from_env() -> Self {
        let os = SystemOS::from_env();

        Self {
            os,
            arch: SystemArch::from_env(),
            libc: SystemLibc::detect(os),
        }
    }

    /// Return the libc to use for the operating system when one has not
    /// been explicitly provided.
    pub fn default_libc(os: SystemOS) -> SystemLibc {
        if os.is_linux() {
            SystemLibc::Gnu
        } else {
            SystemLibc::Unknown
        }
    }

    /// Parse a platform from a string, in the format of `<arch>-<os>`,
    /// `<arch>-<os>-<libc>`, or a Rust target triple.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, Error> {
        Self::from_str(value.as_ref())
    }

    /// Return a copy of this platform with the provided libc.
    pub fn with_libc(mut self, libc: SystemLibc) -> Self {
        self.libc = libc;
        self
    }
}

fn parse_enum<T: DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(Value::String(value.to_owned())).ok()
}

fn parse_platform_arch(value: &str) -> Option<SystemArch> {
    if let Some(arch) = parse_enum(value) {
        return Some(arch);
    }

    // Rust target triple architectures
    Some(match value {
        "i386" | "i586" | "i686" => SystemArch::X86,
        "x86_64h" => SystemArch::X64,
        "aarch64_be" | "arm64e" => SystemArch::Arm64,
        "mipsel" | "mipsisa32r6" | "mipsisa32r6el" => SystemArch::Mips,
        "mips64el" | "mipsisa64r6" | "mipsisa64r6el" => SystemArch::Mips64,
        "powerpc64le" => SystemArch::Powerpc64,
        "sparcv9" => SystemArch::Sparc64,
        arch if arch.starts_with("riscv64") => SystemArch::Riscv64,
        arch if arch.starts_with("armv")
            || arch.starts_with("armeb")
            || arch.starts_with("thumb") =>
        {
            SystemArch::Arm
        }
        _ => return None,
    })
}

fn parse_platform_os(value: &str) -> Option<SystemOS> {
    match value {
        // Rust target triple operating systems
        "darwin" => Some(SystemOS::MacOS),
        _ => parse_enum(value),
    }
}

/// Parse the trailing part, which is either a libc (`gnu`, `musl`, etc), or a
/// Rust target triple environment (`gnueabihf`, `musleabi`, `msvc`, etc).
fn parse_platform_libc(os: SystemOS, value: Option<&str>) -> Option<(SystemOS, SystemLibc)> {
    let Some(value) = value else {
        return Some((os, SystemPlatform::default_libc(os)));
    };

    if let Some(libc) = parse_enum(value) {
        return Some((os, libc));
    }

    Some(match value {
        env if env.starts_with("gnu") => (os, SystemLibc::Gnu),
        env if env.starts_with("musl") => (os, SystemLibc::Musl),
        // Android is a Linux target in Rust triples (`aarch64-linux-android`)
        env if os.is_linux() && env.starts_with("android") => {
            (SystemOS::Android, SystemLibc::Unknown)
        }
        "eabi" | "eabihf" | "macabi" | "msvc" | "sim" => (os, SystemLibc::Unknown),
        _ => return None,
    })
}

impl FromStr for SystemPlatform {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let platform = value.trim().to_lowercase();
        let parts = platform.split('-').collect::<Vec<_>>();

        if parts.len() < 2 || parts.len() > 4 || parts.iter().any(|part| part.is_empty()) {
            return Err(Error::InvalidPlatform(value.to_owned()));
        }

        let unknown_part = |kind: &str, part: &str| Error::UnknownPlatformPart {
            kind: kind.into(),
            value: part.to_owned(),
            platform: value.to_owned(),
        };

        let arch =
            parse_platform_arch(parts[0]).ok_or_else(|| unknown_part("architecture", parts[0]))?;

        // The operating system follows the architecture, or the vendor
        // when a Rust target triple (`x86_64-unknown-linux-gnu`)
        let Some((os_index, os)) = parts
            .iter()
            .enumerate()
            .skip(1)
            .take(2)
            .find_map(|(index, part)| parse_platform_os(part).map(|os| (index, os)))
        else {
            let is_vendor = matches!(parts[1], "apple" | "pc" | "sun" | "unknown");

            return Err(unknown_part(
                "operating system",
                parts
                    .get(if is_vendor { 2 } else { 1 })
                    .unwrap_or(&parts[1]),
            ));
        };

        let rest = &parts[os_index + 1..];

        if rest.len() > 1 {
            return Err(Error::InvalidPlatform(value.to_owned()));
        }

        let (os, libc) = parse_platform_libc(os, rest.first().copied())
            .ok_or_else(|| unknown_part("libc", rest[0]))?;

        Ok(Self { os, arch, libc })
    }
}

impl TryFrom<String> for SystemPlatform {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl From<SystemPlatform> for String {
    fn from(platform: SystemPlatform) -> Self {
        platform.to_string()
    }
}

impl fmt::Display for SystemPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.arch, self.os)?;

        if self.libc != SystemLibc::Unknown {
            write!(f, "-{}", self.libc)?;
        }

        Ok(())
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for SystemPlatform {
    fn schema_name() -> Option<String> {
        Some("SystemPlatform".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}
