#![allow(unreachable_code)]

use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use starbase::AppResult;

#[derive(Args, Clone, Debug)]
pub struct MigrateArgs {
    #[arg(required = true, help = "Operation to migrate")]
    operation: String,
}

#[tracing::instrument(skip_all)]
pub async fn migrate(_session: ProtoSession, args: MigrateArgs) -> AppResult {
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
