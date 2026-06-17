#![allow(unreachable_code)]

use crate::error::ProtoCliError;
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct MigrateArgs {
    #[arg(required = true, help = "Operation to migrate")]
    operation: String,
}

#[instrument(skip(_session))]
pub async fn migrate(_session: ProtoSession, args: MigrateArgs) -> SessionResult {
    // match args.operation.as_str() {
    //     unknown => {
    //         return Err(ProtoCliError::UnknownMigration {
    //             op: unknown.to_owned(),
    //         }
    //         .into());
    //     }
    // }

    return Err(ProtoCliError::MigrateUnknownOperation {
        op: args.operation.to_owned(),
    }
    .into());
}
