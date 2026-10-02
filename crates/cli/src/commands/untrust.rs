use super::trust::{TrustTarget, resolve_target};
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use proto_core::TrustSource;
use starbase_console::ui::*;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct UntrustArgs {
    #[arg(
        help = "Config file, or directory of config files, to untrust. Defaults to the current directory"
    )]
    path: Option<PathBuf>,
}

#[instrument(skip(session))]
pub async fn untrust(session: ProtoSession, args: UntrustArgs) -> SessionResult {
    // Allow untrusting paths that no longer exist, so records can be cleaned up
    let Some(target) = resolve_target(&session, args.path.as_deref(), false)? else {
        return Ok(Some(1));
    };

    let path = target.path();

    if session.env.trust.untrust(path)? {
        session.console.notice(
            Variant::Success,
            match &target {
                TrustTarget::Dir(_) => format!(
                    "Untrusted directory <path>{}</path>. The security-sensitive settings of configs within it will no longer be applied.",
                    path.display()
                ),
                TrustTarget::File(_) => format!(
                    "Untrusted config <path>{}</path>. Its security-sensitive settings will no longer be applied.",
                    path.display()
                ),
            },
        )?;
    } else {
        session.console.notice(
            Variant::Info,
            format!(
                "{} <path>{}</path> was not trusted!",
                match target {
                    TrustTarget::Dir(_) => "Directory",
                    TrustTarget::File(_) => "Config",
                },
                path.display()
            ),
        )?;
    }

    // The configs may still be trusted through another source
    let subject = match target {
        TrustTarget::Dir(_) => "Configs within it are",
        TrustTarget::File(_) => "The config is",
    };
    let mut reported = vec![];

    for file in target.config_files()? {
        let Some(source) = session.env.trust.get_trust_source(&file) else {
            continue;
        };

        let message = match source {
            TrustSource::Ci => {
                format!("{subject} still trusted, as all configs are trusted in CI")
            }
            TrustSource::TrustedPath(path) => format!(
                "{subject} still trusted, as within <path>{}</path> from <property>PROTO_TRUSTED_PATHS</property>",
                path.display()
            ),
            TrustSource::Record(path) => format!(
                "{subject} still trusted, as <path>{}</path> is trusted. Untrust it with <shell>proto untrust {}</shell>",
                path.display(),
                path.display()
            ),
        };

        if !reported.contains(&message) {
            session.console.notice(Variant::Caution, &message)?;
            reported.push(message);
        }
    }

    Ok(None)
}
