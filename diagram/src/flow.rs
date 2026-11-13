//! Flowcharts and state diagrams: read into nodes and edges, laid out in layers (Sugiyama's method: cycles broken,
//! ranks by longest path, crossings reduced by barycentres, positions balanced), drawn as paths and words.
use crate::draw::{self, arrowhead, cross, dot, label, lines_size, rect, wrap, Diagram, Fill, Item, Pen};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape { Rect, Round, Stadium, Subroutine, Cylinder, Circle, Diamond, Hexagon, Lean, Asym, Start, End }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Line { Solid, Dotted, Thick, Invisible }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Head { None, Arrow, Circle, Cross }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir { Down, Up, Right, Left }

struct Node { id: String, text: String, shape: Shape }
struct Edge { from: usize, to: usize, label: String, line: Line, head: Head, tail: Head }
struct Cluster { title: String, members: Vec<usize> }

#[derive(Default)]
struct Graph { nodes: Vec<Node>, edges: Vec<Edge>, clusters: Vec<Cluster>, index: HashMap<String, usize>, dir: Option<Dir>, open: Vec<usize> }

impl Graph {
    fn node(&mut self, id: &str, text: Option<String>, shape: Option<Shape>) -> usize {
        let at = match self.index.get(id) {
            Some(&at) => at,
            None => { self.nodes.push(Node { id: id.to_string(), text: id.to_string(), shape: Shape::Rect }); self.index.insert(id.to_string(), self.nodes.len() - 1); self.nodes.len() - 1 }
        };
        if let Some(text) = text { self.nodes[at].text = text; }
        if let Some(shape) = shape { self.nodes[at].shape = shape; }
        for &cluster in &self.open { if !self.clusters[cluster].members.contains(&at) { self.clusters[cluster].members.push(at); } }
        at
    }
}

fn direction(word: &str) -> Option<Dir> {
    match word.trim().to_ascii_uppercase().as_str() { "TD" | "TB" => Some(Dir::Down), "BT" => Some(Dir::Up), "LR" => Some(Dir::Right), "RL" => Some(Dir::Left), _ => None }
}

/// A flowchart (`graph` or `flowchart`), its header already taken off.
pub fn flowchart(header: &str, body: &[&str]) -> Option<Diagram> {
    let mut g = Graph { dir: direction(header), ..Graph::default() };
    for raw in body {
        for statement in raw.split(';') {
            let s = statement.trim();
            if s.is_empty() { continue; }
            let word = s.split_whitespace().next().unwrap_or("");
            match word {
                "subgraph" => {
                    let rest = s["subgraph".len()..].trim();
                    let title = match (rest.find('['), rest.rfind(']')) { (Some(a), Some(b)) if b > a => rest[a + 1..b].trim().trim_matches('"').to_string(), _ => rest.trim_matches('"').to_string() };
                    g.clusters.push(Cluster { title, members: Vec::new() });
                    g.open.push(g.clusters.len() - 1);
                }
                "end" => { g.open.pop(); }
                "direction" => { if g.open.is_empty() { g.dir = direction(&s["direction".len()..]).or(g.dir); } }
                "classDef" | "class" | "style" | "linkStyle" | "click" | "accTitle" | "accDescr" => {}
                _ => chain(&mut g, s),
            }
        }
    }
    (!g.nodes.is_empty()).then(|| layout(&g))
}

