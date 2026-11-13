//! Sequence diagrams: participants side by side, messages down the page in the order written, notes and frames
//! (loop, alt, opt, par, critical, break) around what they hold.
use crate::draw::{arrowhead, cross, label, lines_size, open_head, rect, wrap, Diagram, Fill, Item, Pen, LINE};
use crate::flow::{finish, Bounds};

struct Participant { id: String, lines: Vec<String>, w: f32, actor: bool }

enum Event {
    Message { from: usize, to: usize, lines: Vec<String>, dash: bool, head: Head },
    Note { from: usize, to: usize, side: Side, lines: Vec<String> },
    Open { kind: String, lines: Vec<String> },
    Divide { lines: Vec<String> },
    Close,
}

#[derive(Clone, Copy, PartialEq)]
enum Head { Arrow, Open, Cross, None }
#[derive(Clone, Copy, PartialEq)]
enum Side { Left, Right, Over }

const ARROWS: &[(&str, bool, Head)] = &[
    ("-->>", true, Head::Arrow), ("->>", false, Head::Arrow), ("--x", true, Head::Cross), ("-x", false, Head::Cross),
    ("--)", true, Head::Open), ("-)", false, Head::Open), ("-->", true, Head::None), ("->", false, Head::None),
];

pub fn sequence(body: &[&str]) -> Option<Diagram> {
    let mut people: Vec<Participant> = Vec::new();
    let mut events = Vec::new();
    let mut number: Option<usize> = None;
    fn who(people: &mut Vec<Participant>, id: &str, name: Option<&str>, actor: bool) -> usize {
        let id = id.trim();
        if let Some(at) = people.iter().position(|p| p.id == id) {
            if let Some(name) = name { let lines = wrap(name, 160.0); people[at].w = lines_size(&lines).0; people[at].lines = lines; }
            people[at].actor |= actor;
            return at;
        }
        let lines = wrap(name.unwrap_or(id), 160.0);
        let w = lines_size(&lines).0;
        people.push(Participant { id: id.to_string(), lines, w, actor });
        people.len() - 1
    }
    for raw in body {
        let s = raw.trim();
        if s.is_empty() { continue; }
        let word = s.split_whitespace().next().unwrap_or("");
        let rest = s[word.len()..].trim();
        match word {
            "participant" | "actor" => {
                let (id, name) = match rest.split_once(" as ") { Some((id, name)) => (id, Some(name.trim())), None => (rest, None) };
                who(&mut people, id, name, word == "actor");
            }
            "autonumber" => number = Some(1),
            "loop" | "alt" | "opt" | "par" | "critical" | "break" | "rect" => events.push(Event::Open { kind: word.to_string(), lines: if word == "rect" { Vec::new() } else { wrap(rest, 240.0) } }),
            "else" | "and" | "option" => events.push(Event::Divide { lines: wrap(rest, 240.0) }),
            "end" => events.push(Event::Close),
            "activate" | "deactivate" | "title" | "box" | "create" | "destroy" | "links" | "link" => {}
            _ if word.eq_ignore_ascii_case("note") => {
                let Some((place, text)) = rest.split_once(':') else { continue };
                let place = place.trim();
                let (side, names) = if let Some(n) = place.strip_prefix("right of") { (Side::Right, n) } else if let Some(n) = place.strip_prefix("left of") { (Side::Left, n) } else if let Some(n) = place.strip_prefix("over") { (Side::Over, n) } else { continue };
                let mut ids = names.split(',').map(|n| who(&mut people, n, None, false));
                let from = ids.next().unwrap_or(0);
                let to = ids.next().unwrap_or(from);
                events.push(Event::Note { from: from.min(to), to: from.max(to), side, lines: wrap(text.trim(), 200.0) });
            }
            _ => {
                let Some((at, arrow, dash, head)) = ARROWS.iter().filter_map(|(a, dash, head)| s.find(a).map(|i| (i, *a, *dash, *head))).min_by_key(|(i, a, ..)| (*i, usize::MAX - a.len())) else { continue };
                let from = s[..at].trim();
                let after = &s[at + arrow.len()..];
                let (to, text) = match after.split_once(':') { Some((to, text)) => (to, text.trim()), None => (after, "") };
                let to = to.trim().trim_start_matches(['+', '-']);
                let (a, b) = (who(&mut people, from, None, false), who(&mut people, to, None, false));
                let text = match &mut number { Some(n) => { let t = format!("{n}. {text}"); *n += 1; t } None => text.to_string() };
                events.push(Event::Message { from: a, to: b, lines: wrap(&text, 220.0), dash, head });
            }
        }
    }
    if people.is_empty() { return None; }
    // Columns: each participant as wide as its name, each gap as wide as the messages and notes it carries.
    let k = people.len();
    let box_w: Vec<f32> = people.iter().map(|p| (p.w + 28.0).max(84.0)).collect();
    let mut gaps: Vec<f32> = (1..k).map(|i| (box_w[i - 1] + box_w[i]) / 2.0 + 36.0).collect();
    for _ in 0..3 {
        for e in &events {
            let (a, b, need) = match e {
                Event::Message { from, to, lines, .. } if from != to => (*from.min(to), *from.max(to), lines_size(lines).0 + 32.0),
                Event::Message { from, lines, .. } if *from + 1 < k => (*from, *from + 1, lines_size(lines).0 + 50.0),
                Event::Note { from, side: Side::Right, lines, .. } if *from + 1 < k => (*from, *from + 1, lines_size(lines).0 + 40.0 + box_w[*from + 1] / 2.0),
                Event::Note { from, side: Side::Left, lines, .. } if *from > 0 => (*from - 1, *from, lines_size(lines).0 + 40.0 + box_w[*from - 1] / 2.0),
                _ => continue,
            };
            let have: f32 = gaps[a..b].iter().sum();
            if have < need { let more = (need - have) / (b - a) as f32; for g in &mut gaps[a..b] { *g += more; } }
        }
    }
    let mut x = vec![box_w[0] / 2.0];
    for g in &gaps { x.push(x.last().unwrap() + g); }
    let names_h = people.iter().map(|p| lines_size(&p.lines).1).fold(0.0, f32::max);
    let head_h = if people.iter().any(|p| p.actor) { names_h + 36.0 } else { names_h + 16.0 };
    let mut items = Vec::new();
    let mut bounds = Bounds::default();
    let mut y = head_h + 18.0;
    struct Frame { kind: String, lines: Vec<String>, top: f32, dividers: Vec<(f32, Vec<String>)>, lo: usize, hi: usize, reach: (f32, f32) }
    let mut frames: Vec<Frame> = Vec::new();
    let mut drawn_frames: Vec<(Frame, f32, usize)> = Vec::new();
    let touch = |frames: &mut Vec<Frame>, a: usize, b: usize| for f in frames.iter_mut() { f.lo = f.lo.min(a); f.hi = f.hi.max(b); };
    // How far right or left what a frame holds reaches past its participants: a message to itself, a note.
    let reach = |frames: &mut Vec<Frame>, x0: f32, x1: f32| for f in frames.iter_mut() { f.reach = (f.reach.0.min(x0), f.reach.1.max(x1)); };
    for e in &events {
        match e {
            Event::Message { from, to, lines, dash, head } => {
                let (_, th) = lines_size(lines);
                touch(&mut frames, *from.min(to), *from.max(to));
                if from == to {
                    let x0 = x[*from];
                    label_left(&mut items, lines, x0 + 12.0, y + th / 2.0);
                    y += th + 4.0;
                    let d = Pen::new().m(x0, y).l(x0 + 36.0, y).l(x0 + 36.0, y + 22.0).l(x0 + 2.0, y + 22.0).done();
                    items.push(Item::Path { d, fill: Fill::None, stroke: 1.4, dash: *dash });
                    mark(&mut items, *head, x0 + 1.0, y + 22.0, -1.0, 0.0);
                    bounds.add(x0 + 40.0 + lines_size(lines).0, y);
                    reach(&mut frames, x0, x0 + 12.0 + lines_size(lines).0.max(36.0));
                    y += 22.0 + 18.0;
                } else {
                    let (x0, x1) = (x[*from], x[*to]);
                    label(&mut items, lines, (x0 + x1) / 2.0, y + th / 2.0, false);
                    y += th + 6.0;
                    let dir = (x1 - x0).signum();
                    items.push(Item::Path { d: Pen::new().m(x0, y).l(x1 - dir, y).done(), fill: Fill::None, stroke: 1.4, dash: *dash });
                    mark(&mut items, *head, x1 - dir, y, dir, 0.0);
                    y += 20.0;
                }
            }
            Event::Note { from, to, side, lines } => {
                let (tw, th) = lines_size(lines);
                let w = tw + 20.0;
                let h = th + 12.0;
                let (x0, x1) = match side {
                    Side::Right => (x[*from] + 10.0, x[*from] + 10.0 + w),
                    Side::Left => (x[*from] - 10.0 - w, x[*from] - 10.0),
                    Side::Over => { let (l, r) = (x[*from], x[*to]); let w = w.max(r - l + 40.0); let c = (l + r) / 2.0; (c - w / 2.0, c + w / 2.0) }
                };
                touch(&mut frames, *from, *to);
                reach(&mut frames, x0, x1);
                let fold = 8.0;
                items.push(Item::Path { d: Pen::new().m(x0, y).l(x1 - fold, y).l(x1, y + fold).l(x1, y + h).l(x0, y + h).z(), fill: Fill::Soft, stroke: 1.2, dash: false });
                items.push(Item::Path { d: Pen::new().m(x1 - fold, y).l(x1 - fold, y + fold).l(x1, y + fold).done(), fill: Fill::None, stroke: 1.2, dash: false });
                label(&mut items, lines, (x0 + x1) / 2.0, y + h / 2.0, false);
                bounds.add(x0, y); bounds.add(x1, y + h);
                y += h + 14.0;
            }
            Event::Open { kind, lines } => {
                frames.push(Frame { kind: kind.clone(), lines: lines.clone(), top: y, dividers: Vec::new(), lo: usize::MAX, hi: 0, reach: (f32::MAX, f32::MIN) });
                y += LINE + 16.0;
            }
            Event::Divide { lines } => {
                if let Some(f) = frames.last_mut() { f.dividers.push((y, lines.clone())); }
                y += LINE + 14.0;
            }
            Event::Close => {
                if let Some(f) = frames.pop() {
                    let depth = frames.len();
                    let (lo, hi) = (f.lo, f.hi);
                    if lo != usize::MAX { touch(&mut frames, lo, hi); }
                    let (r0, r1) = f.reach;
                    if r0 <= r1 { reach(&mut frames, r0 - 12.0, r1 + 12.0); }
                    drawn_frames.push((f, y, depth));
                    y += 12.0;
                }
            }
        }
    }
    // Frames, an unclosed one ending with the diagram; each spans the participants within it, nested ones inset.
    while let Some(f) = frames.pop() { let depth = frames.len(); drawn_frames.push((f, y, depth)); y += 12.0; }
    for (f, bottom, depth) in &drawn_frames {
        let (lo, hi) = if f.lo == usize::MAX { (0, k - 1) } else { (f.lo, f.hi) };
        let inset = 12.0 * (*depth as f32);
        let x0 = (x[lo] - box_w[lo] / 2.0 + inset - 6.0).min(f.reach.0 - 12.0);
        let x1 = (x[hi] + box_w[hi] / 2.0 - inset + 6.0).max(f.reach.1 + 12.0).max(x0 + 120.0);
        let top = f.top;
        let soft = if f.kind == "rect" { Fill::Soft } else { Fill::None };
        items.insert(0, Item::Path { d: rect(x0, top, x1 - x0, bottom - top + 4.0, 4.0), fill: soft, stroke: 1.0, dash: false });
        if f.kind != "rect" {
            let tag_w = crate::draw::width(&f.kind, 12.0) + 16.0;
            items.push(Item::Path { d: Pen::new().m(x0, top).l(x0 + tag_w, top).l(x0 + tag_w, top + 14.0).l(x0 + tag_w - 6.0, top + 20.0).l(x0, top + 20.0).z(), fill: Fill::Soft, stroke: 1.0, dash: false });
            items.push(Item::Text { x: x0 + tag_w / 2.0, y: top + 10.0, text: f.kind.clone(), size: 12.0, bold: true });
            if !f.lines.is_empty() { label(&mut items, &bracket(&f.lines), (x0 + tag_w + x1) / 2.0, top + 10.0 + (f.lines.len() as f32 - 1.0) * LINE / 2.0, false); }
            for (dy, lines) in &f.dividers {
                items.push(Item::Path { d: Pen::new().m(x0, *dy).l(x1, *dy).done(), fill: Fill::None, stroke: 1.0, dash: true });
                if !lines.is_empty() { label(&mut items, &bracket(lines), (x0 + x1) / 2.0, dy + 12.0, false); }
            }
        }
        bounds.add(x0, top); bounds.add(x1, *bottom);
    }
    // Lifelines behind everything, the participants at the top and again at the bottom.
    let bottom = y;
    for i in 0..k {
        items.insert(0, Item::Path { d: Pen::new().m(x[i], head_h).l(x[i], bottom).done(), fill: Fill::None, stroke: 1.0, dash: true });
        for top in [0.0, bottom] { person(&mut items, &people[i], x[i], top, box_w[i], head_h); }
        bounds.add(x[i] - box_w[i] / 2.0, 0.0); bounds.add(x[i] + box_w[i] / 2.0, bottom + head_h);
    }
    Some(finish(items, bounds))
}

