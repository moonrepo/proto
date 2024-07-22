use crate::session::ProtoSession;
use clap::Args;
use indexmap::IndexMap;
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id};
use serde::Serialize;
use starbase::AppResult;
use starbase_shell::{BoxedShell, Hook, ShellType, Statement};
use starbase_utils::json;
use std::env;
use std::path::{Path, PathBuf};
use tokio::task::{self, JoinHandle, JoinSet};
use tracing::error;

#[derive(Default, Serialize)]
struct ActivateItem {
    pub id: Id,
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<PathBuf>,
}

impl ActivateItem {
    pub fn add_path(&mut self, path: &Path) {
        // Only add paths that exist and are normalized
        if let Ok(path) = path.canonicalize() {
            self.paths.push(path);
        }
    }
}

#[derive(Default, Serialize)]
struct ActivateInfo {
    pub env: IndexMap<String, Option<String>>,
    pub paths: Vec<PathBuf>,
    pub tools: Vec<ActivateItem>,
}

impl ActivateInfo {
    pub fn collect(&mut self, item: ActivateItem) {
        // Don't use a set as we need to persist the order!
        for path in &item.paths {
            if !self.paths.contains(path) {
                self.paths.push(path.to_owned());
            }
        }

        self.env.extend(item.env.clone());
        self.tools.push(item);
    }

    pub fn export(self, shell: &BoxedShell) -> String {
        let mut output = vec![];

        for (key, value) in &self.env {
            output.push(shell.format_env(key, value.as_deref()));
        }

        let paths = self
            .paths
            .iter()
            .filter_map(|path| path.to_str().map(|p| p.to_owned()))
            .collect::<Vec<_>>();

        if !paths.is_empty() {
            output.push(shell.format(Statement::PrependPath {
                paths: &paths,
                key: Some("PATH"),
                orig_key: if env::var("__ORIG_PATH").is_ok() {
                    Some("__ORIG_PATH")
                } else {
                    None
                },
            }));
        }

        output.join("\n")
    }
}

#[derive(Args, Clone, Debug)]
pub struct ActivateArgs {
    #[arg(help = "Shell to activate for")]
    shell: Option<ShellType>,

    #[arg(
        long,
        help = "Print the activate instructions in shell specific-syntax"
    )]
    export: bool,

    #[arg(long, help = "Print the activate instructions in JSON format")]
    json: bool,

    #[arg(long, help = "Don't include ~/.proto/bin in path lookup")]
    no_bin: bool,

    #[arg(long, help = "Don't include ~/.proto/shims in path lookup")]
    no_shim: bool,
}

#[tracing::instrument(skip_all)]
pub async fn activate(session: ProtoSession, args: ActivateArgs) -> AppResult {
    // Detect the shell that we need to activate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Pre-load configuration
    session.env.load_config()?;

    // Load all the tools so that we can extract info
    let tools = session.load_tools().await?;
    // let mut futures: Vec<JoinHandle<miette::Result<ActivateItem>>> = vec![];
    let mut set = JoinSet::<miette::Result<ActivateItem>>::new();

    for mut tool in tools {
        // futures.push(task::spawn(
        set.spawn(async move {
            error!("{} - 1", &tool.id);

            let mut item = ActivateItem::default();

            // Detect a version, otherwise return early
            let Ok(version) = detect_version(&tool, None).await else {
                return Ok(item);
            };
            error!("{} - 2", &tool.id);

            // Resolve the version and locate executables
            if tool.is_setup(&version).await? {
                error!("{} - 3", &tool.id);
                tool.locate_exes_dir().await?;
                error!("{} - 4", &tool.id);
                tool.locate_globals_dirs().await?;
                error!("{} - 5", &tool.id);

                // Higher priority over globals
                if let Some(exe_dir) = tool.get_exes_dir() {
                    item.add_path(exe_dir);
                }

                for global_dir in tool.get_globals_dirs() {
                    item.add_path(global_dir);
                }
            }
            error!("{} - 6", &tool.id);

            // Inherit all environment variables for the config
            let config = tool.proto.load_config()?;
            error!("{} - 7", &tool.id);

            item.env.extend(config.get_env_vars(Some(&tool.id))?);
            item.id = tool.id;
            error!("{} - 8", &item.id);

            Ok(item)
        });
    }

    error!("before 1");
    // Aggregate our list of shell exports
    let mut info = ActivateInfo::default();

    // Put shims first so that they can detect newer versions
    if !args.no_shim {
        info.paths.push(session.env.store.shims_dir.clone());
    }

    error!("before 2");

    // for future in futures {
    //     error!("loop");
    //     let data = future.await.into_diagnostic()??;
    //     info.collect(data);
    // }

    while let Some(item) = set.join_next().await {
        error!("loop");
        info.collect(item.into_diagnostic()??);
    }

    error!("after");

    // Put bins last as a last resort lookup
    if !args.no_bin {
        info.paths.push(session.env.store.bin_dir.clone());
    }

    // Output/export the information for the chosen shell
    let shell = shell_type.build();

    if args.export {
        println!("{}", info.export(&shell));

        return Ok(());
    }

    if args.json {
        println!("{}", json::format(&info, true)?);

        return Ok(());
    }

    error!("format");

    match shell.format_hook(Hook::OnChangeDir {
        command: match shell_type {
            // These operate on JSON
            ShellType::Nu => format!("proto activate {} --json", shell_type),
            // While these evaluate shell syntax
            _ => format!("proto activate {} --export", shell_type),
        },
        prefix: "proto".into(),
    }) {
        Ok(output) => {
            println!("{output}");
        }
        Err(_) => {
            // Do nothing? This command is typically wrapped in `eval`,
            // so these warnings would actually just trigger a syntax error.

            // warn!(
            //     "Failed to run {}. Perhaps remove it for the time being?",
            //     color::shell("proto activate")
            // );
            // warn!("Reason: {}", color::muted_light(error.to_string()));
        }
    };

    error!("exit");

    Ok(())
}