/// A state diagram (`stateDiagram`), its header already taken off: `[*]` is where it starts and ends.
pub fn states(body: &[&str]) -> Option<Diagram> {
    let mut g = Graph { dir: Some(Dir::Down), ..Graph::default() };
    let mut scope = 0usize;
    let mut scopes = vec![0usize];
    for raw in body {
        let s = raw.trim();
        if s.is_empty() || s.starts_with("note") || s.starts_with("classDef") || s.starts_with("class ") { continue; }
        if let Some(rest) = s.strip_prefix("direction") { g.dir = direction(rest).or(g.dir); continue; }
        if s == "}" { g.open.pop(); scopes.pop(); continue; }
        if let Some(rest) = s.strip_prefix("state ") {
            let rest = rest.trim();
            if let Some(name) = rest.strip_suffix('{') {
                let (title, id) = alias(name.trim());
                let at = g.node(&id, Some(title.clone()), None);
                g.nodes[at].shape = Shape::Round;
                g.clusters.push(Cluster { title, members: Vec::new() });
                g.open.push(g.clusters.len() - 1);
                scope += 1; scopes.push(scope);
                continue;
            }
            let (title, id) = alias(rest);
            let id = id.split("<<").next().unwrap_or("").trim().to_string();
            g.node(&id, Some(title), Some(Shape::Round));
            continue;
        }
        let here = *scopes.last().unwrap_or(&0);
        let point = |g: &mut Graph, name: &str, start: bool| -> usize {
            let name = name.trim();
            if name == "[*]" { let id = format!("[*]{}{here}", if start { "start" } else { "end" }); g.node(&id, Some(String::new()), Some(if start { Shape::Start } else { Shape::End })) }
            else { let at = g.node(name, None, None); if g.nodes[at].shape == Shape::Rect { g.nodes[at].shape = Shape::Round; } at }
        };
        if let Some(arrow) = s.find("-->") {
            let (from, rest) = (&s[..arrow], &s[arrow + 3..]);
            let (to, text) = match rest.find(':') { Some(c) => (&rest[..c], rest[c + 1..].trim().to_string()), None => (rest, String::new()) };
            let (a, b) = (point(&mut g, from, true), point(&mut g, to, false));
            g.edges.push(Edge { from: a, to: b, label: text, line: Line::Solid, head: Head::Arrow, tail: Head::None });
        } else if let Some(c) = s.find(':') {
            let at = point(&mut g, &s[..c], true);
            let text = s[c + 1..].trim();
            let node = &mut g.nodes[at];
            node.text = if node.text == node.id { text.to_string() } else { format!("{}\n{}", node.text, text) };
        } else {
            point(&mut g, s, true);
        }
    }
    (!g.nodes.is_empty()).then(|| layout(&g))
}

/// `"Long name" as id` or `id`.
fn alias(s: &str) -> (String, String) {
    match s.split_once(" as ") {
        Some((title, id)) => (title.trim().trim_matches('"').to_string(), id.trim().to_string()),
        None => (s.trim_matches('"').to_string(), s.trim_matches('"').to_string()),
    }
}

struct Cursor<'a> { s: &'a [char], at: usize }
impl Cursor<'_> {
    fn peek(&self) -> Option<char> { self.s.get(self.at).copied() }
    fn starts(&self, p: &str) -> bool { p.chars().enumerate().all(|(i, c)| self.s.get(self.at + i) == Some(&c)) }
    fn space(&mut self) { while self.peek().is_some_and(char::is_whitespace) { self.at += 1; } }
    fn rest(&self) -> String { self.s[self.at..].iter().collect() }
}

/// `A[text] --> B & C -- label --- D`: nodes joined by edges, as many as written.
fn chain(g: &mut Graph, s: &str) {
    let chars: Vec<char> = s.chars().collect();
    let mut c = Cursor { s: &chars, at: 0 };
    let Some(mut left) = group(g, &mut c) else { return };
    loop {
        c.space();
        let Some((line, head, tail, mut text)) = edge(&mut c) else { return };
        c.space();
        if c.peek() == Some('|') {
            c.at += 1;
            let start = c.at;
            while c.peek().is_some_and(|ch| ch != '|') { c.at += 1; }
            text = chars[start..c.at].iter().collect::<String>().trim().trim_matches('"').to_string();
            c.at += 1;
        }
        let Some(right) = group(g, &mut c) else { return };
        for &a in &left { for &b in &right { g.edges.push(Edge { from: a, to: b, label: text.clone(), line, head, tail }); } }
        left = right;
    }
}

fn group(g: &mut Graph, c: &mut Cursor) -> Option<Vec<usize>> {
    let mut out = vec![node(g, c)?];
    loop {
        c.space();
        if c.peek() == Some('&') { c.at += 1; c.space(); out.push(node(g, c)?); } else { return Some(out); }
    }
}

const SHAPES: &[(&str, &str, Shape)] = &[
    ("([", "])", Shape::Stadium), ("[[", "]]", Shape::Subroutine), ("[(", ")]", Shape::Cylinder), ("((", "))", Shape::Circle), ("{{", "}}", Shape::Hexagon),
    ("[/", "/]", Shape::Lean), ("[\\", "\\]", Shape::Lean), ("[/", "\\]", Shape::Lean), ("[\\", "/]", Shape::Lean),
    ("[", "]", Shape::Rect), ("(", ")", Shape::Round), ("{", "}", Shape::Diamond), (">", "]", Shape::Asym),
];

