import { PluginError } from "../errors";
import { execCommand } from "./host-functions";
import type { HostEnvironment, TestEnvironment } from "./api";

/**
 * Fetch the provided request and return a response object.
 */
export function fetch(
  req: HttpRequest,
  body?: string | ArrayBufferLike
): HttpResponse {
  console.debug(`Fetching <url>${req.url}</url>`);

  body = typeof body === "string" ? new TextEncoder().encode(body) : body;

  try {
    return Http.request(
      {
        // The pdk _attempts_ to default to GET when missing, but it seems to turn
        // into 'undefined' or an empty string instead.
        method: "GET",
        ...req,
      },
      body
    );
  } catch (e) {
    throw new PluginError(`Failed to make request to <url>${req.url}</url>`, {
      cause: e,
    });
  }
}

/**
 * Fetch the provided URL and deserialize the response as JSON.
 */
export function fetchUrl<T>(url: string | URL): T {
  const response = fetch({ url: url.toString() });
  return JSON.parse(response.body);
}
//
/**
 * Fetch the provided URL and deserialize the response as bytes.
 */
export function fetchUrlBytes(url: string | URL): Uint8Array {
  const text = fetchUrlText(url);
  return new TextEncoder().encode(text);
}

/**
 * Fetch the provided URL and return the text response.
 */
export function fetchUrlText(url: string | URL): string {
  const response = fetch({ url: url.toString() });
  return response.body;
}

/**
 * Fetch the provided URL, deserialize the response as JSON,
 * and cache the response in memory for subsequent WASM function calls.
 */
export function fetchUrlWithCache<T>(url: string | URL): T {
  url = url.toString();

  const cachedBody = Var.getString(url);
  if (cachedBody) {
    console.debug(
      `Reading <url>${url}</url> from cache <mutedlight>(length = ${cachedBody.length})</mutedlight>`
    );

    return JSON.parse(cachedBody);
  }

  const body = fetchUrlText(url);

  console.debug(
    `Writing <url>${url}</url> to cache <mutedlight>(length = ${body.length})</mutedlight>`
  );

  Var.set(url, body);
  return JSON.parse(body);
}

/**
 * Load all git tags from the provided remote URL.
 * The `git` binary must exist on the current machine.
 */
export function loadGitTags(url: string | URL): string[] {
  url = url.toString();

  console.debug(`Loading Git tags from remote <url>${url}</url>`);

  const output = execCommand({
    command: "git",
    args: ["ls-remote", "--tags", "--sort", "version:refname", url],
  });

  const tags: string[] = [];

  if (output.exit_code !== 0) {
    console.debug("Failed to load Git tags");
    return tags;
  }

  for (const line of output.stdout.split("\n")) {
    // https://superuser.com/questions/1445823/what-does-mean-in-the-tags
    if (line.endsWith("^{}")) {
      continue;
    }

    const parts = line.split("\t");
    if (parts.length < 2) {
      continue;
    }

    const prefix = "refs/tags/";
    if (parts[1]?.startsWith(prefix)) {
      tags.push(parts[1].substring(prefix.length));
    }
  }

  console.debug(`Loaded ${tags.length} Git tags`);
  return tags;
}

/**
 * Check whether a command exists or not on the host machine.
 */
export function commandExists(env: HostEnvironment, command: string): boolean {
  console.debug(
    `Checking if command <shell>${command}</shell> exists on the host`
  );

  const result =
    env.os === "windows"
      ? execCommand({
          command: "powershell",
          args: ["-Command", `Get-Command ${command}`],
        })
      : execCommand({
          command: "which",
          args: [command],
        });

  if (result.exit_code === 0) {
    console.debug("Command does exist");
    return true;
  }

  console.debug("Command does NOT exist");
  return false;
}

/**
 * Return the ID for the current plugin.
 */
export function getPluginId(): string {
  const id = Config.get("plugin_id");

  if (!id) throw new PluginError("Missing plugin ID!");

  return id;
}

/**
 * Return information about the host environment.
 */
export function getHostEnvironment(): HostEnvironment {
  const config = Config.get("host_environment");

  if (!config) throw new PluginError("Missing host environment!");

  return JSON.parse(config) as HostEnvironment;
}

/**
 * Return information about the testing environment.
 */
export function getTestEnvironment(): TestEnvironment | null {
  const config = Config.get("test_environment");

  if (config) {
    return JSON.parse(config) as TestEnvironment;
  }

  return null;
}
