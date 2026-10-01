//! The dataflow diagram: real data files (artifacts) and the functions that write or read
//! them, e.g. `uret -> envanter.bin -> oku`. Nothing here is decoration: every node is a
//! function or a file found in the code, every arrow a write, a read or a call.

use crate::crates::{norm, summarize, CrateSummary};
use crate::labels::{crates_subtitle, pick, relation_label, uses_label};
use archify_ir::{
    DataPipeline, DataflowDiagram, DataflowNode, DiagramMeta, SemanticRole, VisualPreset,
};
use graft_model::{CodeGraph, EdgeRelation, NodeKind, NodeV1};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

const MAX_ARTIFACTS: usize = 10;
const MAX_USERS_PER_ARTIFACT: usize = 4;
const MAX_NODES: usize = 40;

fn meta(title: &str, subtitle: String, locale: &str) -> DiagramMeta {
    DiagramMeta {
        title: title.to_string(),
        subtitle: Some(subtitle),
        locale: locale.to_string(),
        visual_preset: VisualPreset::Editorial,
    }
}

/// Role of the crate that owns `path`, `Backend` when unknown.
fn role_at(s: &Option<CrateSummary>, path: &str) -> (SemanticRole, String) {
    let p = norm(path);
    s.as_ref()
        .and_then(|s| {
            s.boxes
                .iter()
                .filter(|b| {
                    !b.external && p.strip_prefix(&b.dir).is_some_and(|r| r.starts_with('/'))
                })
                .max_by_key(|b| b.dir.len())
                .map(|b| (b.role, b.label.clone()))
        })
        .unwrap_or((SemanticRole::Backend, String::new()))
}

fn fn_label(n: &NodeV1) -> String {
    let rest =
        n.id.strip_prefix(n.path.as_str())
            .and_then(|r| r.strip_prefix(':'))
            .unwrap_or(&n.name);
    rest.split('#').next().unwrap_or(rest).to_string()
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

pub fn compile(graph: &CodeGraph, title: &str, locale: &str) -> DataflowDiagram {
    let by_id: HashMap<&str, &NodeV1> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let crates = summarize(graph);

    // artifact -> (writers, readers)
    let mut users: BTreeMap<&str, (BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
    for e in &graph.edges {
        let Some(n) = by_id
            .get(e.target.as_str())
            .filter(|n| n.kind == NodeKind::Artifact)
        else {
            continue;
        };
        let slot = users.entry(n.id.as_str()).or_default();
        match e.relation {
            EdgeRelation::Writes => slot.0.insert(e.source.as_str()),
            EdgeRelation::Reads => slot.1.insert(e.source.as_str()),
            _ => continue,
        };
    }
    if users.is_empty() {
        return fallback(graph, &crates, title, locale);
    }

    let mut ranked: Vec<(&str, usize, bool)> = users
        .iter()
        .map(|(id, (w, r))| (*id, w.len() + r.len(), !w.is_empty() && !r.is_empty()))
        .collect();
    // Files that some code writes and other code reads are the pipeline: first.
    ranked.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)).then(a.0.cmp(b.0)));
    let hidden_artifacts = ranked.len().saturating_sub(MAX_ARTIFACTS);
    ranked.truncate(MAX_ARTIFACTS);

    let mut nodes: Vec<DataflowNode> = Vec::new();
    let mut pipelines: Vec<DataPipeline> = Vec::new();
    let mut have: HashSet<String> = HashSet::new();
    let mut fns: Vec<&str> = Vec::new();
    let (mut hidden_fns, mut edges_seen) = (0usize, HashSet::new());
    for (aid, _, _) in &ranked {
        let a = by_id[aid];
        let (w, r) = &users[aid];
        let detail = match locale {
            "tr" => format!("{} yazar · {} okuyucu", w.len(), r.len()),
            "ar" => format!("{} كاتب · {} قارئ", w.len(), r.len()),
            _ => format!("{} writers · {} readers", w.len(), r.len()),
        };
        nodes.push(DataflowNode {
            id: a.id.clone(),
            label: a.name.clone(),
            role: SemanticRole::Database,
            stream_rate: Some(detail),
        });
        have.insert(a.id.clone());
        for (set, rel) in [(w, EdgeRelation::Writes), (r, EdgeRelation::Reads)] {
            for (k, fid) in set.iter().enumerate() {
                if k >= MAX_USERS_PER_ARTIFACT || nodes.len() >= MAX_NODES {
                    hidden_fns += 1;
                    continue;
                }
                if have.insert((*fid).to_string()) {
                    fns.push(fid);
                }
                let (from, to) = if rel == EdgeRelation::Writes {
                    ((*fid).to_string(), a.id.clone())
                } else {
                    (a.id.clone(), (*fid).to_string())
                };
                if edges_seen.insert((from.clone(), to.clone())) {
                    pipelines.push(DataPipeline {
                        from,
                        to,
                        schema: Some(relation_label(&rel, locale).to_string()),
                        throughput: None,
                    });
                }
            }
        }
    }
    for fid in &fns {
        let n = by_id[fid];
        let (role, krate) = role_at(&crates, &n.path);
        let place = if krate.is_empty() {
            file_name(&n.path).to_string()
        } else {
            format!("{krate} · {}", file_name(&n.path))
        };
        nodes.push(DataflowNode {
            id: n.id.clone(),
            label: fn_label(n),
            role,
            stream_rate: Some(place),
        });
    }
    // One caller from another crate per I/O function: where the data enters the program.
    let mut callers = 0;
    let mut served: HashSet<&str> = HashSet::new();
    for e in graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Calls)
    {
        if callers >= 10 || nodes.len() >= MAX_NODES {
            break;
        }
        let (Some(src), Some(dst)) = (by_id.get(e.source.as_str()), by_id.get(e.target.as_str()))
        else {
            continue;
        };
        if !fns.contains(&e.target.as_str())
            || served.contains(e.target.as_str())
            || have.contains(&e.source)
            || fns.contains(&e.source.as_str())
        {
            continue;
        }
        let (r1, k1) = role_at(&crates, &src.path);
        let (_, k2) = role_at(&crates, &dst.path);
        if k1 == k2 {
            continue;
        }
        have.insert(src.id.clone());
        served.insert(e.target.as_str());
        nodes.push(DataflowNode {
            id: src.id.clone(),
            label: fn_label(src),
            role: r1,
            stream_rate: Some(format!("{k1} · {}", file_name(&src.path))),
        });
        pipelines.push(DataPipeline {
            from: src.id.clone(),
            to: dst.id.clone(),
            schema: Some(relation_label(&EdgeRelation::Calls, locale).to_string()),
            throughput: None,
        });
        callers += 1;
    }
    // Calls between functions that already appear.
    for e in graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Calls)
    {
        if have.contains(&e.source)
            && have.contains(&e.target)
            && edges_seen.insert((e.source.clone(), e.target.clone()))
        {
            pipelines.push(DataPipeline {
                from: e.source.clone(),
                to: e.target.clone(),
                schema: Some(relation_label(&EdgeRelation::Calls, locale).to_string()),
                throughput: None,
            });
        }
    }
    let hidden = hidden_artifacts + hidden_fns;
    if hidden > 0 {
        nodes.push(DataflowNode {
            id: "more".to_string(),
            label: match locale {
                "tr" => format!("+{hidden} daha"),
                "ar" => format!("+{hidden} أخرى"),
                _ => format!("+{hidden} more"),
            },
            role: SemanticRole::External,
            stream_rate: None,
        });
    }
    let files = ranked.len();
    let subtitle = match locale {
        "tr" => format!("Veri akışı · {files} veri dosyası · {} işlev", fns.len()),
        "ar" => format!("تدفق البيانات · {files} ملف · {} دالة", fns.len()),
        _ => format!("Data flow · {files} data files · {} functions", fns.len()),
    };
    DataflowDiagram {
        meta: meta(title, subtitle, locale),
        nodes,
        pipelines,
    }
}

