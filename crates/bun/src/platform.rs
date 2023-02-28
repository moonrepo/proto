use proto_core::ProtoError;
use std::env::consts;
use std::fmt;

pub enum BunArch {
    Arm64,
    X64,
}

impl BunArch {
    // https://doc.rust-lang.org/std/env/consts/constant.ARCH.html
    pub fn from_os_arch() -> Result<BunArch, ProtoError> {
        // from rust archs
        match consts::ARCH {
            "aarch64" => Ok(BunArch::Arm64),
            "x86_64" => Ok(BunArch::X64),
            unknown => Err(ProtoError::UnsupportedArchitecture(
                "Bun".into(),
                unknown.to_owned(),
            )),
        }
    }
}

impl fmt::Display for BunArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BunArch::Arm64 => "aarch64",
                BunArch::X64 => "x64",
            }
        )
    }
}
