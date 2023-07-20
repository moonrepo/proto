use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Empty {}

// GITHUB

#[derive(Deserialize)]
pub struct GitHubApiAsset {
    pub browser_download_url: String,
    pub content_type: String,
    pub name: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct GitHubApiRelease {
    pub assets: Vec<GitHubApiAsset>,
}

// WAPM

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageVersionDistribution {
    pub download_url: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageVersionModule {
    pub name: String,
    pub public_url: String,
    pub source: String,
    pub abi: String,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageVersion {
    pub distribution: WapmPackageVersionDistribution,
    pub modules: Vec<WapmPackageVersionModule>,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageResponseData {
    pub package_version: WapmPackageVersion,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageResponse {
    pub data: WapmPackageResponseData,
}

pub const WAPM_GQL_QUERY: &str = r#"
query PackageQuery($name: String!  $version: String = "latest") {
    packageVersion: getPackageVersion(name: $name, version: $version) {
        distribution {
            downloadUrl
        }
        modules {
            name
            publicUrl
            source
            abi
        }
    }
}"#;

#[derive(Default, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageRequestVariables {
    pub name: String,
    pub owner: String,
    pub version: String,
}

#[derive(Default, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WapmPackageRequest {
    pub query: String,
    pub variables: WapmPackageRequestVariables,
}
