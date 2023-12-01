# Results

|                           | `args[0]` | `current_exe` | Ctrl+C | Piped data |
| ------------------------- | --------- | ------------- | ------ | ---------- |
| Unix non-link: `node`     | `node`    | `node`        | Yes    | Yes        |
| Unix soft-link: `node`    | `node`    | `node`        | Yes    | Yes        |
| Unix hard-link: `node`    | `node`    | `node`        | Yes    | Yes        |
| Windows no-link: `node`   | `node`    | `node`        | Yes    |            |
| Windows soft-link: `node` | N/A       | N/A           | N/A    | N/A        |
| Windows hard-link: `node` | `node`    | `node`        | Yes    |            |
