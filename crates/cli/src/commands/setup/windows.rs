use miette::{miette, IntoDiagnostic, Result};
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use tracing::debug;
use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::{RegKey, RegValue};

pub(crate) fn do_add_to_path(dirs: Vec<PathBuf>) -> Result<bool> {
    let current_path = get_path_var()?;

    let dirs: Vec<Vec<u16>> = dirs
        .iter()
        .map(|dir| OsString::from(dir).encode_wide().collect::<Vec<u16>>())
        .filter(|dir| !path_contains(&current_path, dir))
        .collect();

    let new_path = dirs
        .iter()
        .chain([&current_path])
        .fold(vec![], |acc, path| path_join(&acc, path));

    if current_path == new_path {
        debug!("System PATH already contains the new entries, leaving it untouched");
        Ok(false)
    } else {
        debug!("Updating system PATH");
        set_path_var(&new_path)?;
        Ok(true)
    }
}

fn get_path_var() -> Result<Vec<u16>> {
    use std::io::{Error, ErrorKind};

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ)
        .and_then(|env| env.get_raw_value("PATH"))
        .and_then(|value| {
            winreg_ext::from_winreg_value(&value).ok_or(Error::new(
                ErrorKind::InvalidData,
                "The registry key `HKEY_CURRENT_USER\\Environment\\PATH` is not a string",
            ))
        })
        .or_else(|err| {
            if err.kind() == ErrorKind::NotFound {
                Ok(vec![])
            } else {
                Err(err)
            }
        })
        .into_diagnostic()
}

fn set_path_var(new_path: &[u16]) -> Result<()> {
    if new_path.is_empty() {
        return Err(miette!(
            "New system path is empty, this shouldn't be possible!"
        ));
    }

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .and_then(|environment| {
            let reg_value = RegValue {
                bytes: winreg_ext::to_winreg_bytes(new_path),
                vtype: RegType::REG_EXPAND_SZ,
            };
            environment.set_raw_value("PATH", &reg_value)
        })
        .into_diagnostic()
}

/// Checks whether the windows path contains the given entry.
/// Note this does not guarantee any match isn't just a substring of a longer entry,
/// i.e. the entry "/proto" will match "/proto/bin".
fn path_contains(path: &[u16], entry: &[u16]) -> bool {
    path.windows(entry.len()).any(|path| path == entry)
}

fn path_join(left: &[u16], right: &[u16]) -> Vec<u16> {
    if left.is_empty() {
        right.to_owned()
    } else if right.is_empty() {
        left.to_owned()
    } else {
        let sep = b';' as u16;
        [left, right].join(&sep)
    }
}

// This is copied from:
// https://github.com/rust-lang/rustup/blob/a49059082a7b5cd2a59c1d8adf1b9a5a64402203/src/cli/self_update/windows.rs
//
// License:
//
// Copyright (c) 2016 The Rust Project Developers
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
mod winreg_ext {
    use winreg::enums::RegType;

    /// Convert a slice of UCS-2 chars to a null-terminated UCS-2 string in bytes
    pub fn to_winreg_bytes(val: &[u16]) -> Vec<u8> {
        let mut v = val.to_owned();
        v.push(0);
        unsafe { std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), v.len() * 2).to_vec() }
    }

    /// This is used to decode the value of HKCU\Environment\PATH. If that key is
    /// not REG_SZ | REG_EXPAND_SZ then this returns None. The winreg library itself
    /// does a lossy unicode conversion.
    pub fn from_winreg_value(val: &winreg::RegValue) -> Option<Vec<u16>> {
        use std::slice;

        match val.vtype {
            RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
                // Copied from winreg
                let mut words = unsafe {
                    #[allow(clippy::cast_ptr_alignment)]
                    slice::from_raw_parts(val.bytes.as_ptr().cast::<u16>(), val.bytes.len() / 2)
                        .to_owned()
                };
                while words.last() == Some(&0) {
                    words.pop();
                }
                Some(words)
            }
            _ => None,
        }
    }
}
