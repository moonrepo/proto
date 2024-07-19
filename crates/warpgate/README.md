# warpgate

![Crates.io](https://img.shields.io/crates/v/warpgate) ![Crates.io](https://img.shields.io/crates/d/warpgate)

Warpgate is a library for downloading, resolving, and managing [Extism][extism] powered WASM plugins.

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

let wasm_file = loader.load_plugin(PluginLocator::Url {
	url: "https://registry.com/path/to/file.wasm".into(),
});
```

### Locator strategies

A locator strategy defines instructions on where to locate the necessary `.wasm` file, and is represented as variants on the `PluginLocator` enum.

This enum also supports deserializing from strings (as shown below in comments), which can be useful for configuration files.

The following strategies are currently supported:

#### Local files

File is available on the local host machine. When deserialized, the `path` field must be converted to an absolute path manually.

```rust
// file://./path/to/file.wasm
PluginLocator::File {
	file: "path/to/file.wasm".into(),
	path: Some(PathBuf::from("/absolute/path/to/file.wasm")),
}
```

#### Secure URLs

Download a file from a secure `https` URL.

```rust
// https://registry.com/path/to/file.wasm
PluginLocator::Url {
	url: "https://registry.com/path/to/file.wasm".into(),
}
```

#### GitHub releases

Download an asset from a GitHub release. This approach communicates with the GitHub API, and requires a `.wasm` file to be attached as an asset.

Defining a `GITHUB_TOKEN` environment variable is recommended to avoid rate limiting.

```rust
// github://org/repo
// github://org/repo@v1.2.3
// github://org/repo/project
PluginLocator::GitHub(GitHubLocator{
	repo_slug: "org/repo".into(),
	project_name: None,
	tag: Some("v1.2.3".into()), // Latest if `None`
})
```

> The `project_name` field exists to support monorepos. When defined, it will look for a tag/release that starts with the project name. For example, if the project name was `example_plugin`, it will match `example_plugin-v1.2.3` or `example_plugin@v1.2.3` tags.

## Extism plugin containers

Another mechanism of this library is providing the `PluginContainer` struct; a wrapper around [Extism][extism]'s `Plugin` and `Manifest` types. The container provides convenience methods for calling functions with serde compatible input and output types, _and_ caching the result for subsequent calls. This is extremely useful in avoiding unnecessary overhead when communicating between the WASM guest and host.

To make use of the container, instantiate an instance with a `Manifest`, and optional host functions.

```rust
use warpgate::{Id, PluginContainer, PluginManifest, Wasm};

// Load the plugin and create a manifest
let wasm_file = loader.load_plugin(locator);
let manifest = PluginManifest::new([Wasm::file(wasm_file)]);

// Create a container
let container = PluginContainer::new(Id::new("id")?, manifest, [host, funcs])?;
// Or
let container = PluginContainer::new_without_functions(Id::new("id")?, manifest)?;
```

From here, you can call functions on the plugin with the `call_func` (no input) and `call_func_with` methods. To call _and_ cache functions, use the alternative `cache_func` and `cache_func_with` methods.

Furthermore, these methods require a serde struct for outputs, and optionally for inputs. Non-serde based functions can be handled with the `call` method.

```rust
let output: AddOutput = container.cache_func_with("add", AddInput {
	left: 10,
	right: 20,
})?;

dbg!(output.sum);
```

[extism]: https://extism.org/
