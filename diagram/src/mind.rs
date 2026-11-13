//! Mind maps: an outline by indentation, drawn with the root in the middle and its branches to either side, each
//! side a tidy tree; the branches in boxes, what grows from them as words on a line.
use crate::draw::{dot, label, lines_size, rect, wrap, Diagram, Fill, Item, Pen};
use crate::flow::{finish, Bounds};

struct Node { lines: Vec<String>, w: f32, h: f32, circle: bool, children: Vec<Node> }

/// The text of a node, its shape marks taken off: `((circle))`, `(round)`, `[square]`, `{{hexagon}}`, `))bang((`.
fn node(raw: &str, depth: usize) -> Node {
    let mut s = raw.trim();
    if let Some(at) = s.find("::icon(") { s = s[..at].trim(); }
    let s = s.split(":::").next().unwrap_or("").trim();
    let shapes = [("((", "))"), ("))", "(("), (")", "("), ("{{", "}}"), ("[", "]"), ("(", ")")];
    let mut text = s.to_string();
    let mut circle = false;
    // An id may come before the shape: `root((text))`.
    for (open, close) in shapes {
        if let Some(start) = s.find(open) {
            if s.ends_with(close) && start + open.len() <= s.len() - close.len() {
                text = s[start + open.len()..s.len() - close.len()].to_string();
                circle = open == "((";
                break;
            }
        }
    }
    let text = text.trim().trim_matches('"').trim_matches('`').to_string();
    let lines = wrap(&text, if depth == 0 { 180.0 } else { 220.0 });
    let (tw, th) = lines_size(&lines);
    let (w, h) = match depth { 0 if circle => { let d = tw.max(th) + 28.0; (d, d) } 0 | 1 => (tw + 24.0, th + 14.0), _ => (tw + 8.0, th + 8.0) };
    Node { lines, w, h, circle: circle && depth == 0, children: Vec::new() }
}

pub fn mindmap(body: &[&str]) -> Option<Diagram> {
    // Each line's indentation says whose child it is.
    let mut stack: Vec<(usize, Node)> = Vec::new();
    let mut root: Option<Node> = None;
    let close = |stack: &mut Vec<(usize, Node)>, root: &mut Option<Node>| {
        let (_, done) = stack.pop().unwrap();
        match stack.last_mut() { Some((_, parent)) => parent.children.push(done), None => *root = Some(done) }
    };
    for raw in body {
        let indent = raw.chars().take_while(|c| c.is_whitespace()).count();
        let text = raw.trim();
        if text.is_empty() || text.starts_with("::icon") || text.starts_with("%%") { continue; }
        if root.is_some() && stack.is_empty() { break; }
        while stack.last().is_some_and(|(i, _)| *i >= indent) { close(&mut stack, &mut root); }
        if root.is_some() { break; }
        let depth = stack.len();
        stack.push((indent, node(text, depth)));
    }
    while !stack.is_empty() { close(&mut stack, &mut root); }
    let mut root = root?;
    // The branches shared between the sides by how much each holds, the larger half to the right.
    let weight = |n: &Node| -> usize { fn leaves(n: &Node) -> usize { if n.children.is_empty() { 1 } else { n.children.iter().map(leaves).sum() } } leaves(n) };
    let children = std::mem::take(&mut root.children);
    // The first branches to the right, the rest to the left, cut where the two sides hold most nearly the same.
    let weights: Vec<usize> = children.iter().map(weight).collect();
    let total: usize = weights.iter().sum();
    let cut = (1..=children.len()).min_by_key(|&k| { let r: usize = weights[..k].iter().sum(); (r as i64 * 2 - total as i64).unsigned_abs() }).unwrap_or(0);
    let mut right = children;
    let left = right.split_off(cut);
    let mut items = Vec::new();
    let mut bounds = Bounds::default();
    let (cx, cy) = (0.0, 0.0);
    draw_node(&mut items, &mut bounds, &root, cx, cy, 0);
    for (side, branches) in [(1.0f32, &right), (-1.0f32, &left)] {
        let tall: f32 = branches.iter().map(height).sum::<f32>() + GAP * branches.len().saturating_sub(1) as f32;
        let mut y = cy - tall / 2.0;
        for b in branches.iter() {
            let h = height(b);
            let mid = y + h / 2.0;
            let x = cx + side * (root.w / 2.0 + 48.0 + b.w / 2.0);
            edge(&mut items, cx + side * root.w / 2.0, cy, x - side * b.w / 2.0, mid, 2.0);
            place(&mut items, &mut bounds, b, x, mid, side, 1);
            y += h + GAP;
        }
    }
    Some(finish(items, bounds))
}

const GAP: f32 = 10.0;

/// How tall a subtree stands: the node, or its children one above another, whichever is taller.
fn height(n: &Node) -> f32 {
    let children: f32 = n.children.iter().map(height).sum::<f32>() + GAP * n.children.len().saturating_sub(1) as f32;
    n.h.max(children)
}

/// A node centred on (x, y) and its children beside it, away from the root on `side`.
fn place(items: &mut Vec<Item>, bounds: &mut Bounds, n: &Node, x: f32, y: f32, side: f32, depth: usize) {
    draw_node(items, bounds, n, x, y, depth);
    let tall: f32 = n.children.iter().map(height).sum::<f32>() + GAP * n.children.len().saturating_sub(1) as f32;
    let mut top = y - tall / 2.0;
    // A box's edge leaves from its side; words on a line, from the end of the line.
    let from = (x + side * n.w / 2.0, if depth == 1 { y } else { y + n.h / 2.0 });
    for c in &n.children {
        let h = height(c);
        let mid = top + h / 2.0;
        let cx = x + side * (n.w / 2.0 + 36.0 + c.w / 2.0);
        edge(items, from.0, from.1, cx - side * c.w / 2.0, mid + c.h / 2.0, if depth == 1 { 1.6 } else { 1.2 });
        place(items, bounds, c, cx, mid, side, depth + 1);
        top += h + GAP;
    }
}

fn draw_node(items: &mut Vec<Item>, bounds: &mut Bounds, n: &Node, x: f32, y: f32, depth: usize) {
    let (x0, y0) = (x - n.w / 2.0, y - n.h / 2.0);
    match depth {
        0 if n.circle => items.push(dot(x, y, n.w / 2.0, Fill::Soft)),
        0 => items.push(Item::Path { d: rect(x0, y0, n.w, n.h, n.h / 2.0), fill: Fill::Soft, stroke: 2.0, dash: false }),
        1 => items.push(Item::Path { d: rect(x0, y0, n.w, n.h, 8.0), fill: Fill::Soft, stroke: 1.6, dash: false }),
        _ => items.push(Item::Path { d: Pen::new().m(x0, y0 + n.h).l(x0 + n.w, y0 + n.h).done(), fill: Fill::None, stroke: 1.2, dash: false }),
    }
    label(items, &n.lines, x, if depth >= 2 { y - 1.0 } else { y }, depth <= 1);
    bounds.add(x0, y0); bounds.add(x0 + n.w, y0 + n.h);
}

fn edge(items: &mut Vec<Item>, x0: f32, y0: f32, x1: f32, y1: f32, width: f32) {
    let mid = (x0 + x1) / 2.0;
    items.push(Item::Path { d: Pen::new().m(x0, y0).c(mid, y0, mid, y1, x1, y1).done(), fill: Fill::None, stroke: width, dash: false });
}
