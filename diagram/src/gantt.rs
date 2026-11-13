//! Gantt charts: tasks as bars along time, in sections, with the time axis below. A task starts at a date or after
//! others, and ends at a date or after a while; one written with only a while follows the task before it.
use crate::draw::{label, r, rect, width, wrap, Diagram, Fill, Item, Pen, LINE, SIZE};
use crate::flow::{finish, Bounds};

/// Minutes since 1970-01-01 00:00, the one unit every date and time here is turned into.
type Minute = i64;

struct Task { name: String, start: Minute, end: Minute, done: bool, active: bool, crit: bool, milestone: bool }
struct Section { name: Option<String>, tasks: Vec<Task> }

/// Days from 1970-01-01 to a date of the proleptic Gregorian calendar.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The date of a day counted from 1970-01-01.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// A moment as written: a date (2026-11-13, 2026/11/13), a time of day (09:50), or both.
fn moment(text: &str) -> Option<Minute> {
    let text = text.trim();
    let (date, time) = match text.split_once([' ', 'T']) { Some((d, t)) => (Some(d), Some(t)), None if text.contains(':') => (None, Some(text)), None => (Some(text), None) };
    let day = match date {
        Some(d) => {
            let parts: Vec<i64> = d.split(['-', '/', '.']).map(|p| p.parse().ok()).collect::<Option<_>>()?;
            let [y, m, dd] = parts[..] else { return None };
            if !(1..=12).contains(&m) || !(1..=31).contains(&dd) { return None; }
            days_from_civil(y, m, dd)
        }
        None => 0,
    };
    let minute = match time {
        Some(t) => {
            let mut parts = t.split(':');
            let h: i64 = parts.next()?.parse().ok()?;
            let m: i64 = parts.next().unwrap_or("0").parse().ok()?;
            if h > 24 || m > 59 { return None; }
            h * 60 + m
        }
        None => 0,
    };
    Some(day * 1440 + minute)
}

/// A while as written: 3d, 2h, 90m, 1w, 1.5h, or parts of it together: 3h25m, 1d12h.
fn duration(text: &str) -> Option<Minute> {
    let text = text.trim();
    let mut total = 0.0;
    let mut number = String::new();
    for c in text.chars() {
        let per = match c { 'm' => 1.0, 'h' => 60.0, 'd' => 1440.0, 'w' => 10080.0, c if c.is_ascii_digit() || c == '.' => { number.push(c); continue } _ => return None };
        let n: f64 = number.parse().ok()?;
        total += n * per;
        number.clear();
    }
    (number.is_empty() && !text.is_empty()).then(|| total.round() as Minute)
}