fn node(g: &mut Graph, c: &mut Cursor) -> Option<usize> {
    c.space();
    let start = c.at;
    while let Some(ch) = c.peek() {
        let joins = ch.is_alphanumeric() || ch == '_' || (!ch.is_ascii() && !ch.is_whitespace())
            || (ch == '-' && c.s.get(c.at + 1).is_some_and(|n| n.is_alphanumeric()) && c.at > start)
            || (ch == '.' && c.s.get(c.at + 1).is_some_and(|n| n.is_alphanumeric()) && c.at > start);
        if !joins { break; }
        c.at += 1;
    }
    if c.at == start { return None; }
    let id: String = c.s[start..c.at].iter().collect();
    let mut text = None;
    let mut shape = None;
    for (open, close, kind) in SHAPES {
        if c.starts(open) {
            let save = c.at;
            c.at += open.chars().count();
            let body_start = c.at;
            let quoted = c.peek() == Some('"');
            if quoted {
                c.at += 1;
                while c.peek().is_some_and(|ch| ch != '"') { c.at += 1; }
                c.at += 1;
            }
            while c.at < c.s.len() && !c.starts(close) { c.at += 1; }
            if c.at >= c.s.len() { c.at = save; continue; }
            let raw: String = c.s[body_start..c.at].iter().collect();
            c.at += close.chars().count();
            text = Some(raw.trim().trim_matches('"').to_string());
            shape = Some(*kind);
            break;
        }
    }
    if c.starts(":::") { c.at += 3; while c.peek().is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-') { c.at += 1; } }
    Some(g.node(&id, text, shape))
}

/// An edge: `-->`, `---`, `-.->`, `==>`, `--o`, `--x`, `<-->`, `~~~`, or one with its label inside (`-- text -->`).
fn edge(c: &mut Cursor) -> Option<(Line, Head, Head, String)> {
    let save = c.at;
    let mut tail = Head::None;
    match c.peek() {
        Some('<') => { tail = Head::Arrow; c.at += 1; }
        Some('o') if c.starts("o--") || c.starts("o==") => { tail = Head::Circle; c.at += 1; }
        Some('x') if c.starts("x--") || c.starts("x==") => { tail = Head::Cross; c.at += 1; }
        _ => {}
    }
    let body = |c: &mut Cursor| -> String { let start = c.at; while c.peek().is_some_and(|ch| matches!(ch, '-' | '=' | '.' | '~')) { c.at += 1; } c.s[start..c.at].iter().collect() };
    let head = |c: &mut Cursor| -> Head {
        match c.peek() {
            Some('>') => { c.at += 1; Head::Arrow }
            Some('o') if c.s.get(c.at + 1).is_none_or(|n| n.is_whitespace()) => { c.at += 1; Head::Circle }
            Some('x') if c.s.get(c.at + 1).is_none_or(|n| n.is_whitespace()) => { c.at += 1; Head::Cross }
            _ => Head::None,
        }
    };
    let style = |b: &str| if b.contains('~') { Line::Invisible } else if b.contains('.') { Line::Dotted } else if b.contains('=') { Line::Thick } else { Line::Solid };
    let first = body(c);
    if first.len() < 2 && !(first == "-" && c.peek() == Some('.')) { c.at = save; return None; }
    let h = head(c);
    if h != Head::None || first.len() >= 3 || first.contains('.') && first.len() >= 3 {
        return Some((style(&first), h, tail, String::new()));
    }
    // `-- text -->`: the label runs to the next stretch of dashes.
    let rest = c.rest();
    let ends = ["-->", "---", "==>", "===", ".->", "-.-", "--o", "--x"];
    let found = ends.iter().filter_map(|e| rest.find(e).map(|i| (i, *e))).min();
    let Some((i, _)) = found else { c.at = save; return None };
    let text = rest[..i].trim().trim_matches('"').to_string();
    c.at += rest[..i].chars().count();
    let second = body(c);
    let h = head(c);
    Some((style(&format!("{first}{second}")), h, tail, text))
}

struct Placed { lines: Vec<String>, w: f32, h: f32 }

fn measure(n: &Node) -> Placed {
    let lines = if matches!(n.shape, Shape::Start | Shape::End) { Vec::new() } else { wrap(&n.text, 200.0) };
    let (tw, th) = lines_size(&lines);
    let (w, h) = match n.shape {
        Shape::Start | Shape::End => (18.0, 18.0),
        Shape::Circle => { let d = tw.max(th) + 24.0; (d, d) }
        Shape::Diamond => { let w = tw + 1.6 * th + 24.0; (w, (th + 0.5 * tw + 18.0).min(w)) }
        Shape::Hexagon => (tw + th + 32.0, th + 18.0),
        Shape::Lean | Shape::Asym => (tw + th + 28.0, th + 18.0),
        Shape::Cylinder => (tw + 28.0, th + 30.0),
        Shape::Stadium => (tw + th + 24.0, th + 18.0),
        _ => (tw + 28.0, th + 18.0),
    };
    Placed { lines, w: w.max(if matches!(n.shape, Shape::Start | Shape::End) { 0.0 } else { 48.0 }), h }
}

/// A slot in a layer: a node, a point an edge passes through, or the label an edge carries.
#[derive(Clone)]
struct Slot { node: Option<usize>, along: f32, across: f32 }

fn layout(g: &Graph) -> Diagram {
    let dir = g.dir.unwrap_or(Dir::Down);
    let vertical = matches!(dir, Dir::Down | Dir::Up);
    let placed: Vec<Placed> = g.nodes.iter().map(measure).collect();
    let n = g.nodes.len();
    // Each edge between two nodes, cycles broken by turning back the edges that close them.
    let real: Vec<usize> = (0..g.edges.len()).filter(|&e| g.edges[e].from != g.edges[e].to).collect();
    let mut reversed = vec![false; g.edges.len()];
    {
        let mut state = vec![0u8; n];
        let out: Vec<Vec<usize>> = (0..n).map(|v| real.iter().copied().filter(|&e| g.edges[e].from == v).collect()).collect();
        fn visit(v: usize, g: &Graph, out: &[Vec<usize>], state: &mut [u8], reversed: &mut [bool]) {
            state[v] = 1;
            for &e in &out[v] {
                let w = g.edges[e].to;
                if state[w] == 1 { reversed[e] = true; } else if state[w] == 0 { visit(w, g, out, state, reversed); }
            }
            state[v] = 2;
        }
        for v in 0..n { if state[v] == 0 { visit(v, g, &out, &mut state, &mut reversed); } }
    }
    let ends = |e: usize| if reversed[e] { (g.edges[e].to, g.edges[e].from) } else { (g.edges[e].from, g.edges[e].to) };
    // Ranks by longest path, two apart, so an edge's label can sit in the rank between.
    let mut rank = vec![0usize; n];
    for _ in 0..n { let mut changed = false; for &e in &real { let (a, b) = ends(e); if rank[b] < rank[a] + 2 { rank[b] = rank[a] + 2; changed = true; } } if !changed { break; } }
    // A node nothing leads into sits just above what it leads to.
    for v in 0..n {
        if real.iter().any(|&e| ends(e).1 == v) { continue; }
        if let Some(min) = real.iter().filter(|&&e| ends(e).0 == v).map(|&e| rank[ends(e).1]).min() { rank[v] = min.saturating_sub(2).max(rank[v]); }
    }
    let ranks = rank.iter().max().copied().unwrap_or(0) + 1;
    let mut slots: Vec<Slot> = (0..n).map(|v| {
        let (w, h) = (placed[v].w, placed[v].h);
        // A node with a loop back into itself keeps room beside it for the loop.
        let looped = g.edges.iter().any(|e| e.from == v && e.to == v);
        let (w, h) = if looped { if vertical { (w + 80.0, h) } else { (w, h + 80.0) } } else { (w, h) };
        Slot { node: Some(v), along: if vertical { h } else { w }, across: if vertical { w } else { h } }
    }).collect();
    let mut layer_of: Vec<usize> = rank.clone();
    // Each edge as a chain of slots, one in every rank it crosses.
    let mut chains: Vec<Vec<usize>> = vec![Vec::new(); g.edges.len()];
    let mut label_slot: Vec<Option<usize>> = vec![None; g.edges.len()];
    let mut label_lines: Vec<Vec<String>> = vec![Vec::new(); g.edges.len()];
    for &e in &real {
        let (a, b) = ends(e);
        let mid = (rank[a] + rank[b]) / 2;
        let mut chain = vec![a];
        for r in rank[a] + 1..rank[b] {
            let (along, across) = if r == mid && !g.edges[e].label.is_empty() {
                let lines = wrap(&g.edges[e].label, 160.0);
                let (w, h) = lines_size(&lines);
                label_lines[e] = lines;
                label_slot[e] = Some(slots.len());
                if vertical { (h + 4.0, w + 8.0) } else { (w + 12.0, h + 2.0) }
            } else { (0.0, 0.0) };
            slots.push(Slot { node: None, along, across });
            layer_of.push(r);
            chain.push(slots.len() - 1);
        }
        chain.push(b);
        chains[e] = chain;
    }
    let pairs: Vec<(usize, usize)> = chains.iter().flat_map(|c| c.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()).collect();
    let up: Vec<Vec<usize>> = (0..slots.len()).map(|s| pairs.iter().filter(|p| p.1 == s).map(|p| p.0).collect()).collect();
    let down: Vec<Vec<usize>> = (0..slots.len()).map(|s| pairs.iter().filter(|p| p.0 == s).map(|p| p.1).collect()).collect();
    // First order: as written, nodes before the points edges pass through.
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); ranks];
    for (s, &r) in layer_of.iter().enumerate() { layers[r].push(s); }
    let crossings = |layers: &[Vec<usize>]| -> usize {
        let mut total = 0;
        let mut pos = vec![0usize; slots.len()];
        for layer in layers { for (i, &s) in layer.iter().enumerate() { pos[s] = i; } }
        for layer in layers {
            let edges: Vec<(usize, usize)> = layer.iter().flat_map(|&s| down[s].iter().map(move |&t| (s, t))).map(|(s, t)| (pos[s], pos[t])).collect();
            for i in 0..edges.len() { for j in i + 1..edges.len() { if (edges[i].0 < edges[j].0 && edges[i].1 > edges[j].1) || (edges[i].0 > edges[j].0 && edges[i].1 < edges[j].1) { total += 1; } } }
        }
        total
    };
    let mut best = layers.clone();
    let mut best_count = crossings(&layers);
    for round in 0..12 {
        let mut pos = vec![0f32; slots.len()];
        for layer in &layers { for (i, &s) in layer.iter().enumerate() { pos[s] = i as f32; } }
        let order: Vec<usize> = if round % 2 == 0 { (1..ranks).collect() } else { (0..ranks.saturating_sub(1)).rev().collect() };
        for r in order {
            let near = if round % 2 == 0 { &up } else { &down };
            let mut keyed: Vec<(f32, f32, usize)> = layers[r].iter().map(|&s| {
                let ns = &near[s];
                let bary = if ns.is_empty() { pos[s] } else { ns.iter().map(|&t| pos[t]).sum::<f32>() / ns.len() as f32 };
                (bary, pos[s], s)
            }).collect();
            keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
            layers[r] = keyed.into_iter().map(|k| k.2).collect();
            for (i, &s) in layers[r].iter().enumerate() { pos[s] = i as f32; }
        }
        let count = crossings(&layers);
        if count < best_count { best_count = count; best = layers.clone(); }
        if best_count == 0 { break; }
    }
    let layers = best;
    // Positions across: each slot drawn towards what it joins, never closer to its neighbour than they both allow.
    let gap = |a: usize, b: usize| match (slots[a].node.is_some() || slots[a].across > 0.0, slots[b].node.is_some() || slots[b].across > 0.0) { (true, true) => 28.0, (false, false) => 10.0, _ => 16.0 };
    let mut x = vec![0f32; slots.len()];
    for layer in &layers { let mut at = 0.0; for (i, &s) in layer.iter().enumerate() { if i > 0 { at += (slots[layer[i - 1]].across + slots[s].across) / 2.0 + gap(layer[i - 1], s); } x[s] = at; } }
    for round in 0..24 {
        let order: Vec<usize> = if round % 2 == 0 { (0..ranks).collect() } else { (0..ranks).rev().collect() };
        for r in order {
            let layer = &layers[r];
            let desired: Vec<f32> = layer.iter().map(|&s| {
                let mut sum = 0.0; let mut weight = 0.0;
                for &t in up[s].iter().chain(down[s].iter()) {
                    let w = match (slots[s].node.is_some(), slots[t].node.is_some()) { (false, false) => 8.0, (true, true) => 1.0, _ => 2.0 };
                    sum += x[t] * w; weight += w;
                }
                if weight > 0.0 { sum / weight } else { x[s] }
            }).collect();
            let k = layer.len();
            let sep = |i: usize| (slots[layer[i - 1]].across + slots[layer[i]].across) / 2.0 + gap(layer[i - 1], layer[i]);
            let mut left = desired.clone();
            for i in 1..k { left[i] = left[i].max(left[i - 1] + sep(i)); }
            let mut right = desired.clone();
            for i in (0..k.saturating_sub(1)).rev() { right[i] = right[i].min(right[i + 1] - sep(i + 1)); }
            for i in 0..k { x[layer[i]] = (left[i] + right[i]) / 2.0; }
        }
    }
    // Room between ranks for the clusters that end above or begin below: their padding, and a title on top.
    let mut room = vec![0f32; ranks];
    for cluster in g.clusters.iter().filter(|c| !c.members.is_empty()) {
        let first = cluster.members.iter().map(|&m| rank[m]).min().unwrap_or(0);
        let last = cluster.members.iter().map(|&m| rank[m]).max().unwrap_or(0);
        let title = if vertical && !cluster.title.is_empty() { lines_size(&wrap(&cluster.title, 240.0)).1 + 4.0 } else { 0.0 };
        if first > 0 { room[first] += 14.0 + title; }
        if last + 1 < ranks { room[last + 1] += 14.0; }
    }
    // Positions along: each rank as deep as its deepest slot.
    let mut y = vec![0f32; slots.len()];
    let mut at = 0.0;
    for (r, layer) in layers.iter().enumerate() {
        let depth = layer.iter().map(|&s| slots[s].along).fold(0.0, f32::max);
        if r > 0 { at += 20.0 + room[r]; }
        for &s in layer { y[s] = at + depth / 2.0; }
        at += depth;
    }
    let total_along = at;
    // Into the page: flipped for upward and leftward, turned for sideways.
    let point = |across: f32, along: f32| -> (f32, f32) {
        let along = if matches!(dir, Dir::Up | Dir::Left) { total_along - along } else { along };
        if vertical { (across, along) } else { (along, across) }
    };
    let mut items = Vec::new();
    let mut bounds = Bounds::default();
    // Clusters first, behind what they hold.
    for cluster in &g.clusters {
        if cluster.members.is_empty() { continue; }
        let mut b = Bounds::default();
        for &m in &cluster.members { let (cx, cy) = point(x[m], y[m]); b.add(cx - placed[m].w / 2.0, cy - placed[m].h / 2.0); b.add(cx + placed[m].w / 2.0, cy + placed[m].h / 2.0); }
        let lines = wrap(&cluster.title, 240.0);
        let (tw, th) = lines_size(&lines);
        let (x0, y0, x1, y1) = (b.x0 - 14.0, b.y0 - 14.0 - if lines.iter().any(|l| !l.is_empty()) { th + 4.0 } else { 0.0 }, b.x1.max(b.x0 + tw) + 14.0, b.y1 + 14.0);
        items.push(Item::Path { d: rect(x0, y0, x1 - x0, y1 - y0, 8.0), fill: Fill::Soft, stroke: 1.0, dash: true });
        // The title in the corner, clear of the edges that come in through the middle.
        label(&mut items, &lines, x0 + 10.0 + tw / 2.0, y0 + 8.0 + th / 2.0, true);
        bounds.add(x0, y0); bounds.add(x1, y1);
    }
    // Edges, each a curve from slot to slot that leaves and arrives square to the ranks.
    for &e in &real {
        let edge = &g.edges[e];
        let chain = &chains[e];
        if edge.line == Line::Invisible { continue; }
        // Through a point an edge merely passes, it runs along the line from the slot before to the slot after.
        let slope = |i: usize| -> f32 {
            let s = chain[i];
            if i == 0 || i + 1 == chain.len() || slots[s].along > 0.0 { return 0.0; }
            let (p, q) = (chain[i - 1], chain[i + 1]);
            (x[q] - x[p]) / (y[q] - y[p]).max(1.0)
        };
        let mut d = Pen::new();
        let mut last = (0.0, 0.0, 0.0, 0.0);
        for i in 0..chain.len() - 1 {
            let (a, b) = (chain[i], chain[i + 1]);
            let (ax, ay) = (x[a], y[a] + slots[a].along / 2.0);
            let (bx, by) = (x[b], y[b] - slots[b].along / 2.0);
            let half = (by - ay) / 2.0;
            let (p0, p1, p2, p3) = (point(ax, ay), point(ax + slope(i) * half, ay + half), point(bx - slope(i + 1) * half, by - half), point(bx, by));
            if i == 0 || slots[a].along > 0.0 { d = d.m(p0.0, p0.1); }
            d = d.c(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
            last = (p2.0, p2.1, p3.0, p3.1);
        }
        let path = d.done();
        let width = match edge.line { Line::Thick => 2.6, _ => 1.4 };
        items.push(Item::Path { d: path, fill: Fill::None, stroke: width, dash: edge.line == Line::Dotted });
        // Heads at either end, as written: an edge turned back for layout still points where it was written to.
        let (head_end, tail_end) = if reversed[e] { (edge.tail, edge.head) } else { (edge.head, edge.tail) };
        let start = { let (a, b) = (chain[0], chain[1]); let (ax, ay) = (x[a], y[a] + slots[a].along / 2.0); let by = y[b] - slots[b].along / 2.0; (point(ax, ay), point(ax, (ay + by) / 2.0)) };
        ends_mark(&mut items, head_end, last.2, last.3, last.2 - last.0, last.3 - last.1);
        ends_mark(&mut items, tail_end, start.0 .0, start.0 .1, start.0 .0 - start.1 .0, start.0 .1 - start.1 .1);
        if let Some(s) = label_slot[e] { let (lx, ly) = point(x[s], y[s]); label(&mut items, &label_lines[e], lx, ly, false); }
    }
    // Loops back into the node they leave.
    for edge in g.edges.iter().filter(|edge| edge.from == edge.to) {
        let v = edge.from;
        let (cx, cy) = point(x[v], y[v]);
        let (w, h) = (placed[v].w, placed[v].h);
        let (sx, sy, ex, ey, c1, c2) = if vertical {
            (cx + w / 2.0, cy - h / 4.0, cx + w / 2.0, cy + h / 4.0, (cx + w / 2.0 + 36.0, cy - h / 2.0 - 8.0), (cx + w / 2.0 + 36.0, cy + h / 2.0 + 8.0))
        } else {
            (cx - w / 4.0, cy + h / 2.0, cx + w / 4.0, cy + h / 2.0, (cx - w / 2.0 - 8.0, cy + h / 2.0 + 36.0), (cx + w / 2.0 + 8.0, cy + h / 2.0 + 36.0))
        };
        items.push(Item::Path { d: Pen::new().m(sx, sy).c(c1.0, c1.1, c2.0, c2.1, ex, ey).done(), fill: Fill::None, stroke: 1.4, dash: edge.line == Line::Dotted });
        ends_mark(&mut items, edge.head, ex, ey, ex - c2.0, ey - c2.1);
        let lines = wrap(&edge.label, 160.0);
        let (lw, _) = lines_size(&lines);
        let (lx, ly) = if vertical { (cx + w / 2.0 + 34.0 + lw / 2.0, cy) } else { (cx, cy + h / 2.0 + 40.0) };
        label(&mut items, &lines, lx, ly, false);
        bounds.add(lx + lw / 2.0 + 4.0, ly); bounds.add(c1.0, c1.1); bounds.add(c2.0, c2.1);
    }
    // Nodes over the edges.
    for v in 0..n {
        let (cx, cy) = point(x[v], y[v]);
        let p = &placed[v];
        shape(&mut items, g.nodes[v].shape, cx, cy, p.w, p.h);
        label(&mut items, &p.lines, cx, cy + if g.nodes[v].shape == Shape::Cylinder { 5.0 } else { 0.0 }, false);
        bounds.add(cx - p.w / 2.0, cy - p.h / 2.0); bounds.add(cx + p.w / 2.0, cy + p.h / 2.0);
    }
    for s in label_slot.iter().flatten() { let (lx, ly) = point(x[*s], y[*s]); let (w, h) = if vertical { (slots[*s].across, slots[*s].along) } else { (slots[*s].along, slots[*s].across) }; bounds.add(lx - w / 2.0, ly - h / 2.0); bounds.add(lx + w / 2.0, ly + h / 2.0); }
    finish(items, bounds)
}

