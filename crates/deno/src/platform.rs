use proto_core::ProtoError;
use std::env::consts;
use std::fmt;

pub enum DenoArch {
    Arm64,
    X64,
}

impl DenoArch {
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub fn from_os_arch() -> Result<DenoArch, ProtoError> {
        // from rust archs
        match consts::ARCH {
            "aarch64" => Ok(DenoArch::Arm64),
            "x86_64" => Ok(DenoArch::X64),
            unknown => Err(ProtoError::UnsupportedArchitecture(
                "Deno".into(),
                unknown.to_owned(),
            )),
        }
    }
}

impl fmt::Display for DenoArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DenoArch::Arm64 => "aarch64",
                DenoArch::X64 => "x86_64",
            }
        )
    }
}
