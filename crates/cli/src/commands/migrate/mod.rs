mod v0_20;
mod v0_24;

use crate::error::ProtoCliError;
use clap::Args;
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct MigrateArgs {
    #[arg(required = true, help = "Operation to migrate")]
    operation: String,
}

#[system]
pub async fn migrate(args: ArgsRef<MigrateArgs>) {
    match args.operation.as_str() {
        "v0.20" => {
            v0_20::migrate().await?;
        }
        "v0.24" => {
            v0_24::migrate().await?;
        }
        unknown => {
            return Err(ProtoCliError::UnknownMigration {
                op: unknown.to_owned(),
            }
            .into());
        }
    }
}
