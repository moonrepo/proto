/*! *****************************************************************************
Copyright (c) Microsoft Corporation. All rights reserved.
Licensed under the Apache License, Version 2.0 (the "License"); you may not use
this file except in compliance with the License. You may obtain a copy of the
License at http://www.apache.org/licenses/LICENSE-2.0

THIS CODE IS PROVIDED ON AN *AS IS* BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
KIND, EITHER EXPRESS OR IMPLIED, INCLUDING WITHOUT LIMITATION ANY IMPLIED
WARRANTIES OR CONDITIONS OF TITLE, FITNESS FOR A PARTICULAR PURPOSE,
MERCHANTABLITY OR NON-INFRINGEMENT.

See the Apache Version 2.0 License for specific language governing permissions
and limitations under the License.
***************************************************************************** */

/// <reference types="urlpattern-polyfill" />

/**
 * The URL interface represents an object providing static methods used for creating object URLs.
 *
 * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL)
 */
interface URL {
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/hash) */
  hash: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/host) */
  host: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/hostname) */
  hostname: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/href) */
  href: string;
  toString(): string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/origin) */
  readonly origin: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/password) */
  password: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/pathname) */
  pathname: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/port) */
  port: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/protocol) */
  protocol: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/search) */
  search: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/searchParams) */
  readonly searchParams: URLSearchParams;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/username) */
  username: string;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/toJSON) */
  toJSON(): string;
}

declare var URL: {
  prototype: URL;
  new (url: string | URL, base?: string | URL): URL;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URL/canParse_static) */
  canParse(url: string | URL, base?: string): boolean;
};

/** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams) */
interface URLSearchParams {
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/size) */
  readonly size: number;
  /**
   * Appends a specified key/value pair as a new search parameter.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/append)
   */
  append(name: string, value: string): void;
  /**
   * Deletes the given search parameter, and its associated value, from the list of all search parameters.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/delete)
   */
  delete(name: string, value?: string): void;
  /**
   * Returns the first value associated to the given search parameter.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/get)
   */
  get(name: string): string | null;
  /**
   * Returns all the values association with a given search parameter.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/getAll)
   */
  getAll(name: string): string[];
  /**
   * Returns a Boolean indicating if such a search parameter exists.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/has)
   */
  has(name: string, value?: string): boolean;
  /**
   * Sets the value associated to a given search parameter to the given value. If there were several values, delete the others.
   *
   * [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/set)
   */
  set(name: string, value: string): void;
  /** [MDN Reference](https://developer.mozilla.org/docs/Web/API/URLSearchParams/sort) */
  sort(): void;
  /** Returns a string containing a query string suitable for use in a URL. Does not include the question mark. */
  toString(): string;
  forEach(
    callbackfn: (value: string, key: string, parent: URLSearchParams) => void,
    thisArg?: any
  ): void;
}

declare var URLSearchParams: {
  prototype: URLSearchParams;
  new (
    init?: string[][] | Record<string, string> | string | URLSearchParams
  ): URLSearchParams;
};
