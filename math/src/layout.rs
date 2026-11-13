//! Typesetting a formula by TeX's rules (The TeXbook, appendix G), each measure read from the font's OpenType MATH
//! table rather than written here. Lengths are in ems of the formula's size; y rises from the baseline.
use crate::tex::{Class, Face, Node, StyleChange};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use ttf_parser::{math, GlyphId, OutlineBuilder};

static DATA: &[u8] = include_bytes!("../fonts/NewCMMath-Regular.otf");

pub(crate) struct Font { face: ttf_parser::Face<'static>, upem: f32 }
pub(crate) fn font() -> &'static Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| { let face = ttf_parser::Face::parse(DATA, 0).expect("the math font"); Font { upem: f32::from(face.units_per_em()), face } })
}
impl Font {
    fn table(&self) -> math::Table<'static> { self.face.tables().math.expect("a MATH table") }
    fn constants(&self) -> math::Constants<'static> { self.table().constants.expect("MATH constants") }
    /// A MATH constant, in ems.
    fn c(&self, value: math::MathValue) -> f32 { f32::from(value.value) / self.upem }
    fn em(&self, units: impl Into<f32>) -> f32 { units.into() / self.upem }
}

/// What is drawn, placed: a glyph of the font at a scale, a rule, or words the client draws in its own font.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    Glyph { id: u16, x: f32, y: f32, scale: f32 },
    Rule { x: f32, y: f32, width: f32, height: f32 },
    Text { x: f32, y: f32, size: f32, text: String },
}

#[derive(Debug, Clone, Default)]
struct Bx { w: f32, h: f32, d: f32, items: Vec<Item>, italic: f32, class: Option<Class>, glyph: Option<u16>, limits: bool, accent_at: Option<f32> }

