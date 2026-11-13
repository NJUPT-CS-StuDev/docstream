import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { diagram, html, initialize, math, read, Stream } from './index.js';

beforeAll(async () => { await initialize(readFileSync(new URL('../wasm/docstream_wasm_bg.wasm', import.meta.url))); });

describe('docstream on the web', () => {
    it('reads the fixture into the tree the core keeps', () => {
        const text = readFileSync(new URL('../../core/tests/fixtures/everything.md', import.meta.url), 'utf8').replace(/\r\n/g, '\n');
        const kept = JSON.parse(readFileSync(new URL('../../core/tests/fixtures/everything.json', import.meta.url), 'utf8'));
        expect(read(text)).toEqual(kept);
    });
    it('streams, the block being written already styled, settled ones kept', () => {
        const stream = new Stream();
        expect(JSON.stringify(stream.push('结论：**三次都避开').blocks[0]!.block)).toContain('"strong"');
        const first = stream.push('了**。\n\n下一段').blocks[0]!;
        expect(first.settled).toBe(true);
        expect(stream.finish().blocks.map(block => block.block.type)).toEqual(['paragraph', 'paragraph']);
        stream.free();
    });
    it('typesets TeX as SVG that sits on the baseline in the colour of the text', () => {
        const svg = math('\\frac{a}{b}', true);
        expect(svg.startsWith('<svg')).toBe(true);
        expect(svg).toContain('fill="currentColor"');
        expect(svg).toMatch(/vertical-align:-[\d.]+em/);
    });
    it('lays out Mermaid as SVG, and leaves a kind it does not know to be shown as written', () => {
        expect(diagram('graph TD\nA[开始] --> B[结束]')).toContain('开始');
        expect(diagram('pie\n"a": 1')).toBeUndefined();
    });
    it('draws a whole document as HTML for pages built ahead of time', () => {
        const page = html('## 标题\n\n$x^2$ 与\n\n```mermaid\ngraph TD\nA-->B\n```');
        expect(page).toContain('<h2 id="标题">');
        expect(page).toContain('<span class="math math-inline"><svg');
        expect(page).toContain('<figure class="diagram"><svg');
    });
});
