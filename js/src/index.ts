import init, { diagram as wasmDiagram, math as wasmMath, read as wasmRead, Stream as WasmStream, type InitInput } from '../wasm/docstream_wasm.js';
import type { Block, Update } from './types.js';
export type * from './types.js';

let initialization: Promise<void> | undefined;

/** Initialize once. Browsers may omit input; Node should pass the WASM bytes. */
export function initialize(input?: InitInput | Promise<InitInput>): Promise<void> {
    initialization ??= init({ module_or_path: input }).then(() => undefined).catch(error => {
        initialization = undefined;
        throw error;
    });
    return initialization;
}

/** A whole document, read at once. */
export function read(text: string): Block[] { return JSON.parse(wasmRead(text)) as Block[]; }

/** Text as it streams in: each call gives the whole document as it now stands. Call free() when finished. */
export class Stream {
    private readonly inner = new WasmStream();
    push(more: string): Update { return JSON.parse(this.inner.push(more)) as Update; }
    finish(): Update { return JSON.parse(this.inner.finish()) as Update; }
    free(): void { this.inner.free(); }
}

/** TeX typeset as SVG sized in ems, in the colour of the text around it; `display` for a formula on its own line. */
export function math(tex: string, display = false): string { return wasmMath(tex, display); }

/** A Mermaid diagram as SVG in the colour of the text around it, or undefined for a kind not laid out. */
export function diagram(source: string): string | undefined { return wasmDiagram(source); }
