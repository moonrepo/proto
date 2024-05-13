# @moonrepo/proto-pdk

A plugin development kit for creating proto WASM plugins using JS/TS.

> [!IMPORTANT]  
> The Extism JavaScript PDK does not currently provide filesystem APIs, hence
> some functionality, such as custom checksumming, is difficult to achieve using
> this PDK.
>
> If you need this you're better off using the [Rust PDK](../crates/pdk) for now.

## Prerequisites

You will need:

- [binaryen >=v117](https://github.com/WebAssembly/binaryen)
- [extism-js >=v1.0.0-rc9](https://github.com/extism/js-pdk)

## Installation

Not yet published, please check back later.

<!--
```shell
npm i @moonrepo/proto-pdk
```
-->

## Usage

[The test plugin](./test) is currently the best reference for a complete TypeScript-based example,
until we get the PDK properly documented.

Neither TypeScript or ESBuild is a requirement. A bundler (like ESBuild) is however, and the bundled output
should target at ES2020 or below, while using CommonJS as the format.

It is strongly recommended to minify your bundled JavaScript, as it greatly affects the final WASM file size.
