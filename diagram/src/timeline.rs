//! Timelines: periods along a line, left to right, each with what happened in it below; sections band periods together.
use crate::draw::{label, lines_size, rect, wrap, Diagram, Fill, Item, Pen, LINE};
use crate::flow::{finish, Bounds};

struct Period { section: Option<usize>, name: Vec<String>, events: Vec<Vec<String>> }

pub fn timeline(body: &[&str]) -> Option<Diagram> {
    let mut title = None;
    let mut sections: Vec<String> = Vec::new();
    let mut periods: Vec<Period> = Vec::new();
    for line in body {
        if let Some(rest) = line.strip_prefix("title ") { title = Some(rest.trim().to_string()); continue; }
        if let Some(rest) = line.strip_prefix("section ") { sections.push(rest.trim().to_string()); continue; }
        let mut parts = line.split(" : ").map(str::trim).flat_map(|p| p.split(':').map(str::trim)).collect::<Vec<_>>();
        // A line that begins with `:` adds events to the period before it.
        if line.trim_start().starts_with(':') {
            parts.retain(|p| !p.is_empty());
            let period = periods.last_mut()?;
            period.events.extend(parts.iter().map(|e| wrap(e, 140.0)));
            continue;
        }
        let name = parts.remove(0);
        if name.is_empty() { continue; }
        periods.push(Period { section: sections.len().checked_sub(1), name: wrap(name, 120.0), events: parts.iter().filter(|e| !e.is_empty()).map(|e| wrap(e, 140.0)).collect() });
    }
    if periods.is_empty() { return None; }

    let mut items = Vec::new();
    let mut b = Bounds::default();
    let gap = 18.0;
    let widths: Vec<f32> = periods.iter().map(|p| p.events.iter().map(|e| lines_size(e).0 + 20.0).chain([lines_size(&p.name).0 + 24.0]).fold(96.0, f32::max)).collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
    let mut y = 0.0;
    if let Some(title) = &title {
        let lines = wrap(title, total.max(240.0));
        label(&mut items, &lines, total / 2.0, LINE * lines.len() as f32 / 2.0, true);
        y += LINE * lines.len() as f32 + 12.0;
    }
    // Sections over the periods they hold.
    if !sections.is_empty() {
        let mut x = 0.0;
        let mut at = 0;
        while at < periods.len() {
            let section = periods[at].section;
            let start = x;
            while at < periods.len() && periods[at].section == section { x += widths[at] + gap; at += 1; }
            if let Some(s) = section {
                let lines = wrap(&sections[s], x - gap - start);
                let h = lines_size(&lines).1 + 10.0;
                items.push(Item::Path { d: rect(start, y, x - gap - start, h, 6.0), fill: Fill::Soft, stroke: 0.0, dash: false });
                label(&mut items, &lines, start + (x - gap - start) / 2.0, y + h / 2.0, true);
            }
        }
        y += LINE + 10.0 + 12.0;
    }
    let name_h = periods.iter().map(|p| lines_size(&p.name).1).fold(0.0, f32::max) + 14.0;
    let axis = y + name_h + 14.0;
    items.push(Item::Path { d: Pen::new().m(0.0, axis).l(total, axis).done(), fill: Fill::None, stroke: 1.4, dash: false });
    let mut x = 0.0;
    let mut bottom = axis;
    for (period, w) in periods.iter().zip(&widths) {
        let cx = x + w / 2.0;
        let (nw, _) = lines_size(&period.name);
        items.push(Item::Path { d: rect(cx - nw / 2.0 - 12.0, y, nw + 24.0, name_h, 8.0), fill: Fill::Soft, stroke: 1.4, dash: false });
        label(&mut items, &period.name, cx, y + name_h / 2.0, true);
        items.push(crate::draw::dot(cx, axis, 4.0, Fill::Solid));
        let mut ey = axis + 16.0;
        for event in &period.events {
            let (ew, eh) = lines_size(event);
            items.push(Item::Path { d: Pen::new().m(cx, ey - 12.0).l(cx, ey).done(), fill: Fill::None, stroke: 1.0, dash: true });
            items.push(Item::Path { d: rect(cx - ew / 2.0 - 10.0, ey, ew + 20.0, eh + 10.0, 6.0), fill: Fill::None, stroke: 1.2, dash: false });
            label(&mut items, event, cx, ey + (eh + 10.0) / 2.0, false);
            ey += eh + 10.0 + 14.0;
        }
        bottom = f32::max(bottom, ey);
        x += w + gap;
    }
    b.add(0.0, 0.0); b.add(total, bottom);
    Some(finish(items, b))
}
