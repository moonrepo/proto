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
