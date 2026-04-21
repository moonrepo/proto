use crate::registry_error::WarpgateRegistryError;
use cargo_toml::{Inheritable, Manifest};
use std::collections::BTreeMap;
use std::path::Path;
use warpgate::{Id, PluginContainer, PluginManifest, Wasm};

pub struct PublishOptions {
    pub plugin_type: String,
    pub required_funcs: Vec<String>,
    pub runtime: String,
}

pub async fn verify_valid_wasm_plugin(
    id: &Id,
    wasm_path: &Path,
    options: &PublishOptions,
) -> Result<(), WarpgateRegistryError> {
    let plugin = PluginContainer::new_without_functions(
        id.to_owned(),
        PluginManifest::new([Wasm::file(wasm_path)]),
    )?;

    for func in &options.required_funcs {
        if !plugin.has_func(func).await {}
    }

    Ok(())
}

// https://github.com/opencontainers/image-spec/blob/main/annotations.md
// Set by GitHub when publishing:
// - org.opencontainers.image.created
// - org.opencontainers.image.vendor (org or user)
pub async fn gather_annotations(
    manifest: &Manifest,
    options: &PublishOptions,
) -> BTreeMap<String, String> {
    let mut annotations = BTreeMap::new();
    annotations.insert("moonrepo.runtime".into(), options.runtime.clone());
    annotations.insert("moonrepo.plugin.format".into(), "wasm".into());
    annotations.insert("moonrepo.plugin.type".into(), options.plugin_type.clone());

    if let Some(package) = &manifest.package {
        annotations.insert(
            "org.opencontainers.image.title".into(),
            package.name.to_lowercase().replace('-', "_").into(),
        );

        if let Some(Inheritable::Set(description)) = &package.description {
            annotations.insert(
                "org.opencontainers.image.description".into(),
                description.into(),
            );
        }

        if let Inheritable::Set(version) = &package.version {
            annotations.insert("org.opencontainers.image.version".into(), version.into());
        } else {
            todo!("REQUIRES VERSION");
        }

        if let Some(Inheritable::Set(license)) = &package.license {
            annotations.insert("org.opencontainers.image.licenses".into(), license.into());
        }

        if let Some(Inheritable::Set(repository)) = &package.repository {
            annotations.insert("org.opencontainers.image.source".into(), repository.into());
        } else {
            todo!("REQUIRES SOURCE");
        }

        if let Some(Inheritable::Set(documentation)) = &package.documentation {
            annotations.insert(
                "org.opencontainers.image.documentation".into(),
                documentation.into(),
            );
        }

        if let Some(Inheritable::Set(homepage)) = &package.homepage {
            annotations.insert("org.opencontainers.image.url".into(), homepage.into());
        }

        if let Inheritable::Set(authors) = &package.authors {
            annotations.insert(
                "org.opencontainers.image.authors".into(),
                authors.join(", "),
            );
        }
    } else {
        todo!();
    }

    annotations
}
