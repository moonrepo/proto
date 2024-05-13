export type PluginErrorOptions = {
  /**
   * The original cause of the error.
   *
   * Homebrew version of ES2022 Error.cause, which we can't use as the
   * Extism doesn't yet support it.
   */
  cause?: unknown;
  returnCode?: number;
};

/**
 * An error containing a return code for Extism.
 *
 * NOTE: There's currently no way in the JS PDK to set an error along
 * with the specified return code. Thus this is currently semi useless.
 */
export class PluginError extends Error {
  returnCode: number;
  cause?: unknown;

  constructor(message?: string, options: PluginErrorOptions = {}) {
    super(message);

    this.returnCode = options.returnCode ?? 1;

    if ("cause" in options) {
      this.cause = options.cause;
    }

    if (this.cause instanceof Error && "stack" in this.cause) {
      if ("stack" in this) {
        // suuuper gross, but otherwise we lose the stack due to lack of native `Error.cause`
        this.stack = `${this.stack.split("\n").slice(0, 2).join("\n")}\n${
          this.cause.stack
        }`;
      } else {
        this.stack = this.cause.stack;
      }
    }
  }
}

export class UnsupportedOSError extends PluginError {
  constructor(
    { tool, os }: { tool: string; os: string },
    options?: PluginErrorOptions
  ) {
    super(`Unable to install ${tool}, unsupported OS ${os}.`, options);
  }
}

export class UnsupportedArchError extends PluginError {
  constructor(
    { tool, arch }: { tool: string; arch: string },
    options?: PluginErrorOptions
  ) {
    super(
      `Unable to install ${tool}, unsupported architecture ${arch}.`,
      options
    );
  }
}

export class UnsupportedCanaryError extends PluginError {
  constructor({ tool }: { tool: string }, options?: PluginErrorOptions) {
    super(`${tool} does not support canary/nightly versions.`, options);
  }
}

export class UnsupportedTargetError extends PluginError {
  constructor(
    { tool, arch, os }: { tool: string; arch: string; os: string },
    options?: PluginErrorOptions
  ) {
    super(
      `"Unable to install ${tool}, unsupported architecture ${arch} for ${os}."`,
      options
    );
  }
}
