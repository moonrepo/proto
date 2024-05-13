import { SemVer } from "semver";

export type CreatePluginFnFactoryOptions<I, O> = {
  /**
   * Postprocessor for the plugin function input to turn raw JSON values into class instances.
   */
  reviveInput?: (input: any) => I;

  /**
   * Preprocessor for the plugin function output, before stringifying it with JSON.stringify.
   *
   * Currently unused as we opt for implementing toJSON in our classes when possible,
   * as it greatly simplifies defining these plugin function creators.
   */
  replaceOutput?: (output: O) => any;
};

/**
 * Creates a new plugin function helper method.
 */
export function createPluginFnFactory<I, O>({
  reviveInput = (input) => input,
  replaceOutput = (output) => output,
}: CreatePluginFnFactoryOptions<I, O> = {}) {
  function getInput(): I {
    const inputString = Host.inputString();
    let input: unknown = undefined;

    try {
      input = JSON.parse(inputString);
    } catch {}

    return reviveInput ? reviveInput(input) : (input as I);
  }

  function setOutput(output: O) {
    const replaced = replaceOutput ? replaceOutput(output) : output;
    const outputString = JSON.stringify(replaced, baseJsonReplacer) ?? "";

    if (outputString) {
      Host.outputString(outputString);
    }
  }

  // I would've liked to allow returning Promise<O>, but then the JS PDK gets confused
  // when an error is thrown and returns success with neither an output nor error.
  return function createPluginFn(impl: (input: I) => O) {
    return () => {
      try {
        const input = getInput();
        const output = impl(input);
        setOutput(output as O);
      } catch (e) {
        // The pdk automatically calls error_set with the message of unhandled errors and uses
        // -1 as the return code. There's no way for us to change that return code currently,
        // but this is where we'd do it.
        throw e;
      }
    };
  };
}

/**
 * In our own classes, we just implement toJSON(), but externally owned
 * classes, such as SemVer, we need to handle manually.
 *
 * The alternative would be monkey-patching their prototype or needing to
 * always manually handle them via `replaceOutput` function in our plugin
 * function creators.
 *
 * Note: this is called during JSON.stringify, so _after_ `replaceOutput`.
 */
const baseJsonReplacer = (_key: string, value: any) => {
  if (value instanceof SemVer) {
    return value.toString();
  }

  return value;
};
