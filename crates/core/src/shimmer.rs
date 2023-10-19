use crate::error::ProtoError;
use crate::ProtoEnvironment;
use serde::Serialize;
use serde_json::Value;
use starbase_utils::fs;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use tinytemplate::error::Error as TemplateError;
use tinytemplate::TinyTemplate;
use tracing::debug;

pub const SHIM_VERSION: u8 = 7;

#[derive(Debug, Default, Serialize)]
pub struct ShimContext<'tool> {
    /// Name of the shim file.
    pub shim_file: &'tool str,

    // BINARY INFO
    /// Name of the binary to execute. Will be used for `proto run` in the shim.
    pub bin: &'tool str,

    /// Alternate path to the binary to execute.
    /// For global (optional), passes `--bin`. For local (required), executes the file.
    pub bin_path: Option<&'tool Path>,

    /// Name of a parent binary required to execute the current binary.
    pub parent_bin: Option<&'tool str>,

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

fn create_shim(
    context: &ShimContext,
    shim_path: PathBuf,
    global: bool,
    find_only: bool,
) -> miette::Result<PathBuf> {
    if find_only && shim_path.exists() {
        return Ok(shim_path);
    }

    fs::write_file(&shim_path, build_shim_file(context, &shim_path, global)?)?;
    fs::update_perms(&shim_path, None)?;

    Ok(shim_path)
}

pub fn create_global_shim<'tool, C: AsRef<ShimContext<'tool>>>(
    proto: &ProtoEnvironment,
    context: C,
    find_only: bool,
) -> miette::Result<PathBuf> {
    let context = context.as_ref();
    let shim_path = proto
        .shims_dir
        .join(get_shim_file_name(context.shim_file, true));

    if !find_only {
        debug!(tool = &context.tool_id, shim = ?shim_path, "Creating global shim");
    }

    create_shim(context, shim_path, true, find_only)
}

pub fn create_local_shim<'tool, C: AsRef<ShimContext<'tool>>>(
    context: C,
    find_only: bool,
) -> miette::Result<PathBuf> {
    let context = context.as_ref();
    let shim_path = context
        .tool_dir
        .as_ref()
        .expect("Missing tool directory for shims.")
        .join("shims")
        .join(get_shim_file_name(context.shim_file, false));

    if !find_only {
        debug!(tool = &context.tool_id, shim = ?shim_path, "Creating local shim");
    }

    create_shim(context, shim_path, false, find_only)
}
