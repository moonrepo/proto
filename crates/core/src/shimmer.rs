use crate::errors::ProtoError;
use crate::helpers::get_bin_dir;
use serde::Serialize;
use serde_json::Value;
use starbase_utils::fs;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use tinytemplate::error::Error as TemplateError;
use tinytemplate::TinyTemplate;
use tracing::debug;

pub const SHIM_VERSION: u8 = 4;

#[derive(Default, Serialize)]
pub struct ShimContext<'tool> {
    // BINARY INFO
    /// Name of the binary to execute.
    pub bin: &'tool str,
    /// Path to the binary to execute.
    pub bin_path: Option<&'tool Path>,
    /// Name or relative path to an alternative binary file to execute.
    pub alt_bin: Option<&'tool str>,
    /// Name of a parent binary required to execute the current binary.
    pub parent_bin: Option<&'tool str>,
    /// Args to prepend to user-provided args.
    pub before_args: Option<&'tool str>,
    /// Args to append to user-provided args.
    pub after_args: Option<&'tool str>,

    // TOOL INFO
    /// Path to the proto tool installation directory.
    pub tool_dir: Option<&'tool Path>,
    pub tool_version: Option<&'tool str>,
}

impl<'tool> ShimContext<'tool> {
    pub fn new_global(bin: &'tool str) -> Self {
        ShimContext {
            bin,
            ..ShimContext::default()
        }
    }

    pub fn new_global_alt(parent_bin: &'tool str, bin: &'tool str, alt_bin: &'tool str) -> Self {
        ShimContext {
            bin,
            parent_bin: Some(parent_bin),
            alt_bin: Some(alt_bin),
            ..ShimContext::default()
        }
    }

    pub fn new_local(bin: &'tool str, bin_path: &'tool Path) -> Self {
        ShimContext {
            bin,
            bin_path: Some(bin_path),
            ..ShimContext::default()
        }
    }
}

impl<'tool> AsRef<ShimContext<'tool>> for ShimContext<'tool> {
    fn as_ref(&self) -> &ShimContext<'tool> {
        self
    }
}

#[async_trait::async_trait]
pub trait Shimable<'tool>: Send + Sync {
    /// Create one or many shims in the root of the tool's install directory.
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError>;

    /// Return an absolute path to the shim file if utilizing shims.
    fn get_shim_path(&self) -> Option<&Path> {
        None
    }
}

fn format_uppercase(value: &Value, output: &mut String) -> Result<(), TemplateError> {
    if let Value::String(string) = value {
        write!(output, "{}", string.to_uppercase())?;
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

#[tracing::instrument(skip_all)]
fn build_shim_file(context: &ShimContext, global: bool) -> Result<String, ProtoError> {
    let mut template = TinyTemplate::new();

    template.add_formatter("uppercase", format_uppercase);

    let contents = format!(
        "{}\n\n{}",
        get_template_header(global),
        get_template(global)
    );

    template
        .add_template("shim", &contents)
        .map_err(ProtoError::Shim)?;

    template.render("shim", context).map_err(ProtoError::Shim)
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
) -> Result<PathBuf, ProtoError> {
    if find_only && shim_path.exists() {
        return Ok(shim_path);
    }

    fs::write_file(&shim_path, build_shim_file(context, global)?)?;
    fs::update_perms(&shim_path, None)?;

    Ok(shim_path)
}

pub fn create_global_shim<'tool, C: AsRef<ShimContext<'tool>>>(
    context: C,
) -> Result<PathBuf, ProtoError> {
    create_global_shim_with_name(context.as_ref(), context.as_ref().bin)
}

#[tracing::instrument(name = "create_global_shim", skip_all)]
pub fn create_global_shim_with_name<'tool, C: AsRef<ShimContext<'tool>>>(
    context: C,
    name: &str,
) -> Result<PathBuf, ProtoError> {
    let context = context.as_ref();
    let shim_path = get_bin_dir()?.join(get_shim_file_name(name, true));

    debug!(tool = context.bin, file = ?shim_path, "Creating global shim");

    create_shim(context, shim_path, true, false)
}

#[tracing::instrument(skip_all)]
pub fn create_local_shim<'tool, C: AsRef<ShimContext<'tool>>>(
    context: C,
    find_only: bool,
) -> Result<PathBuf, ProtoError> {
    let context = context.as_ref();
    let shim_path = context
        .tool_dir
        .as_ref()
        .unwrap()
        .join("shims")
        .join(get_shim_file_name(context.bin, false));

    debug!(tool = context.bin, file = ?shim_path, "Creating local shim");

    create_shim(context, shim_path, false, find_only)
}
