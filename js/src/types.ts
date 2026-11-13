// The document tree, the same shape docstream gives every client. See core/src/tree.rs.
export type Align = 'none' | 'left' | 'center' | 'right';
export type AlertKind = 'note' | 'tip' | 'important' | 'warning' | 'caution';
export type Inline =
    | { type: 'text'; text: string }
    | { type: 'strong' | 'emphasis' | 'strike' | 'highlight' | 'superscript' | 'subscript'; children: Inline[] }
    | { type: 'code'; text: string }
    | { type: 'math'; tex: string }
    | { type: 'link'; href: string; children: Inline[] }
    | { type: 'image'; src: string; alt: string }
    | { type: 'break' };
export interface Item { checked?: boolean; blocks: Block[] }
export type Block =
    | { type: 'paragraph'; inlines: Inline[] }
    | { type: 'heading'; level: number; inlines: Inline[] }
    | { type: 'quote'; blocks: Block[] }
    | { type: 'alert'; kind: AlertKind; blocks: Block[] }
    | { type: 'list'; ordered: boolean; start: number; items: Item[] }
    | { type: 'code'; language: string; text: string }
    | { type: 'math'; tex: string }
    | { type: 'table'; align: Align[]; head: Inline[][]; rows: Inline[][][] }
    | { type: 'rule' }
    | { type: 'html'; text: string };
/** A top-level block: `key` changes only when the block does; `settled` once nothing more can join it. */
export interface Keyed { key: string; settled: boolean; block: Block }
/** The whole document as it now stands. */
export interface Update { blocks: Keyed[]; done: boolean }