/// No data files found: crate dependencies, or the heaviest file imports.
fn fallback(
    graph: &CodeGraph,
    crates: &Option<CrateSummary>,
    title: &str,
    locale: &str,
) -> DataflowDiagram {
    if let Some(s) = crates {
        let nodes = s
            .boxes
            .iter()
            .take(MAX_NODES)
            .map(|b| DataflowNode {
                id: b.id.clone(),
                label: b.label.clone(),
                role: b.role,
                stream_rate: Some(crate::labels::crate_sublabel(
                    b.lib, b.bin, b.external, b.lines, locale,
                )),
            })
            .collect::<Vec<_>>();
        let shown = nodes.len();
        let pipelines = s
            .edges
            .iter()
            .filter(|e| e.from < shown && e.to < shown)
            .map(|e| DataPipeline {
                from: s.boxes[e.from].id.clone(),
                to: s.boxes[e.to].id.clone(),
                schema: Some(uses_label(e.uses, locale)),
                throughput: None,
            })
            .collect::<Vec<_>>();
        let subtitle = crates_subtitle(shown, s.boxes.len(), pipelines.len(), locale);
        return DataflowDiagram {
            meta: meta(title, subtitle, locale),
            nodes,
            pipelines,
        };
    }
    let mut degree: HashMap<&str, usize> = HashMap::new();
    for e in graph
        .edges
        .iter()
        .filter(|e| e.relation == EdgeRelation::Imports)
    {
        *degree.entry(e.source.as_str()).or_default() += 1;
        *degree.entry(e.target.as_str()).or_default() += 1;
    }
    let mut files: Vec<(&str, usize)> = degree.into_iter().collect();
    files.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    files.truncate(MAX_NODES);
    let keep: HashSet<&str> = files.iter().map(|f| f.0).collect();
    let by_id: HashMap<&str, &NodeV1> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let nodes = files
        .iter()
        .filter_map(|(id, _)| by_id.get(id))
        .map(|n| DataflowNode {
            id: n.id.clone(),
            label: n.name.clone(),
            role: crate::labels::role_for(&n.name, ""),
            stream_rate: None,
        })
        .collect();
    let pipelines = graph
        .edges
        .iter()
        .filter(|e| {
            e.relation == EdgeRelation::Imports
                && keep.contains(e.source.as_str())
                && keep.contains(e.target.as_str())
        })
        .map(|e| DataPipeline {
            from: e.source.clone(),
            to: e.target.clone(),
            schema: Some(relation_label(&EdgeRelation::Imports, locale).to_string()),
            throughput: None,
        })
        .collect();
    let subtitle = pick(
        locale,
        "Dosya bağımlılıkları (veri dosyası bulunamadı)",
        "تبعيات الملفات",
        "File dependencies (no data files found)",
    )
    .to_string();
    DataflowDiagram {
        meta: meta(title, subtitle, locale),
        nodes,
        pipelines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_model::EdgeV1;

    fn node(id: &str, path: &str, name: &str, kind: NodeKind) -> NodeV1 {
        NodeV1 {
            id: id.into(),
            path: path.into(),
            name: name.into(),
            kind,
            span: None,
            search_body: String::new(),
            file_residual: String::new(),
        }
    }

    fn edge(s: &str, t: &str, r: EdgeRelation) -> EdgeV1 {
        EdgeV1 {
            source: s.into(),
            target: t.into(),
            relation: r,
            confidence: 0.9,
        }
    }

    fn pipeline_graph() -> CodeGraph {
        let mut g = CodeGraph::new();
        g.add_node(node(
            "/r/c/uret.rs:uret",
            "/r/c/uret.rs",
            "uret",
            NodeKind::Function,
        ));
        g.add_node(node(
            "/r/c/oku.rs:oku",
            "/r/c/oku.rs",
            "oku",
            NodeKind::Function,
        ));
        g.add_node(node(
            "artifact:envanter.bin",
            "",
            "envanter.bin",
            NodeKind::Artifact,
        ));
        g.add_edge(edge(
            "/r/c/uret.rs:uret",
            "artifact:envanter.bin",
            EdgeRelation::Writes,
        ));
        g.add_edge(edge(
            "/r/c/oku.rs:oku",
            "artifact:envanter.bin",
            EdgeRelation::Reads,
        ));
        g
    }

    #[test]
    fn a_write_and_a_read_of_one_file_form_a_directed_pipeline_without_filler_text() {
        let d = compile(&pipeline_graph(), "t", "tr");
        let labels: Vec<&str> = d.nodes.iter().map(|n| n.label.as_str()).collect();
        assert!(
            labels.contains(&"envanter.bin") && labels.contains(&"uret") && labels.contains(&"oku")
        );
        let p = |a: &str, b: &str| {
            d.pipelines
                .iter()
                .find(|p| p.from.ends_with(a) && p.to.ends_with(b))
        };
        assert_eq!(
            p("uret", "envanter.bin").unwrap().schema.as_deref(),
            Some("yazar")
        );
        assert_eq!(
            p("envanter.bin", "oku").unwrap().schema.as_deref(),
            Some("okur")
        );
        assert!(d
            .nodes
            .iter()
            .all(|n| n.stream_rate.as_deref() != Some("Direct DMA")));
        assert!(d.pipelines.iter().all(|p| p.throughput.is_none()));
    }

    #[test]
    fn containment_is_never_a_flow_and_no_data_files_fall_back_to_imports() {
        let mut g = CodeGraph::new();
        g.add_node(node("file:/r/a.rs", "/r/a.rs", "a.rs", NodeKind::File));
        g.add_node(node("file:/r/b.rs", "/r/b.rs", "b.rs", NodeKind::File));
        g.add_node(node("/r/a.rs:f", "/r/a.rs", "f", NodeKind::Function));
        g.add_edge(edge("file:/r/a.rs", "/r/a.rs:f", EdgeRelation::Contains));
        g.add_edge(edge("file:/r/a.rs", "file:/r/b.rs", EdgeRelation::Imports));
        let d = compile(&g, "t", "en");
        assert_eq!(d.pipelines.len(), 1);
        assert_eq!(d.nodes.len(), 2);
    }

    #[test]
    fn a_long_tail_of_files_becomes_a_counted_more_node() {
        let mut g = CodeGraph::new();
        g.add_node(node("/r/w.rs:w", "/r/w.rs", "w", NodeKind::Function));
        for i in 0..14 {
            let id = format!("artifact:f{i}.bin");
            g.add_node(node(&id, "", &format!("f{i}.bin"), NodeKind::Artifact));
            g.add_edge(edge("/r/w.rs:w", &id, EdgeRelation::Writes));
        }
        let d = compile(&g, "t", "en");
        assert!(
            d.nodes.iter().any(|n| n.label == "+4 more"),
            "{:?}",
            d.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
    }
}
