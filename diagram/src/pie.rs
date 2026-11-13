//! Pie charts: each part a slice of the circle by its share, named outside it with its share, largest first from the
//! top, clockwise.
use crate::draw::{label, r, width, wrap, Diagram, Fill, Item, Pen, LINE, SIZE};
use crate::flow::{finish, Bounds};

pub fn pie(header: &str, body: &[&str]) -> Option<Diagram> {
    let show = header.split_whitespace().any(|w| w == "showData");
    let mut title = header.split_once("title").map(|(_, t)| t.trim().to_string()).filter(|t| !t.is_empty());
    let mut parts: Vec<(String, f64)> = Vec::new();
    for line in body {
        if let Some(rest) = line.strip_prefix("title ") { title = Some(rest.trim().to_string()); continue; }
        let Some((name, value)) = line.rsplit_once(':') else { continue };
        let Ok(value) = value.trim().parse::<f64>() else { continue };
        let name = name.trim().trim_matches('"').trim().to_string();
        if value > 0.0 && !name.is_empty() { parts.push((name, value)); }
    }
    let total: f64 = parts.iter().map(|p| p.1).sum();
    if parts.is_empty() || total <= 0.0 { return None; }

    let mut items = Vec::new();
    let mut b = Bounds::default();
    let radius = 90.0_f32;
    let (cx, mut cy) = (0.0_f32, 0.0_f32);
    if let Some(title) = &title {
        let lines = wrap(title, 320.0);
        label(&mut items, &lines, cx, -radius - 44.0 - LINE * (lines.len() as f32 - 1.0) / 2.0, true);
        b.add(cx - width(title, SIZE).min(320.0) / 2.0, -radius - 44.0 - LINE * lines.len() as f32);
        cy = 0.0;
    }
    let mut angle = -std::f64::consts::FRAC_PI_2;
    for (name, value) in &parts {
        let share = value / total;
        let sweep = share * std::f64::consts::TAU;
        let (a0, a1) = (angle, angle + sweep);
        let point = |a: f64, k: f32| (cx + k * a.cos() as f32, cy + k * a.sin() as f32);
        let (x0, y0) = point(a0, radius);
        let (x1, y1) = point(a1, radius);
        let d = if parts.len() == 1 {
            Pen::new().m(cx - radius, cy).a(radius, radius, true, cx + radius, cy).a(radius, radius, true, cx - radius, cy).z()
        } else {
            let large = sweep > std::f64::consts::PI;
            let mut pen = Pen::new().m(cx, cy).l(x0, y0);
            pen.0.push_str(&format!("A{} {} 0 {} 1 {} {}", r(radius), r(radius), u8::from(large), r(x1), r(y1)));
            pen.z()
        };
        items.push(Item::Path { d, fill: Fill::Soft, stroke: 1.4, dash: false });
        // Its name outside, on a short line from the middle of its arc.
        let mid = a0 + sweep / 2.0;
        let (ex, ey) = point(mid, radius);
        let (lx, ly) = point(mid, radius + 18.0);
        let right = mid.cos() >= 0.0;
        let tx = lx + if right { 10.0 } else { -10.0 };
        items.push(Item::Path { d: Pen::new().m(ex, ey).l(lx, ly).l(tx, ly).done(), fill: Fill::None, stroke: 1.0, dash: false });
        let percent = share * 100.0;
        let said = if show { format!("{name} {} ({percent:.1}%)", trim(*value)) } else { format!("{name} {percent:.1}%") };
        let w = width(&said, SIZE);
        let x = if right { tx + 4.0 + w / 2.0 } else { tx - 4.0 - w / 2.0 };
        items.push(Item::Text { x: r(x), y: r(ly), text: said, size: SIZE, bold: false });
        b.add(x - w / 2.0, ly - LINE / 2.0); b.add(x + w / 2.0, ly + LINE / 2.0);
        angle = a1;
    }
    b.add(cx - radius - 2.0, cy - radius - 2.0); b.add(cx + radius + 2.0, cy + radius + 2.0);
    Some(finish(items, b))
}

/// A number as written, without a trailing .0.
fn trim(v: f64) -> String { if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") } }
