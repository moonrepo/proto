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
