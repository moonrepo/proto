use crate::error::ProtoError;
use serde::Serialize;
use starbase_utils::fs;
use std::path::Path;
use tera::{Tera, Context};
use tracing::debug;

pub const SHIM_VERSION: u8 = 10;

#[derive(Debug, Default, Serialize)]
pub struct ShimContext<'tool> {
    // BINARY INFO
    /// Name of the binary to execute. Will be used for `proto run` in the shim.
    pub bin: &'tool str,

    /// Name of the alternate binary to execute. Uses `proto run --alt`.
    pub alt_bin: Option<&'tool str>,

    /// Args to prepend to user-provided args.
    pub before_args: Option<&'tool str>,

    /// Args to append to user-provided args.
    pub after_args: Option<&'tool str>,

    // TOOL INFO
    /// ID of the tool, for logging purposes.
    pub tool_id: &'tool str,
}

impl<'tool> ShimContext<'tool> {
    pub fn create_shim(&self, shim_path: &Path, find_only: bool) -> miette::Result<()> {
        if find_only && shim_path.exists() {
            return Ok(());
        }

        debug!(
            tool = &self.tool_id,
            shim = ?shim_path,
             "Creating global shim"
        );

        fs::write_file(shim_path, build_shim_file(self, shim_path)?)?;
        fs::update_perms(shim_path, None)?;

        Ok(())
    }
}

impl<'tool> AsRef<ShimContext<'tool>> for ShimContext<'tool> {
    fn as_ref(&self) -> &ShimContext<'tool> {
        self
    }
}

#[cfg(windows)]
fn get_template<'l>(shim_path: &Path) -> &'l str {
    match shim_path.extension().map(|ext| ext.to_str().unwrap()) {
        Some("cmd") => include_str!("../templates/windows/cmd.tpl"),
        Some("ps1") => include_str!("../templates/windows/ps1.tpl"),
        _ => include_str!("../templates/windows/sh.tpl"),
    }
}

#[cfg(not(windows))]
fn get_template<'l>(_shim_path: &Path) -> &'l str {
    include_str!("../templates/unix/sh.tpl")
}

fn build_shim_file(context: &ShimContext, shim_path: &Path) -> miette::Result<String> {
    let handle_error = |error: tera::Error| ProtoError::Shim {
        path: shim_path.to_path_buf(),
        error,
    };

    let mut tera = Tera::default();

    if cfg!(windows) {
        tera.add_raw_template("macros.tpl", include_str!("../templates/windows/macros.tpl")).map_err(handle_error)?;
    }

    tera.add_raw_template("shim.tpl", get_template(shim_path)).map_err(handle_error)?;

    let result = tera.render("shim.tpl", &Context::from_serialize(&context).map_err(handle_error)?).map_err(handle_error)?;

    Ok(result)
}

#[cfg(windows)]
pub fn get_shim_file_names(name: &str) -> Vec<String> {
    // Order is important!
    vec![
        format!("{name}.ps1"),
        format!("{name}.cmd"),
        format!("{name}"),
    ]
}

#[cfg(not(windows))]
pub fn get_shim_file_names(name: &str) -> Vec<String> {
    vec![name.to_owned()]
}
