//! Mermaid diagrams laid out on the device, the same on every client: flowcharts, state diagrams, sequence diagrams,
//! mind maps, Gantt charts, timelines and pie charts, given back as paths and words a client draws in its own font and
//! colours, or as SVG.
mod draw;
mod flow;
mod gantt;
mod mind;
mod pie;
mod sequence;
mod timeline;

pub use draw::{Diagram, Fill, Item};

/// The kinds of Mermaid diagram laid out here, as their first word is written.
pub const KINDS: [&str; 7] = ["flowchart", "sequenceDiagram", "stateDiagram", "mindmap", "gantt", "timeline", "pie"];

/// A diagram from Mermaid source, or nothing when it is not a kind this lays out (the client shows the source).
pub fn diagram(source: &str) -> Option<Diagram> {
    let mut lines: Vec<&str> = Vec::new();
    let mut front = false;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line == "---" && (i == 0 || front) { front = !front; continue; }
        if front { continue; }
        // Indentation is kept: a mind map's outline is written in it.
        let line = match raw.find("%%") { Some(at) => raw[..at].trim_end(), None => raw.trim_end() };
        if !line.trim().is_empty() { lines.push(line); }
    }
    let (header, raw) = lines.split_first()?;
    let header = header.trim();
    let kind = header.split_whitespace().next()?;
    let rest = header[kind.len()..].trim();
    let body: Vec<&str> = raw.iter().map(|l| l.trim()).collect();
    let body = &body[..];
    match kind {
        "graph" | "flowchart" | "flowchart-elk" => flow::flowchart(rest, body),
        "stateDiagram" | "stateDiagram-v2" => flow::states(body),
        "sequenceDiagram" => sequence::sequence(body),
        "mindmap" => mind::mindmap(raw),
        "gantt" => gantt::gantt(body),
        "timeline" => timeline::timeline(body),
        "pie" => pie::pie(rest, body),
        _ => None,
    }
}

/// The diagram as JSON, for clients that draw it themselves.
pub fn json(source: &str) -> Option<String> { diagram(source).map(|d| serde_json::to_string(&d).expect("a diagram serializes")) }

/// The diagram as SVG in pixels of 14px text, in the colour of the text around it.
pub fn svg(source: &str) -> Option<String> {
    let d = diagram(source)?;
    let title = source.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    Some(draw::svg(&d, title))
}
