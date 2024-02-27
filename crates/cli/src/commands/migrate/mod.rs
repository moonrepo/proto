#![allow(unreachable_code)]

use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use clap::Args;
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct MigrateArgs {
    #[arg(required = true, help = "Operation to migrate")]
    operation: String,
}

#[system]
pub async fn migrate(args: ArgsRef<MigrateArgs>, _proto: ResourceRef<ProtoResource>) {
    // match args.operation.as_str() {
    //     unknown => {
    //         return Err(ProtoCliError::UnknownMigration {
    //             op: unknown.to_owned(),
    //         }
    //         .into());
    //     }
    // }

    return Err(ProtoCliError::UnknownMigration {
        op: args.operation.to_owned(),
    }
    .into());
}