impl Bx {
    fn space(w: f32) -> Bx { Bx { w, ..Bx::default() } }
    /// This box moved right by `dx` and up by `dy`, its items with it.
    fn shifted(mut self, dx: f32, dy: f32) -> Vec<Item> { self.shift(dx, dy); self.items }
    /// This box's items moved in place.
    fn shift(&mut self, dx: f32, dy: f32) {
        for item in &mut self.items {
            match item { Item::Glyph { x, y, .. } | Item::Rule { x, y, .. } | Item::Text { x, y, .. } => { *x += dx; *y += dy; } }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct St { level: u8, cramped: bool }
impl St {
    const DISPLAY: St = St { level: 0, cramped: false };
    const TEXT: St = St { level: 1, cramped: false };
    fn display(self) -> bool { self.level == 0 }
    fn script(self) -> bool { self.level >= 2 }
    fn size(self) -> f32 { let c = font().constants(); match self.level { 0 | 1 => 1.0, 2 => f32::from(c.script_percent_scale_down()) / 100.0, _ => f32::from(c.script_script_percent_scale_down()) / 100.0 } }
    fn sup(self) -> St { St { level: if self.level < 2 { 2 } else { 3 }, cramped: self.cramped } }
    fn sub(self) -> St { St { level: if self.level < 2 { 2 } else { 3 }, cramped: true } }
    fn num(self) -> St { St { level: (self.level + 1).clamp(1, 3).max(if self.level == 0 { 1 } else { self.level + 1 }).min(3), cramped: self.cramped } }
    fn den(self) -> St { St { cramped: true, ..self.num() } }
    fn cramp(self) -> St { St { cramped: true, ..self } }
    fn of(change: StyleChange) -> St { St { level: match change { StyleChange::Display => 0, StyleChange::Text => 1, StyleChange::Script => 2, StyleChange::ScriptScript => 3 }, cramped: false } }
}

/// The letter a face draws: italic letters are the Mathematical Alphanumeric Symbols, with Unicode's older exceptions.
fn styled(c: char, face: Face) -> char {
    let at = |base: u32, offset: u32| char::from_u32(base + offset).unwrap_or(c);
    let (upper, lower, digit) = (c.is_ascii_uppercase(), c.is_ascii_lowercase(), c.is_ascii_digit());
    let i = if upper { c as u32 - 'A' as u32 } else if lower { c as u32 - 'a' as u32 } else if digit { c as u32 - '0' as u32 } else { 0 };
    match face {
        Face::Upright => c,
        Face::Italic => match c { 'h' => 'ℎ', _ if upper => at(0x1D434, i), _ if lower => at(0x1D44E, i), 'α'..='ω' => at(0x1D6FC, c as u32 - 'α' as u32), 'ϵ' => '𝜖', 'ϑ' => '𝜗', 'ϕ' => '𝜙', 'ϱ' => '𝜚', 'ϖ' => '𝜛', _ => c },
        Face::Bold => if upper { at(0x1D400, i) } else if lower { at(0x1D41A, i) } else if digit { at(0x1D7CE, i) } else { c },
        Face::DoubleStruck => match c { 'C' => 'ℂ', 'H' => 'ℍ', 'N' => 'ℕ', 'P' => 'ℙ', 'Q' => 'ℚ', 'R' => 'ℝ', 'Z' => 'ℤ', _ if upper => at(0x1D538, i), _ if lower => at(0x1D552, i), _ if digit => at(0x1D7D8, i), _ => c },
        Face::Script => match c { 'B' => 'ℬ', 'E' => 'ℰ', 'F' => 'ℱ', 'H' => 'ℋ', 'I' => 'ℐ', 'L' => 'ℒ', 'M' => 'ℳ', 'R' => 'ℛ', 'e' => 'ℯ', 'g' => 'ℊ', 'o' => 'ℴ', _ if upper => at(0x1D49C, i), _ if lower => at(0x1D4B6, i), _ => c },
        Face::Fraktur => match c { 'C' => 'ℭ', 'H' => 'ℌ', 'I' => 'ℑ', 'R' => 'ℜ', 'Z' => 'ℨ', _ if upper => at(0x1D504, i), _ if lower => at(0x1D51E, i), _ => c },
        Face::SansSerif => if upper { at(0x1D5A0, i) } else if lower { at(0x1D5BA, i) } else if digit { at(0x1D7E2, i) } else { c },
        Face::Mono => if upper { at(0x1D670, i) } else if lower { at(0x1D68A, i) } else if digit { at(0x1D7F6, i) } else { c },
    }
}

fn glyph_box(id: GlyphId, size: f32) -> Bx {
    let f = font();
    let advance = f.em(f.face.glyph_hor_advance(id).unwrap_or(0)) * size;
    let (h, d) = f.face.glyph_bounding_box(id).map_or((0.0, 0.0), |b| (f.em(b.y_max).max(0.0) * size, f.em(-b.y_min).max(0.0) * size));
    let info = f.table().glyph_info;
    let italic = info.and_then(|i| i.italic_corrections).and_then(|v| v.get(id)).map_or(0.0, |v| f.c(v) * size);
    let accent_at = info.and_then(|i| i.top_accent_attachments).and_then(|v| v.get(id)).map(|v| f.c(v) * size);
    Bx { w: advance, h, d, items: vec![Item::Glyph { id: id.0, x: 0.0, y: 0.0, scale: size }], italic, glyph: Some(id.0), accent_at, ..Bx::default() }
}

/// A character in the font, or words the client draws when the font has not got it (CJK in \text, say).
fn char_box(c: char, size: f32) -> Bx {
    match font().face.glyph_index(c) {
        Some(id) => glyph_box(id, size),
        None => {
            let w = if c.is_ascii() { 0.5 } else { 1.0 } * size;
            Bx { w, h: 0.8 * size, d: 0.2 * size, items: vec![Item::Text { x: 0.0, y: 0.0, size, text: c.to_string() }], ..Bx::default() }
        }
    }
}

/// Glyphs side by side, aligned on the baseline.
fn row(boxes: Vec<Bx>) -> Bx {
    let mut out = Bx::default();
    for b in boxes {
        out.h = out.h.max(b.h); out.d = out.d.max(b.d);
        let w = b.w; let italic = b.italic;
        out.items.extend(b.shifted(out.w, 0.0));
        out.w += w; out.italic = italic;
    }
    out
}

/// The space TeX puts between two atoms, in mu (1/18 em); in scripts only the unparenthesized ones.
fn spacing(left: Class, right: Class, st: St) -> f32 {
    use Class::*;
    let (mu, scripts) = match (left, right) {
        (Ord, Op) | (Op, Ord) | (Op, Op) | (Close, Op) | (Inner, Op) => (3.0, true),
        (Ord, Bin) | (Bin, Ord) | (Bin, Op) | (Bin, Open) | (Bin, Inner) | (Close, Bin) | (Inner, Bin) | (Op, Bin) => (4.0, false),
        (Ord, Rel) | (Rel, Ord) | (Rel, Op) | (Rel, Open) | (Rel, Inner) | (Close, Rel) | (Inner, Rel) | (Op, Rel) | (Bin, Rel) => (5.0, false),
        (Ord, Inner) | (Op, Inner) | (Close, Inner) | (Inner, Ord) | (Inner, Open) | (Inner, Punct) | (Inner, Inner) => (3.0, false),
        (Punct, _) => (3.0, false),
        _ => (0.0, true),
    };
    if mu == 0.0 || (st.script() && !scripts) { 0.0 } else { mu / 18.0 * st.size() }
}

pub(crate) struct Typeset { pub width: f32, pub height: f32, pub depth: f32, pub items: Vec<Item> }

pub(crate) fn typeset(node: &Node, display: bool) -> Typeset {
    let b = layout(node, if display { St::DISPLAY } else { St::TEXT });
    Typeset { width: b.w, height: b.h, depth: b.d, items: b.items }
}

fn list(nodes: &[Node], st: St) -> Bx {
    let mut boxes: Vec<Bx> = nodes.iter().map(|n| layout(n, st)).collect();
    // A binary operator with nothing to bind on one side is an ordinary symbol (TeX's rule 5 and 6).
    for i in 0..boxes.len() {
        if boxes[i].class == Some(Class::Bin) {
            let before = boxes[..i].iter().rev().find_map(|b| b.class);
            let after = boxes[i + 1..].iter().find_map(|b| b.class);
            if matches!(before, None | Some(Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct)) || matches!(after, None | Some(Class::Rel | Class::Close | Class::Punct)) { boxes[i].class = Some(Class::Ord); }
        }
    }
    let mut spaced = Vec::with_capacity(boxes.len() * 2);
    let mut last: Option<Class> = None;
    for b in boxes {
        if let (Some(l), Some(r)) = (last, b.class) { let s = spacing(l, r, st); if s != 0.0 { spaced.push(Bx::space(s)); } }
        if b.class.is_some() { last = b.class; }
        spaced.push(b);
    }
    let class = if spaced.len() == 1 { spaced[0].class } else { Some(Class::Ord) };
    let single = if spaced.len() == 1 { spaced[0].clone() } else { Bx::default() };
    let mut out = row(spaced);
    out.class = class;
    if single.glyph.is_some() { out.glyph = single.glyph; out.accent_at = single.accent_at; out.limits = single.limits; }
    out
}

fn layout(node: &Node, st: St) -> Bx {
    let f = font(); let c = f.constants(); let size = st.size();
    match node {
        Node::Symbol { text, class, face } => {
            let mut b = row(text.chars().map(|ch| char_box(styled(ch, *face), size)).collect());
            if text.chars().count() == 1 { if let Some(Item::Glyph { id, .. }) = b.items.first() { b.glyph = Some(*id); b.accent_at = glyph_box(GlyphId(*id), size).accent_at; } }
            // An italic letter keeps its correction for what follows; a symbol does not lean.
            if !matches!(face, Face::Italic) { b.italic = 0.0; }
            b.class = Some(*class); b
        }
        Node::Text { text, class } => { let mut b = row(text.chars().map(|ch| char_box(ch, size)).collect()); b.italic = 0.0; b.class = Some(*class); b }
        Node::Group(nodes) => list(nodes, st),
        Node::Space(em) => Bx::space(em * size),
        Node::Style(change, inner) => layout(inner, St::of(*change)),
        Node::NamedOp { name, limits } => { let mut b = row(name.chars().map(|ch| char_box(ch, size)).collect()); b.italic = 0.0; b.class = Some(Class::Op); b.limits = *limits && st.display(); b }
        Node::LargeOp { symbol, limits } => {
            let Some(id) = f.face.glyph_index(*symbol) else { return char_box(*symbol, size) };
            // In display, the variant tall enough for display operators.
            let id = if st.display() { variant(id, f.em(c.display_operator_min_height()), true).unwrap_or(id) } else { id };
            let mut b = glyph_box(id, size);
            // A large operator sits centred on the axis.
            let shift = (b.h - b.d) / 2.0 - f.c(c.axis_height()) * size;
            b.h -= shift; b.d += shift;
            b.shift(0.0, -shift);
            b.class = Some(Class::Op); b.limits = *limits && st.display(); b
        }
        Node::Scripts { base, sup, sub } => scripts(base, sup.as_deref(), sub.as_deref(), st),
        Node::Frac { num, den, rule, style } => {
            let st = style.map_or(st, St::of);
            let size = st.size();
            let (n, d) = (layout(num, st.num()), layout(den, st.den()));
            let axis = f.c(c.axis_height()) * size;
            let t = if *rule { f.c(c.fraction_rule_thickness()) * size } else { 0.0 };
            let (mut u, mut v) = if st.display() { (f.c(c.fraction_numerator_display_style_shift_up()), f.c(c.fraction_denominator_display_style_shift_down())) } else { (f.c(c.fraction_numerator_shift_up()), f.c(c.fraction_denominator_shift_down())) };
            u *= size; v *= size;
            let (gn, gd) = if st.display() { (f.c(c.fraction_num_display_style_gap_min()), f.c(c.fraction_denom_display_style_gap_min())) } else { (f.c(c.fraction_numerator_gap_min()), f.c(c.fraction_denominator_gap_min())) };
            u = u.max(axis + t / 2.0 + gn * size + n.d);
            v = v.max(d.h + gn.max(0.0) * 0.0 + gd * size - (axis - t / 2.0));
            let pad = 0.12 * size;
            let w = n.w.max(d.w) + 2.0 * pad;
            let mut out = Bx { w, h: u + n.h, d: v + d.d, class: Some(Class::Inner), ..Bx::default() };
            let (nw, dw) = (n.w, d.w);
            out.items.extend(n.shifted((w - nw) / 2.0, u));
            out.items.extend(d.shifted((w - dw) / 2.0, -v));
            if *rule { out.items.push(Item::Rule { x: pad * 0.5, y: axis + t / 2.0, width: w - pad, height: t }); }
            out
        }
        Node::Sqrt { body, index } => {
            let b = layout(body, st.cramp());
            let gap = if st.display() { f.c(c.radical_display_style_vertical_gap()) } else { f.c(c.radical_vertical_gap()) } * size;
            let t = f.c(c.radical_rule_thickness()) * size;
            let need = b.h + b.d + gap + t;
            let mut r = stretched('√', need, size);
            // The sign's top meets the rule over the body.
            let top = b.h + gap + t;
            let lift = top - r.h;
            r.h += lift; r.d -= lift;
            let (rw, bw) = (r.w, b.w);
            let mut out = Bx { h: top + f.c(c.radical_extra_ascender()) * size, d: b.d.max(r.d), class: Some(Class::Ord), ..Bx::default() };
            let mut x = 0.0;
            if let Some(index) = index {
                let i = layout(index, St { level: 3, cramped: true });
                let raise = f32::from(c.radical_degree_bottom_raise_percent()) / 100.0 * (r.h + r.d) - r.d;
                let before = f.c(c.radical_kern_before_degree()) * size;
                let after = f.c(c.radical_kern_after_degree()) * size;
                let iw = i.w;
                out.h = out.h.max(raise + i.h);
                out.items.extend(i.shifted(before, raise));
                x = (before + iw + after).max(0.0);
            }
            out.items.extend(r.shifted(x, lift));
            out.items.push(Item::Rule { x: x + rw, y: top, width: bw, height: t });
            out.items.extend(b.shifted(x + rw, 0.0));
            out.w = x + rw + bw;
            out
        }
        Node::Fenced { open, close, body } => {
            let b = list(body, st);
            let axis = f.c(c.axis_height()) * size;
            let half = (b.h - axis).max(b.d + axis);
            let need = (2.0 * half * 0.901).max(2.0 * half - 0.5 * size).max(f.em(c.delimited_sub_formula_min_height()) * size * 0.5);
            let mut parts = Vec::new();
            if let Some(o) = open { let mut d = delimiter(*o, need, size); d.class = Some(Class::Open); parts.push(d); }
            let mut inner = b; inner.class = Some(Class::Inner); parts.push(inner);
            if let Some(cl) = close { let mut d = delimiter(*cl, need, size); d.class = Some(Class::Close); parts.push(d); }
            let mut out = row(parts); out.class = Some(Class::Inner); out
        }
        Node::Sized { delimiter: ch, class, size: em } => { let mut d = if *ch == '.' { Bx::space(0.0) } else { delimiter(*ch, em * size, size) }; d.class = Some(*class); d }
        Node::Accent { base, accent, stretch } => {
            let b = layout(base, st.cramp());
            let a = if *stretch { stretched_wide(*accent, b.w, size) } else { char_box(*accent, size) };
            let base_height = f.c(c.accent_base_height()) * size;
            // A combining accent is drawn to sit over x-height letters, raised for taller ones; an arrow over the base
            // is set clear above it, its ink a small gap over the base's top.
            let combining = matches!(*accent as u32, 0x300..=0x36F | 0x20D0..=0x20FF);
            let lift = if combining { (b.h - base_height).max(0.0) } else { b.h + 0.1 * size - ink_bottom(&a) };
            // Centred on the base's accent point, or its middle; the accent's own point, or its middle.
            let over = b.accent_at.unwrap_or(b.w / 2.0 + b.italic / 2.0);
            let own = a.accent_at.unwrap_or(a.w / 2.0);
            let x = over - own;
            let (bw, bh, bd, aw, ah) = (b.w, b.h, b.d, a.w, a.h);
            let mut out = Bx { w: bw.max(x + aw), h: bh.max(lift + ah), d: bd, class: b.class.or(Some(Class::Ord)), ..Bx::default() };
            out.items.extend(b.shifted(0.0, 0.0));
            out.items.extend(a.shifted(x, lift));
            out
        }
        Node::Over { base, under } => {
            let b = layout(base, if *under { st } else { st.cramp() });
            let (t, gap) = if *under { (f.c(c.underbar_rule_thickness()), f.c(c.underbar_vertical_gap())) } else { (f.c(c.overbar_rule_thickness()), f.c(c.overbar_vertical_gap())) };
            let (t, gap) = (t * size, gap * size);
            let (bw, bh, bd) = (b.w, b.h, b.d);
            let mut out = Bx { w: bw, h: bh, d: bd, class: Some(Class::Ord), ..Bx::default() };
            out.items.extend(b.shifted(0.0, 0.0));
            if *under { out.items.push(Item::Rule { x: 0.0, y: -bd - gap, width: bw, height: t }); out.d = bd + gap + t; }
            else { out.items.push(Item::Rule { x: 0.0, y: bh + gap + t, width: bw, height: t }); out.h = bh + gap + t + f.c(c.overbar_extra_ascender()) * size; }
            out
        }
        Node::Array { rows, align, open, close, gap } => array(rows, align, *open, *close, *gap, st),
        Node::Stack { base, over, under, class } => {
            let mut out = stack(layout(base, st), over.as_ref().map(|n| layout(n, st.sup())), under.as_ref().map(|n| layout(n, st.sub())), size);
            out.class = Some(*class); out
        }
        Node::Brace { body, under } => {
            let b = layout(body, st);
            let brace = stretched_wide(if *under { '⏟' } else { '⏞' }, b.w, size);
            let mut out = if *under { stack(b, None, Some(brace), size) } else { stack(b, Some(brace), None, size) };
            // What is written after it, ^ or _, goes above or below it, as limits do.
            out.class = Some(Class::Op); out.limits = true; out
        }
        Node::Arrow { symbol, over, under } => {
            let (p, q) = (over.as_ref().map(|n| layout(n, st.sup())), under.as_ref().map(|n| layout(n, st.sub())));
            let span = p.as_ref().map_or(0.0, |b| b.w).max(q.as_ref().map_or(0.0, |b| b.w)) + 0.6 * size;
            let mut arrow = stretched_wide(*symbol, span.max(1.0 * size), size);
            arrow.glyph = None;
            let mut out = stack(arrow, p, q, size);
            out.class = Some(Class::Rel); out
        }
        Node::Boxed(inner) => {
            let b = layout(inner, st);
            let (pad, t) = (0.2 * size, 0.04 * size);
            let (w, h, d) = (b.w + 2.0 * pad, b.h + pad, b.d + pad);
            let mut out = Bx { w, h, d, class: Some(Class::Ord), ..Bx::default() };
            out.items.extend(b.shifted(pad, 0.0));
            out.items.push(Item::Rule { x: 0.0, y: h, width: w, height: t });
            out.items.push(Item::Rule { x: 0.0, y: -d + t, width: w, height: t });
            out.items.push(Item::Rule { x: 0.0, y: h, width: t, height: h + d });
            out.items.push(Item::Rule { x: w - t, y: h, width: t, height: h + d });
            out
        }
        Node::Phantom(inner) => { let b = layout(inner, st); Bx { w: b.w, h: b.h, d: b.d, class: b.class, ..Bx::default() } }
    }
}

fn scripts(base: &Node, sup: Option<&Node>, sub: Option<&Node>, st: St) -> Bx {
    let f = font(); let c = f.constants(); let size = st.size();
    let b = layout(base, st);
    let (p, q) = (sup.map(|n| layout(n, st.sup())), sub.map(|n| layout(n, st.sub())));
    // Limits above and below a large operator in display.
    if b.limits {
        let w = b.w.max(p.as_ref().map_or(0.0, |p| p.w)).max(q.as_ref().map_or(0.0, |q| q.w));
        let (bw, bh, bd) = (b.w, b.h, b.d);
        let mut out = Bx { w, h: bh, d: bd, class: Some(Class::Op), ..Bx::default() };
        out.items.extend(b.shifted((w - bw) / 2.0, 0.0));
        if let Some(p) = p {
            let rise = (bh + f.c(c.upper_limit_gap_min()) * size + p.d).max(bh + f.c(c.upper_limit_baseline_rise_min()) * size);
            out.h = rise + p.h; let pw = p.w; out.items.extend(p.shifted((w - pw) / 2.0, rise));
        }
        if let Some(q) = q {
            let drop = (bd + f.c(c.lower_limit_gap_min()) * size + q.h).max(bd + f.c(c.lower_limit_baseline_drop_min()) * size);
            out.d = drop + q.d; let qw = q.w; out.items.extend(q.shifted((w - qw) / 2.0, -drop));
        }
        return out;
    }
    // A single glyph raises its scripts from the constants alone; a built box from its own height and depth too.
    let op = b.class == Some(Class::Op);
    let built = b.glyph.is_none() || op;
    let mut u = if st.cramped { f.c(c.superscript_shift_up_cramped()) } else { f.c(c.superscript_shift_up()) } * size;
    let mut v = f.c(c.subscript_shift_down()) * size;
    if built { u = u.max(b.h - f.c(c.superscript_baseline_drop_max()) * size); v = v.max(b.d + f.c(c.subscript_baseline_drop_min()) * size); }
    if let Some(p) = &p { u = u.max(p.d + f.c(c.superscript_bottom_min()) * size); }
    if let Some(q) = &q { v = v.max(q.h - f.c(c.subscript_top_max()) * size); }
    if let (Some(p), Some(q)) = (&p, &q) {
        let gap = (u - p.d) - (q.h - v);
        let min = f.c(c.sub_superscript_gap_min()) * size;
        if gap < min {
            v += min - gap;
            let lift = f.c(c.superscript_bottom_max_with_subscript()) * size - (u - p.d);
            if lift > 0.0 { u += lift; v -= lift; }
        }
    }
    let after = f.c(c.space_after_script()) * size;
    let (bw, bh, bd, italic) = (b.w, b.h, b.d, b.italic);
    let mut out = Bx { w: bw, h: bh, d: bd, class: b.class, ..Bx::default() };
    out.items.extend(b.shifted(0.0, 0.0));
    let mut w = bw;
    if let Some(p) = p { let x = if op { bw } else { bw + italic }; out.h = out.h.max(u + p.h); w = w.max(x + p.w); out.items.extend(p.shifted(x, u)); }
    if let Some(q) = q { let x = if op { bw - italic } else { bw }; out.d = out.d.max(v + q.d); w = w.max(x + q.w); out.items.extend(q.shifted(x, -v)); }
    out.w = w + after;
    out
}

/// A glyph's vertical variant at least `height` ems tall, if the font has one.
fn variant(id: GlyphId, height: f32, vertical: bool) -> Option<GlyphId> {
    let f = font();
    let variants = f.table().variants?;
    let constructions = if vertical { variants.vertical_constructions } else { variants.horizontal_constructions };
    let construction = constructions.get(id)?;
    let mut best = None;
    for v in construction.variants { best = Some(v.variant_glyph); if f.em(v.advance_measurement) >= height { break; } }
    best
}

/// How far above its baseline a box's lowest ink is.
fn ink_bottom(b: &Bx) -> f32 {
    let f = font();
    b.items.iter().filter_map(|item| match item {
        Item::Glyph { id, y, scale, .. } => f.face.glyph_bounding_box(GlyphId(*id)).map(|r| y + f.em(r.y_min) * scale),
        Item::Rule { y, height, .. } => Some(y - height),
        Item::Text { y, .. } => Some(*y),
    }).fold(f32::MAX, f32::min).min(b.h)
}

/// A delimiter or radical sign grown to at least `height` ems in all, its bottom on the baseline.
fn stretched(c: char, height: f32, size: f32) -> Bx { grown(c, height, size, true) }

/// A glyph grown to at least `length` ems along its axis: the first of the font's variants long enough, else built
/// from its parts as the font's glyph assembly says, the extenders repeated until it is. Upright it stands on the
/// baseline; sideways it sits on it as the parts do.
fn grown(c: char, length: f32, size: f32, vertical: bool) -> Bx {
    let f = font();
    let Some(id) = f.face.glyph_index(c) else { return char_box(c, size) };
    let target = length / size;
    let plain = |id: GlyphId| { let mut b = glyph_box(id, size); if !vertical { b.accent_at = None; } b };
    let Some(construction) = f.table().variants.and_then(|v| if vertical { v.vertical_constructions } else { v.horizontal_constructions }.get(id)) else { return plain(id) };
    for v in construction.variants { if f.em(v.advance_measurement) >= target { return plain(v.variant_glyph); } }
    let Some(assembly) = construction.assembly else { return construction.variants.last().map_or_else(|| plain(id), |v| plain(v.variant_glyph)) };
    let overlap = f.em(f.table().variants.map_or(0, |v| v.min_connector_overlap));
    let parts: Vec<math::GlyphPart> = assembly.parts.into_iter().collect();
    let (fixed_n, ext_n) = (parts.iter().filter(|p| !p.part_flags.extender()).count(), parts.iter().filter(|p| p.part_flags.extender()).count());
    let fixed: f32 = parts.iter().filter(|p| !p.part_flags.extender()).map(|p| f.em(p.full_advance)).sum();
    let extender: f32 = parts.iter().filter(|p| p.part_flags.extender()).map(|p| f.em(p.full_advance)).sum();
    let total = |n: usize| fixed + extender * n as f32 - overlap * (fixed_n + ext_n * n).saturating_sub(1) as f32;
    let mut repeats = 0usize;
    while total(repeats) < target && repeats < 64 && extender > 0.0 { repeats += 1; }
    let mut out = Bx::default();
    let mut at = 0.0;
    let mut first = true;
    for part in &parts {
        for _ in 0..if part.part_flags.extender() { repeats } else { 1 } {
            if !first { at -= overlap; }
            first = false;
            let g = glyph_box(part.glyph_id, size);
            if vertical {
                // Parts stack from the bottom; each glyph's own descent puts it.
                out.w = out.w.max(g.w);
                let d = g.d;
                out.items.extend(g.shifted(0.0, at * size + d));
            } else {
                out.h = out.h.max(g.h); out.d = out.d.max(g.d);
                out.items.extend(g.shifted(at * size, 0.0));
            }
            at += f.em(part.full_advance);
        }
    }
    if vertical { out.h = at * size; } else { out.w = at * size; }
    out
}

/// A delimiter grown to `height` ems, centred on the axis.
fn delimiter(c: char, height: f32, size: f32) -> Bx {
    let f = font();
    let mut d = stretched(c, height, size);
    let axis = f.c(f.constants().axis_height()) * size;
    let shift = (d.h - d.d) / 2.0 - axis;
    d.h -= shift; d.d += shift;
    d.shift(0.0, -shift);
    d.glyph = None;
    d
}

/// An accent, brace or arrow grown sideways to cover `width` ems.
fn stretched_wide(c: char, width: f32, size: f32) -> Bx { grown(c, width, size, false) }

/// `over` and `under` centred above and below `base`, at the gaps TeX keeps for limits.
fn stack(base: Bx, over: Option<Bx>, under: Option<Bx>, size: f32) -> Bx {
    let f = font(); let c = f.constants();
    let w = base.w.max(over.as_ref().map_or(0.0, |b| b.w)).max(under.as_ref().map_or(0.0, |b| b.w));
    let (bw, bh, bd) = (base.w, base.h, base.d);
    let mut out = Bx { w, h: bh, d: bd, ..Bx::default() };
    out.items.extend(base.shifted((w - bw) / 2.0, 0.0));
    if let Some(p) = over {
        let rise = bh + f.c(c.upper_limit_gap_min()) * size * 0.6 + p.d;
        out.h = rise + p.h; let pw = p.w; out.items.extend(p.shifted((w - pw) / 2.0, rise));
    }
    if let Some(q) = under {
        let drop = bd + f.c(c.lower_limit_gap_min()) * size * 0.6 + q.h;
        out.d = drop + q.d; let qw = q.w; out.items.extend(q.shifted((w - qw) / 2.0, -drop));
    }
    out
}

fn array(rows: &[Vec<Node>], align: &[char], open: Option<char>, close: Option<char>, gap: f32, st: St) -> Bx {
    let f = font(); let c = f.constants();
    let cell_st = if st.display() { St::TEXT } else { st };
    let size = st.size();
    // A cell that continues an aligned equation (`&= d`) spaces its leading relation as if something stood before it.
    let cells: Vec<Vec<Bx>> = rows.iter().map(|r| r.iter().enumerate().map(|(i, n)| {
        if i > 0 && align.get(i) == Some(&'l') && align.get(i - 1) == Some(&'r') { let mut items = vec![Node::Group(vec![])]; match n { Node::Group(inner) => items.extend(inner.iter().cloned()), other => items.push(other.clone()) } layout(&Node::Group(items), cell_st) } else { layout(n, cell_st) }
    }).collect()).collect();
    let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<f32> = (0..columns).map(|i| cells.iter().filter_map(|r| r.get(i)).map(|b| b.w).fold(0.0, f32::max)).collect();
    let col_gap = if gap > 0.0 { gap * size } else { 0.0 };
    let strut_h = 0.7 * size; let strut_d = 0.3 * size; let jot = 0.25 * size;
    let mut out = Bx::default();
    let mut y = 0.0f32;
    let mut first = true;
    for row in &cells {
        let h = row.iter().map(|b| b.h).fold(strut_h, f32::max);
        let d = row.iter().map(|b| b.d).fold(strut_d, f32::max);
        if !first { y -= h + jot; } else { y = 0.0; out.h = h; first = false; }
        let mut x = 0.0;
        for (i, cell) in row.iter().enumerate() {
            let a = align.get(i).copied().unwrap_or('c');
            let dx = match a { 'l' => 0.0, 'r' => widths[i] - cell.w, _ => (widths[i] - cell.w) / 2.0 };
            out.items.extend(cell.clone().shifted(x + dx, y));
            x += widths[i] + col_gap;
        }
        out.d = -y + d;
        y -= d;
    }
    out.w = widths.iter().sum::<f32>() + col_gap * columns.saturating_sub(1) as f32;
    // Centred on the axis.
    let axis = f.c(c.axis_height()) * size;
    let shift = (out.h - out.d) / 2.0 - axis;
    out.shift(0.0, -shift);
    out.h -= shift; out.d += shift;
    let pad = if open.is_some() || close.is_some() { 0.2 * size } else { 0.0 };
    let body = Bx { w: out.w + 2.0 * pad, items: out.clone().shifted(pad, 0.0), ..out };
    let need = body.h + body.d;
    let mut parts = Vec::new();
    if let Some(o) = open { parts.push(delimiter(o, need, size)); }
    parts.push(body);
    if let Some(cl) = close { parts.push(delimiter(cl, need, size)); }
    let mut r = row(parts); r.class = Some(Class::Inner); r.italic = 0.0; r
}

/// A glyph's outline as SVG path data, in ems, y downward from its origin.
pub(crate) fn outline(id: u16) -> String {
    struct Path { d: String, upem: f32 }
    impl Path { fn p(&mut self, x: f32, y: f32) { use std::fmt::Write; let _ = write!(self.d, "{} {} ", round(x / self.upem), round(-y / self.upem)); } }
    impl OutlineBuilder for Path {
        fn move_to(&mut self, x: f32, y: f32) { self.d.push('M'); self.p(x, y); }
        fn line_to(&mut self, x: f32, y: f32) { self.d.push('L'); self.p(x, y); }
        fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) { self.d.push('Q'); self.p(x1, y1); self.p(x, y); }
        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) { self.d.push('C'); self.p(x1, y1); self.p(x2, y2); self.p(x, y); }
        fn close(&mut self) { self.d.push('Z'); }
    }
    let f = font();
    let mut path = Path { d: String::new(), upem: f.upem };
    f.face.outline_glyph(GlyphId(id), &mut path);
    path.d.trim_end().to_string()
}

pub(crate) fn round(v: f32) -> f32 { (v * 10000.0).round() / 10000.0 }

/// Every glyph the items use, by id, with its outline.
pub(crate) fn outlines(items: &[Item]) -> BTreeMap<u16, String> {
    items.iter().filter_map(|item| if let Item::Glyph { id, .. } = item { Some(*id) } else { None }).map(|id| (id, outline(id))).collect()
}
