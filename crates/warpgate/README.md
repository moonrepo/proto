# warpgate

![Crates.io](https://img.shields.io/crates/v/warpgate) ![Crates.io](https://img.shields.io/crates/d/warpgate)

Warpgate is a library for downloading, loading, and managing [Extism](https://extism.org/) powered WASM plugins at runtime.

The warp in warpgate stands for Web Assembly Runtime Plugins. Pretty stellar huh.

## Loading plugins

Before a WASM file can be used, it must be loaded. Depending on the locator strategy used, loading a plugin can mean referencing a local file, downloading a file from a secure URL, or even making API requests to specific registries.

To begin, instantiate a `PluginLoader` and provide a directory path in which to cache `.wasm` files, and a temporary directory in which to download and unpack files.

```rust
use warpgate::PluginLoader;

let root = get_cache_root();
let loader = PluginLoader::new(root.join("plugins"), root.join("temp"));
```

Plugins can then be loaded with the `load_plugin` method, which requires a unique ID (becomes the file name), and a `PluginLocator` enum variant (in which to locate the `.wasm` file). This method returns an absolute path to the cached `.wasm` file on the host machine.

```rust
use warpgate::PluginLocator;

let wasm_file = loader.load_plugin(PluginLocator::SourceUrl {
	url: "https://registry.com/path/to/file.wasm".into(),
});
```

### Locator strategies

A locator strategy defines instructions on where to locate the necessary `.wasm` file, and is represented as variants on the `PluginLocator` enum.

This enum also supports deserializing from strings (as shown below in comments), which can be useful for configuration files.

The following strategies are currently supported:

#### Local files

File is available on the local host machine. When deserialized, the `path` field is resolved as-is to `file`, and must be converted to an absolute path beforehand.

```rust
// source:path/to/file.wasm
PluginLocator::SourceFile {
	file: "path/to/file.wasm".into(),
	path: PathBuf::from("/absolute/path/to/file.wasm"),
}
```

#### Secure URLs

Download a file from a secure `https` URL.

```rust
// source:https://registry.com/path/to/file.wasm
PluginLocator::SourceUrl {
	url: "https://registry.com/path/to/file.wasm".into(),
}
```

#### GitHub releases

Download an asset from a GitHub release. This approach communicates with the GitHub API, and requires a `.wasm` file to be attached as an asset.

```rust
// github:org/repo
// github:org/repo@v1.2.3
PluginLocator::GitHub(GitHubLocator{
	file_stem: "file_stem".into(),
	repo_slug: "org/repo".into(),
	tag: Some("v1.2.3".into()), // Latest if `None`
})
```

> The `file_stem` cannot be configured via the string format, and defaults to the repository name in snake_case, suffixed with `_plugin`.
