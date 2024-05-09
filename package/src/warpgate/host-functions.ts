import type * as raw from "../internal/raw-api-types";
import type { SetRequired } from "type-fest";

const {
  exec_command,
  host_log,
  get_env_var,
  set_env_var,
  from_virtual_path,
  to_virtual_path,
} = Host.getFunctions();

/**
 * Calls the `get_env_var` host function to manage environment
 * variables on the host.
 */
export function getEnvVar(name: string): string {
  const nameOffset = Memory.fromString(name).offset;
  const valueOffset = get_env_var(nameOffset);
  return Memory.find(valueOffset).readString();
}

/**
 * Calls the `set_env_var` host function to manage environment
 * variables on the host.
 *
 * When setting `PATH`, the provided value will append to `PATH`,
 * not overwrite it. Supports both `;` and `:` delimiters.
 */
export function setEnvVar(name: string, value: string): void {
  const nameOffset = Memory.fromString(name).offset;
  const valueOffset = Memory.fromString(value).offset;
  set_env_var(nameOffset, valueOffset);
}

/**
 * Calls `from_virtual_path` on the host to convert the provided value to a real path
 * from a virtual path.
 */
export function fromVirtualPath(virtualPath: string): string {
  const virtualPathOffset = Memory.fromString(virtualPath).offset;
  const pathOffset = from_virtual_path(virtualPathOffset);
  return Memory.find(pathOffset).readString();
}

/**
 * Calls `to_virtual_path` on the host to convert the provided value to a virtual path
 * from a real path.
 */
export function toVirtualPath(realPath: string): string {
  const pathOffset = Memory.fromString(realPath).offset;
  const virtualPathOffset = to_virtual_path(pathOffset);
  return Memory.find(virtualPathOffset).readString();
}

export type ExecCommandInput = SetRequired<
  Partial<raw.ExecCommandInput>,
  "command"
>;
export type ExecCommandOutput = raw.ExecCommandOutput;

/**
 * Calls the `exec_command` host function to execute a command on
 * the host as a synchronous child process.
 */
export function execCommand(input: ExecCommandInput): ExecCommandOutput {
  const inputMemory = Memory.fromJsonObject({
    args: [],
    env: {},
    set_executable: false,
    stream: false,
    working_dir: null,
    ...input,
  } satisfies raw.ExecCommandInput as any); // TODO: remove `any` once upstream type is fixed

  const outputOffset = exec_command(inputMemory.offset);
  return Memory.find(outputOffset).readJsonObject();
}

export type HostLogInput = SetRequired<Partial<raw.HostLogInput>, "message">;
export type HostLogTarget = raw.HostLogTarget;

/**
 * Calls the `host_log` host function to log a message to the host's terminal.
 */
export function hostLog(input: HostLogInput): void {
  const inputMemory = Memory.fromJsonObject({
    target: "tracing",
    data: {},
    ...input,
  } satisfies raw.HostLogInput as any); // TODO: remove `any` once upstream type is fixed

  host_log(inputMemory.offset);
}
