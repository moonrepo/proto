# Results

When the shim file name is `node` (instead of `proto-shim`):

|                   | `args[0]` | `current_exe` | Ctrl+C | Piped data |
| ----------------- | --------- | ------------- | ------ | ---------- |
| Unix file         | `node`    | `node`        | Yes    | Yes        |
| Unix soft-link    | `node`    | `node`        | Yes    | Yes        |
| Unix hard-link    | `node`    | `node`        | Yes    | Yes        |
| Windows file      | `node`    | `node`        | Yes    |            |
| Windows soft-link | N/A       | N/A           | N/A    | N/A        |
| Windows hard-link | `node`    | `node`        | Yes    |            |