fn bracket(lines: &[String]) -> Vec<String> {
    let mut out = lines.to_vec();
    if let Some(first) = out.first_mut() { *first = format!("[{first}"); }
    if let Some(last) = out.last_mut() { last.push(']'); }
    out
}

fn label_left(items: &mut Vec<Item>, lines: &[String], x: f32, y: f32) {
    let w = lines_size(lines).0;
    label(items, lines, x + w / 2.0, y, false);
}

fn person(items: &mut Vec<Item>, p: &Participant, cx: f32, top: f32, w: f32, h: f32) {
    if p.actor {
        // A figure, its name beneath.
        let (hx, hy) = (cx, top + 7.0);
        items.push(crate::draw::dot(hx, hy, 5.0, Fill::None));
        items.push(Item::Path { d: Pen::new().m(cx, top + 12.0).l(cx, top + 22.0).m(cx - 8.0, top + 15.0).l(cx + 8.0, top + 15.0).m(cx, top + 22.0).l(cx - 6.0, top + 29.0).m(cx, top + 22.0).l(cx + 6.0, top + 29.0).done(), fill: Fill::None, stroke: 1.4, dash: false });
        label(items, &p.lines, cx, top + 34.0 + (h - 34.0) / 2.0, false);
        return;
    }
    items.push(Item::Path { d: rect(cx - w / 2.0, top, w, h, 6.0), fill: Fill::Soft, stroke: 1.4, dash: false });
    label(items, &p.lines, cx, top + h / 2.0, false);
}

fn mark(items: &mut Vec<Item>, head: Head, x: f32, y: f32, dx: f32, dy: f32) {
    match head {
        Head::Arrow => items.push(arrowhead(x, y, dx, dy)),
        Head::Open => items.push(open_head(x, y, dx, dy)),
        Head::Cross => items.push(cross(x - dx * 6.0, y - dy * 6.0)),
        Head::None => {}
    }
}
