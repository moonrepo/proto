use crate::error::ProtoError;
use serde::Serialize;
use serde_json::Value;
use starbase_utils::fs;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use tinytemplate::error::Error as TemplateError;
use tinytemplate::TinyTemplate;
use tracing::debug;

pub const SHIM_VERSION: u8 = 8;

#[derive(Debug, Default, Serialize)]
pub struct ShimContext<'tool> {
    // BINARY INFO
    /// Name of the binary to execute. Will be used for `proto run` in the shim.
    pub bin: &'tool str,

    /// Alternate path to the binary to execute. Uses `proto run --bin`.
    pub bin_path: Option<PathBuf>,

    /// Args to prepend to user-provided args.
    pub before_args: Option<&'tool str>,

    /// Args to append to user-provided args.
    pub after_args: Option<&'tool str>,

    // TOOL INFO
    /// ID of the tool, for logging purposes.
    pub tool_id: &'tool str,

    /// Path to the proto tool installation directory.
    pub tool_dir: Option<PathBuf>,

    /// Current resolved version.
    pub tool_version: Option<String>,
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

        fs::write_file(shim_path, build_shim_file(self, shim_path, true)?)?;
        fs::update_perms(shim_path, None)?;

        Ok(())
    }
}

impl<'tool> AsRef<ShimContext<'tool>> for ShimContext<'tool> {
    fn as_ref(&self) -> &ShimContext<'tool> {
        self
    }
}

fn format_uppercase(value: &Value, output: &mut String) -> Result<(), TemplateError> {
    if let Value::String(string) = value {
        write!(output, "{}", string.to_uppercase().replace('-', "_"))?;
    }

    Ok(())
}

fn get_template_header<'l>(global: bool) -> &'l str {
    if cfg!(windows) {
        if global {
            include_str!("../templates/cmd_header.tpl")
        } else {
            include_str!("../templates/pwsh_header.tpl")
        }
    } else {
        include_str!("../templates/bash_header.tpl")
    }
}

fn get_template<'l>(global: bool) -> &'l str {
    if cfg!(windows) {
        if global {
            include_str!("../templates/cmd_global.tpl")
        } else {
            include_str!("../templates/pwsh_local.tpl")
        }
    } else if global {
        include_str!("../templates/bash_global.tpl")
    } else {
        include_str!("../templates/bash_local.tpl")
    }
}

fn build_shim_file(
    context: &ShimContext,
    shim_path: &Path,
    global: bool,
) -> miette::Result<String> {
    let mut template = TinyTemplate::new();

    template.add_formatter("uppercase", format_uppercase);

    let contents = format!(
        "{}\n\n{}",
        get_template_header(global),
        get_template(global)
    );

    template
        .add_template("shim", &contents)
        .map_err(|error| ProtoError::Shim {
            path: shim_path.to_path_buf(),
            error,
        })?;

    let result = template
        .render("shim", context)
        .map_err(|error| ProtoError::Shim {
            path: shim_path.to_path_buf(),
            error,
        })?;

    Ok(result)
}

#[cfg(windows)]
pub fn get_shim_file_name(name: &str, global: bool) -> String {
    format!("{name}.{}", if global { "cmd" } else { "ps1" })
}

#[cfg(not(windows))]
pub fn get_shim_file_name(name: &str, _global: bool) -> String {
    name.to_owned()
}