pub fn gantt(body: &[&str]) -> Option<Diagram> {
    let mut title = None;
    let mut axis = None;
    let mut sections = vec![Section { name: None, tasks: Vec::new() }];
    let mut last_end: Option<Minute> = None;
    let mut ends: Vec<(String, Minute)> = Vec::new();
    for line in body {
        let (word, rest) = line.split_once(char::is_whitespace).map(|(w, r)| (w, r.trim())).unwrap_or((line, ""));
        match word {
            "title" => { title = Some(rest.to_string()); continue; }
            "axisFormat" => { axis = Some(rest.to_string()); continue; }
            "section" => { sections.push(Section { name: Some(rest.to_string()), tasks: Vec::new() }); continue; }
            "dateFormat" | "excludes" | "includes" | "todayMarker" | "tickInterval" | "weekday" | "displayMode" | "accTitle" | "accDescr" => continue,
            _ => {}
        }
        let Some((name, spec)) = line.split_once(':') else { continue };
        let mut parts: Vec<&str> = spec.split(',').map(str::trim).filter(|p| !p.is_empty()).collect();
        let (mut done, mut active, mut crit, mut milestone) = (false, false, false, false);
        while let Some(first) = parts.first() {
            match *first { "done" => done = true, "active" => active = true, "crit" => crit = true, "milestone" => milestone = true, _ => break }
            parts.remove(0);
        }
        // What is left: [id,] [start,] end-or-while. An id is a word that is neither a moment, a while nor `after`.
        let id = if parts.len() >= 2 && moment(parts[0]).is_none() && duration(parts[0]).is_none() && !parts[0].starts_with("after ") {
            Some(parts.remove(0).to_string())
        } else { None };
        let after = |text: &str| -> Option<Minute> {
            let names = text.strip_prefix("after ")?;
            names.split_whitespace().filter_map(|n| ends.iter().rev().find(|(id, _)| id == n).map(|(_, e)| *e)).max()
        };
        // A task whose times cannot be read is left out; the rest of the chart still stands.
        let times = match parts[..] {
            [only] => last_end.and_then(|start| Some((start, duration(only).map(|d| start + d).or_else(|| moment(only))?))),
            [from, to, ..] => moment(from).or_else(|| after(from)).and_then(|start| Some((start, duration(to).map(|d| start + d).or_else(|| moment(to))?))),
            [] if milestone => last_end.map(|start| (start, start)),
            [] => None,
        };
        let Some((start, end)) = times else { continue };
        let end = end.max(start);
        last_end = Some(end);
        if let Some(id) = id { ends.push((id, end)); }
        sections.last_mut()?.tasks.push(Task { name: name.trim().to_string(), start, end, done, active, crit, milestone });
    }
    sections.retain(|s| !s.tasks.is_empty());
    let first = sections.iter().flat_map(|s| &s.tasks).map(|t| t.start).min()?;
    let last = sections.iter().flat_map(|s| &s.tasks).map(|t| t.end).max()?.max(first + 1);

    // The time axis: a step that gives about six to ten ticks, starting on a whole step.
    let span = last - first;
    const STEPS: [Minute; 14] = [5, 10, 15, 30, 60, 120, 180, 360, 720, 1440, 2880, 10080, 20160, 43200];
    let step = *STEPS.iter().find(|&&s| span / s <= 10).unwrap_or(&129600);
    let day_based = step >= 1440;
    let plot = (span as f32 / step as f32 * 72.0).clamp(360.0, 900.0);
    let scale = plot / span as f32;

    let names: Vec<Vec<String>> = sections.iter().flat_map(|s| &s.tasks).map(|t| wrap(&t.name, 200.0)).collect();
    let section_w = sections.iter().filter_map(|s| s.name.as_ref()).map(|n| width(n, SIZE)).fold(0.0, f32::max);
    let left = names.iter().flat_map(|l| l.iter()).map(|l| width(l, SIZE)).fold(section_w, f32::max) + 16.0;
    let x_of = |m: Minute| left + (m - first) as f32 * scale;

    let mut items = Vec::new();
    let mut b = Bounds::default();
    let mut y = 0.0;
    if let Some(title) = &title {
        let lines = wrap(title, (left + plot).max(240.0));
        label(&mut items, &lines, (left + plot) / 2.0, y + LINE * lines.len() as f32 / 2.0, true);
        y += LINE * lines.len() as f32 + 10.0;
    }
    let top = y;
    let mut row = 0;
    for section in &sections {
        if let Some(name) = &section.name {
            items.push(Item::Text { x: r(width(name, SIZE) / 2.0), y: r(y + LINE / 2.0 + 2.0), text: name.clone(), size: SIZE, bold: true });
            y += LINE + 6.0;
        }
        for task in &section.tasks {
            let lines = &names[row];
            let h = (LINE * lines.len() as f32).max(22.0) + 8.0;
            let cy = y + h / 2.0;
            let label_w = lines.iter().map(|l| width(l, SIZE)).fold(0.0, f32::max);
            label(&mut items, lines, left - 12.0 - label_w / 2.0, cy, false);
            let (x0, x1) = (x_of(task.start), x_of(task.end));
            if task.milestone {
                let k = 7.0;
                items.push(Item::Path { d: Pen::new().m(x0, cy - k).l(x0 + k, cy).l(x0, cy + k).l(x0 - k, cy).z(), fill: Fill::Solid, stroke: 0.0, dash: false });
            } else {
                let bar = rect(x0, cy - 10.0, (x1 - x0).max(3.0), 20.0, 4.0);
                let stroke = if task.crit { 2.4 } else if task.active { 2.0 } else { 1.2 };
                items.push(Item::Path { d: bar, fill: if task.done { Fill::None } else { Fill::Soft }, stroke, dash: task.done });
            }
            y += h;
            row += 1;
        }
        y += 4.0;
    }
    // Ticks and their times, under the bars, and a faint line down from each.
    let bottom = y + 4.0;
    let mut tick = first - first.rem_euclid(step) + if first.rem_euclid(step) == 0 { 0 } else { step };
    if day_based { tick = first - first.rem_euclid(1440) + if first.rem_euclid(1440) == 0 { 0 } else { 1440 }; }
    items.push(Item::Path { d: Pen::new().m(left, bottom).l(left + plot, bottom).done(), fill: Fill::None, stroke: 1.0, dash: false });
    while tick <= last {
        let x = x_of(tick);
        items.push(Item::Path { d: Pen::new().m(x, top).l(x, bottom + 4.0).done(), fill: Fill::None, stroke: 0.6, dash: true });
        items.push(Item::Text { x: r(x), y: r(bottom + 4.0 + LINE / 2.0 + 2.0), text: format_tick(tick, axis.as_deref(), day_based), size: 12.0, bold: false });
        tick += step;
    }
    let end_y = bottom + 4.0 + LINE + 4.0;
    b.add(0.0, 0.0); b.add(left + plot + 24.0, end_y);
    Some(finish(items, b))
}

/// A tick's time as the axis format says (%Y %m %d %H %M %b %a are read), else as a day or a time of day.
fn format_tick(at: Minute, format: Option<&str>, day_based: bool) -> String {
    let (y, m, d) = civil_from_days(at.div_euclid(1440));
    let (h, min) = (at.rem_euclid(1440) / 60, at.rem_euclid(60));
    let format = format.unwrap_or(if day_based { "%m-%d" } else { "%H:%M" });
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' { out.push(c); continue; }
        match chars.next() {
            Some('Y') => out.push_str(&y.to_string()), Some('y') => out.push_str(&format!("{:02}", y % 100)),
            Some('m') => out.push_str(&format!("{m:02}")), Some('d') => out.push_str(&format!("{d:02}")), Some('e') => out.push_str(&d.to_string()),
            Some('H') => out.push_str(&format!("{h:02}")), Some('M') => out.push_str(&format!("{min:02}")),
            Some('b') => out.push_str(&format!("{m}月")), Some('a') => out.push_str(["周四", "周五", "周六", "周日", "周一", "周二", "周三"][at.div_euclid(1440).rem_euclid(7) as usize]),
            Some(other) => { out.push('%'); out.push(other); }
            None => out.push('%'),
        }
    }
    out
}
