use crate::errors::ProtoError;
use crate::helpers::{get_bin_dir, get_root};
use serde::Serialize;
use serde_json::Value;
use starbase_utils::fs;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use tinytemplate::error::Error as TemplateError;
use tinytemplate::TinyTemplate;
use tracing::debug;

pub const SHIM_VERSION: u8 = 3;

#[derive(Default, Serialize)]
pub struct ShimContext {
    // BINARY INFO
    /// Name of the binary to execute.
    bin: String,
    /// Path to the binary to execute.
    bin_path: PathBuf,
    /// Name or relative path to an alternative binary file to execute.
    alt_bin: Option<String>,
    /// Name of a parent binary required to execute the current binary.
    parent_bin: Option<String>,

    // TOOL INFO
    /// Path to the proto tool installation directory.
    tool_dir: Option<PathBuf>,
    tool_version: Option<String>,
}

#[derive(Default, Serialize)]
pub struct Context {
    alt_bin: Option<String>,
    bin_path: PathBuf,
    install_dir: Option<PathBuf>,
    name: String,
    parent_name: Option<String>,
    root: PathBuf,
    version: Option<String>,
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
fn build_shim_file(
    builder: &ShimBuilder,
    contents: &str,
    global: bool,
) -> Result<String, ProtoError> {
    let mut template = TinyTemplate::new();

    template.add_formatter("uppercase", format_uppercase);

    let contents = format!("{}\n\n{}", get_template_header(global), contents);

    template
        .add_template("shim", &contents)
        .map_err(ProtoError::Shim)?;

    template
        .render("shim", &builder.context)
        .map_err(ProtoError::Shim)
}

#[cfg(windows)]
fn get_shim_file_name(name: &str, global: bool) -> String {
    format!("{name}.{}", if global { "cmd" } else { "ps1" })
}

#[cfg(not(windows))]
fn get_shim_file_name(name: &str, _global: bool) -> String {
    name.to_owned()
}

pub struct ShimBuilder {
    pub context: Context,
    pub global_template: Option<String>,
    pub local_template: Option<String>,
}

impl ShimBuilder {
    pub fn new(name: &str, bin_path: &Path) -> Result<Self, ProtoError> {
        Ok(ShimBuilder {
            context: Context {
                bin_path: bin_path.to_owned(),
                name: name.to_owned(),
                root: get_root()?,
                ..Context::default()
            },
            global_template: None,
            local_template: None,
        })
    }

    pub fn alt_bin<V: AsRef<str>>(&mut self, bin: V) -> &mut Self {
        self.context.alt_bin = Some(bin.as_ref().to_owned());
        self
    }

    pub fn dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.context.install_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn parent<V: AsRef<str>>(&mut self, name: V) -> &mut Self {
        self.context.parent_name = Some(name.as_ref().to_owned());
        self
    }

    pub fn version<V: AsRef<str>>(&mut self, version: V) -> &mut Self {
        self.context.version = Some(version.as_ref().to_owned());
        self
    }

    pub fn set_global_template(&mut self, template: String) -> &mut Self {
        self.global_template = Some(template);
        self
    }

    pub fn set_tool_template(&mut self, template: String) -> &mut Self {
        self.local_template = Some(template);
        self
    }

    pub fn create_global_shim(&self) -> Result<PathBuf, ProtoError> {
        let shim_path = get_bin_dir()?.join(get_shim_file_name(&self.context.name, true));

        self.do_create(
            shim_path,
            if let Some(template) = &self.global_template {
                template
            } else {
                get_template(true)
            },
            true,
            false,
        )
    }

    pub fn create_tool_shim(&self, find_only: bool) -> Result<PathBuf, ProtoError> {
        let shim_path = self
            .context
            .install_dir
            .as_ref()
            .unwrap()
            .join("shims")
            .join(get_shim_file_name(&self.context.name, false));

        self.do_create(
            shim_path,
            if let Some(template) = &self.local_template {
                template
            } else {
                get_template(false)
            },
            false,
            find_only,
        )
    }

    fn do_create(
        &self,
        shim_path: PathBuf,
        contents: &str,
        global: bool,
        find_only: bool,
    ) -> Result<PathBuf, ProtoError> {
        let shim_exists = shim_path.exists();

        if find_only && shim_exists {
            return Ok(shim_path);
        }

        if let Some(parent) = shim_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write_file(&shim_path, build_shim_file(self, contents, global)?)?;
        fs::update_perms(&shim_path, None)?;

        debug!(tool = &self.context.name, file = ?shim_path, "Created shim");

        Ok(shim_path)
    }
}
