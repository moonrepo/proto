# Plugin registry

proto's registry is currently powered by static JSON files located in the [./data](./data) directory.

In the future, we plan to build and host an actual server based registry, in which all plugin artifacts are stored. Until then, this works quite well.

## Publishing a plugin

Since we don't have a database, to make a plugin available to the community (via `proto plugin search`), you'll need to modify the [./data/third-party.json](./data/third-party.json) JSON file.

Simply add an entry to the array with your relevant information. View this [TypeScript interface](https://github.com/moonrepo/moon/blob/master/website/src/data/proto-tools.tsx) for available fields, and which are required.

```json
{
  "plugins": [
    // ...
    {
      "id": "my-new-tool"
      // ...
    }
  ]
}
```

Once your entry has been added, run `just gen` (or `cargo run -p proto_codegen`) to validate and format the dataset.

```
$ just gen
```

Then commit your changes and create a pull request. Once merged, your plugin will start showing up in search results!
