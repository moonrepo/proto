/**
 * Extism doesn't have official types yet, so I did my best and plan to upstream this.
 */

/// <reference lib="ES2020" />
/// <reference path="extism.polyfills.d.ts" />

declare type I32 = number;
declare type I64 = number;

declare module "extism:host" {
  // this will be declared by the plugin developer
  interface user {}
}

interface Console {
  log(...data: any[]): void;
  warn(...data: any[]): void;
  error(...data: any[]): void;

  // These are custom currently, but hopefully https://github.com/extism/js-pdk/pull/68 will get merged.
  debug(...data: any[]): void;
  info(...data: any[]): void;
}

declare var console: Console;

interface TextDecoderOptions {
  ignoreBOM?: boolean;
  fatal?: boolean;
}

interface DecodeOptions {
  stream?: any;
}

declare interface TextDecoder {
  readonly encoding: string;
  readonly fatal: boolean;
  readonly ignoreBOM: boolean;

  decode(buffer?: ArrayBufferLike, options?: DecodeOptions): string;
}

declare var TextDecoder: {
  prototype: TextDecoder;
  new (label?: string, options?: TextDecoderOptions): TextDecoder;
};

declare interface TextEncoder {
  readonly encoding: string;

  encode(input: string): Uint8Array;
}

declare var TextEncoder: {
  prototype: TextEncoder;
  new (): TextEncoder;
};

declare interface MemoryHandle {
  readonly offset: number;
  readonly len: number;

  readString(): string;
  readUInt32(): number;
  readUInt64(): number;
  readFloat32(): number;
  readUFloat64(): number;
  readBytes(): ArrayBufferLike;
  readJsonObject(): any;

  free(): void;
}

declare var MemoryHandle: {
  prototype: MemoryHandle;
  new (offset: number, len: number): MemoryHandle;
};

declare var Memory: {
  fromString(str: string): MemoryHandle;
  fromBuffer(bytes: ArrayBufferLike): MemoryHandle;
  fromJsonObject(obj: any): MemoryHandle;
  allocUInt32(i: number): MemoryHandle;
  allocUInt64(i: bigint): MemoryHandle;
  allocFloat32(i: number): MemoryHandle;
  allocFloat64(i: number): MemoryHandle;
  find(offset: number): MemoryHandle;
};

declare var Host: {
  getFunctions(): import("extism:host").user;
  inputBytes(): ArrayBufferLike;
  inputString(): string;
  outputBytes(output: ArrayBufferLike): boolean;
  outputString(output: string): boolean;
};

interface HttpRequest {
  url: string;
  method?:
    | "GET"
    | "HEAD"
    | "POST"
    | "PUT"
    | "DELETE"
    | "CONNECT"
    | "OPTIONS"
    | "TRACE"
    | "PATCH";
  headers?: Record<string, string | number | boolean>;
}

interface HttpResponse {
  body: string;
  status: number;
}

declare var Http: {
  request(req: HttpRequest, body?: ArrayBufferLike): HttpResponse;
};

declare var Var: {
  set(name: string, value: string | ArrayBufferLike): void;
  getBytes(name: string): ArrayBufferLike | null;
  getString(name: string): string | null;
};

declare var Config: {
  get(key: string): string | null;
};
