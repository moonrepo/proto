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

pub const SHIM_VERSION: u8 = 1;

#[derive(Serialize)]
pub struct Context {
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

fn get_template_header<'l>() -> &'l str {
    if cfg!(windows) {
        include_str!("../templates/pwsh_header.tpl")
    } else {
        include_str!("../templates/bash_header.tpl")
    }
}

fn get_template<'l>(global: bool) -> &'l str {
    if cfg!(windows) {
        if global {
            include_str!("../templates/pwsh_global.tpl")
        } else {
            include_str!("../templates/pwsh.tpl")
        }
    } else if global {
        include_str!("../templates/bash_global.tpl")
    } else {
        include_str!("../templates/bash.tpl")
    }
}

#[tracing::instrument(skip_all)]
fn build_shim_file(builder: &ShimBuilder, contents: &str) -> Result<String, ProtoError> {
    let mut template = TinyTemplate::new();

    template.add_formatter("uppercase", format_uppercase);

    let contents = format!("{}\n\n{}", get_template_header(), contents);

    template
        .add_template("shim", &contents)
        .map_err(ProtoError::Shim)?;

    template
        .render("shim", &builder.create_context()?)
        .map_err(ProtoError::Shim)
}

#[cfg(windows)]
fn get_shim_file_name(name: &str) -> String {
    format!("{name}.ps1")
}

#[cfg(not(windows))]
fn get_shim_file_name(name: &str) -> String {
    name.to_owned()
}

pub struct ShimBuilder {
    pub name: String,
    pub bin_path: PathBuf,
    pub install_dir: Option<PathBuf>,
    pub parent_name: Option<String>,
    pub version: Option<String>,
    pub global_template: Option<String>,
    pub local_template: Option<String>,
}

impl ShimBuilder {
    pub fn new(name: &str, bin_path: &Path) -> Self {
        ShimBuilder {
            name: name.to_owned(),
            bin_path: bin_path.to_path_buf(),
            install_dir: None,
            parent_name: None,
            version: None,
            global_template: None,
            local_template: None,
        }
    }

    pub fn dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.install_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn parent<V: AsRef<str>>(&mut self, name: V) -> &mut Self {
        self.parent_name = Some(name.as_ref().to_owned());
        self
    }

    pub fn version<V: AsRef<str>>(&mut self, version: V) -> &mut Self {
        self.version = Some(version.as_ref().to_owned());
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
        let shim_path = get_bin_dir()?.join(get_shim_file_name(&self.name));

        self.do_create(
            shim_path,
            if let Some(template) = &self.global_template {
                template
            } else {
                get_template(true)
            },
            false,
        )
    }

    pub fn create_tool_shim(&self, find_only: bool) -> Result<PathBuf, ProtoError> {
        let shim_path = self
            .install_dir
            .as_ref()
            .unwrap()
            .join("shims")
            .join(get_shim_file_name(&self.name));

        self.do_create(
            shim_path,
            if let Some(template) = &self.local_template {
                template
            } else {
                get_template(false)
            },
            find_only,
        )
    }

    pub fn create_context(&self) -> Result<Context, ProtoError> {
        Ok(Context {
            bin_path: self.bin_path.clone(),
            install_dir: self.install_dir.clone(),
            name: self.name.clone(),
            parent_name: self.parent_name.clone(),
            root: get_root()?,
            version: self.version.clone(),
        })
    }

    fn do_create(
        &self,
        shim_path: PathBuf,
        contents: &str,
        find_only: bool,
    ) -> Result<PathBuf, ProtoError> {
        let shim_exists = shim_path.exists();

        if find_only && shim_exists {
            return Ok(shim_path);
        }

        if let Some(parent) = shim_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write_file(&shim_path, build_shim_file(self, contents)?)?;
        fs::update_perms(&shim_path, None)?;

        debug!(file = %shim_path.display(), "Created shim");

        Ok(shim_path)
    }
}
