use proto_core::ProtoError;
use std::env::consts;
use std::fmt;

// Not everything is supported at the moment...
pub enum RustArch {
    Amd64,  // x86_64
    Arm64,  // Arm64
    Armv6l, // Arm V6
    I386,
    S390x,
}

impl RustArch {
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub fn from_os_arch() -> Result<RustArch, ProtoError> {
        // from rust archs
        match consts::ARCH {
            "arm" => Ok(RustArch::Armv6l),
            "aarch64" => Ok(RustArch::Arm64),
            "s390x" => Ok(RustArch::S390x),
            "x86_64" => Ok(RustArch::Amd64),
            "x86" => Ok(RustArch::I386),
            unknown => Err(ProtoError::UnsupportedArchitecture(
                "Go".into(),
                unknown.to_owned(),
            )),
        }
    }
}

impl fmt::Display for RustArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RustArch::Amd64 => "amd64",
                RustArch::Arm64 => "arm64",
                RustArch::Armv6l => "armv6l",
                RustArch::S390x => "s390x",
                RustArch::I386 => "386",
            }
        )
    }
}
