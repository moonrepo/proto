/// <reference types="@moonrepo/proto-pdk" />

declare module "main" {
  export function detect_version_files(): I32;
  export function download_prebuilt(): I32;
  export function load_versions(): I32;
  export function locate_executables(): I32;
  export function parse_version_file(): I32;
  export function register_tool(): I32;
  export function resolve_version(): I32;
  export function verify_checksum(): I32;
}

// These imports unfortunately has to be copied, as extism-js only parses this file.
declare module "extism:host" {
  interface user {
    from_virtual_path(path: I64): I64;
    to_virtual_path(path: I64): I64;
    host_log(input: I64): void;
    exec_command(input: I64): I64;
    get_env_var(name: I64): I64;
    set_env_var(name: I64, value: I64): void;
  }
}
