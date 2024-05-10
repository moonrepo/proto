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

// These imports unfortunately have to be copied, as extism-js only parses this file.
declare module "extism:host" {
  interface user {
    from_virtual_path(path: PTR): PTR;
    to_virtual_path(path: PTR): PTR;
    host_log(input: PTR): void;
    exec_command(input: PTR): PTR;
    get_env_var(name: PTR): PTR;
    set_env_var(name: PTR, value: PTR): void;
  }
}