fn ends_mark(items: &mut Vec<Item>, head: Head, x: f32, y: f32, dx: f32, dy: f32) {
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    match head {
        Head::None => {}
        Head::Arrow => items.push(arrowhead(x, y, dx, dy)),
        Head::Circle => items.push(dot(x - dx / len * 4.0, y - dy / len * 4.0, 4.0, Fill::Solid)),
        Head::Cross => items.push(cross(x - dx / len * 6.0, y - dy / len * 6.0)),
    }
}

fn shape(items: &mut Vec<Item>, shape: Shape, cx: f32, cy: f32, w: f32, h: f32) {
    let (x0, y0, x1, y1) = (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
    let path = |d: String| Item::Path { d, fill: Fill::Soft, stroke: 1.4, dash: false };
    match shape {
        Shape::Rect => items.push(path(rect(x0, y0, w, h, 3.0))),
        Shape::Round => items.push(path(rect(x0, y0, w, h, 10.0))),
        Shape::Stadium => items.push(path(rect(x0, y0, w, h, h / 2.0))),
        Shape::Subroutine => {
            items.push(path(rect(x0, y0, w, h, 0.0)));
            items.push(Item::Path { d: Pen::new().m(x0 + 8.0, y0).l(x0 + 8.0, y1).m(x1 - 8.0, y0).l(x1 - 8.0, y1).done(), fill: Fill::None, stroke: 1.4, dash: false });
        }
        Shape::Cylinder => {
            let ry = 7.0;
            items.push(path(Pen::new().m(x0, y0 + ry).a(w / 2.0, ry, true, x1, y0 + ry).l(x1, y1 - ry).a(w / 2.0, ry, true, x0, y1 - ry).z()));
            items.push(Item::Path { d: Pen::new().m(x0, y0 + ry).a(w / 2.0, ry, false, x1, y0 + ry).done(), fill: Fill::None, stroke: 1.4, dash: false });
        }
        Shape::Circle => items.push(dot(cx, cy, w / 2.0, Fill::Soft)),
        Shape::Diamond => items.push(path(Pen::new().m(cx, y0).l(x1, cy).l(cx, y1).l(x0, cy).z())),
        Shape::Hexagon => { let k = h / 2.0; items.push(path(Pen::new().m(x0 + k, y0).l(x1 - k, y0).l(x1, cy).l(x1 - k, y1).l(x0 + k, y1).l(x0, cy).z())); }
        Shape::Lean => { let k = h / 2.0; items.push(path(Pen::new().m(x0 + k, y0).l(x1, y0).l(x1 - k, y1).l(x0, y1).z())); }
        Shape::Asym => { let k = h / 2.0; items.push(path(Pen::new().m(x0, y0).l(x1, y0).l(x1, y1).l(x0, y1).l(x0 + k, cy).z())); }
        Shape::Start => items.push(dot(cx, cy, 8.0, Fill::Solid)),
        Shape::End => { items.push(dot(cx, cy, 9.0, Fill::None)); items.push(dot(cx, cy, 5.5, Fill::Solid)); }
    }
}

#[derive(Default)]
pub(crate) struct Bounds { pub x0: f32, pub y0: f32, pub x1: f32, pub y1: f32, any: bool }
impl Bounds {
    pub fn add(&mut self, x: f32, y: f32) {
        if !self.any { *self = Bounds { x0: x, y0: y, x1: x, y1: y, any: true }; return; }
        self.x0 = self.x0.min(x); self.y0 = self.y0.min(y); self.x1 = self.x1.max(x); self.y1 = self.y1.max(y);
    }
}

/// The diagram as drawn, with a small margin around what it holds; its items keep their places, the view moves.
pub(crate) fn finish(items: Vec<Item>, b: Bounds) -> Diagram {
    let margin = 4.0;
    Diagram { x: draw::r(b.x0 - margin), y: draw::r(b.y0 - margin), width: draw::r(b.x1 - b.x0 + 2.0 * margin), height: draw::r(b.y1 - b.y0 + 2.0 * margin), items }
}
