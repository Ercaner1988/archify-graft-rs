//! Tour beats: the ordered node lists of the "Explore this system" mode, with the relation
//! between consecutive nodes worked out from the real edges (nothing is invented).

use crate::{Scene, SceneEdge, SceneNode};
use archify_ir::StoryBeat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Start,
    /// previous -> this
    Forward,
    /// this -> previous
    Reverse,
    Both,
    /// no direct edge between the two
    Group,
}

#[derive(Debug, Clone)]
pub struct BeatStep {
    pub node: usize,
    pub relation: Relation,
    /// Labels of the edges between the previous node and this one.
    pub edge_labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SceneBeat {
    pub title: String,
    pub description: String,
    pub nodes: Vec<usize>,
    pub steps: Vec<BeatStep>,
}

impl BeatStep {
    /// `A → B`, `A ← B`, `A ⇄ B`, `A · B` or `Start: A`.
    pub fn headline(&self, scene: &Scene, previous: Option<usize>, start_word: &str) -> String {
        let this = &scene.nodes[self.node].label.text;
        let Some(prev) = previous.map(|p| &scene.nodes[p].label.text) else {
            return format!("{start_word}: {this}");
        };
        match self.relation {
            Relation::Start => format!("{start_word}: {this}"),
            Relation::Forward => format!("{prev} → {this}"),
            Relation::Reverse => format!("{prev} ← {this}"),
            Relation::Both => format!("{prev} ⇄ {this}"),
            Relation::Group => format!("{prev} · {this}"),
        }
    }
}

fn relation(edges: &[SceneEdge], a: usize, b: usize) -> (Relation, Vec<String>) {
    let mut fwd = false;
    let mut rev = false;
    let mut labels = Vec::new();
    for e in edges {
        let hit = if e.from == a && e.to == b {
            fwd = true;
            true
        } else if e.from == b && e.to == a {
            rev = true;
            true
        } else {
            false
        };
        if hit {
            if let Some(l) = e.full_label.as_ref().or(e.label.as_ref()) {
                if !labels.contains(l) {
                    labels.push(l.clone());
                }
            }
        }
    }
    let rel = match (fwd, rev) {
        (true, true) => Relation::Both,
        (true, false) => Relation::Forward,
        (false, true) => Relation::Reverse,
        _ => Relation::Group,
    };
    (rel, labels)
}

fn steps_for(nodes: &[usize], edges: &[SceneEdge]) -> Vec<BeatStep> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            if i == 0 {
                BeatStep {
                    node: n,
                    relation: Relation::Start,
                    edge_labels: Vec::new(),
                }
            } else {
                let (relation, edge_labels) = relation(edges, nodes[i - 1], n);
                BeatStep {
                    node: n,
                    relation,
                    edge_labels,
                }
            }
        })
        .collect()
}

/// Turns the diagram's story beats into scene beats. Unknown ids are skipped; beats left
/// without nodes disappear. When the diagram has none, a tour is derived from the graph.
pub fn build(
    beats: &[StoryBeat],
    nodes: &[SceneNode],
    edges: &[SceneEdge],
    locale: &str,
) -> Vec<SceneBeat> {
    let mut out: Vec<SceneBeat> = beats
        .iter()
        .filter_map(|b| {
            let idx: Vec<usize> = b
                .highlighted_nodes
                .iter()
                .filter_map(|id| nodes.iter().position(|n| &n.id == id))
                .collect();
            (!idx.is_empty()).then(|| SceneBeat {
                title: b.title.clone(),
                description: b.description.clone().unwrap_or_default(),
                steps: steps_for(&idx, edges),
                nodes: idx,
            })
        })
        .collect();
    if out.is_empty() {
        out = auto_beats(nodes, edges, locale);
    }
    out
}

/// Main flow (longest path) plus up to three hub neighbourhoods.
fn auto_beats(nodes: &[SceneNode], edges: &[SceneEdge], locale: &str) -> Vec<SceneBeat> {
    let n = nodes.len();
    if n < 3 || edges.is_empty() {
        return Vec::new();
    }
    let tr = locale == "tr";
    let mut beats = Vec::new();
    let path = longest_path(n, edges);
    if path.len() >= 3 {
        let title = if tr { "Ana akış" } else { "Main flow" };
        let desc = if tr {
            "Bağımlılık yönünde en uzun zincir."
        } else {
            "The longest chain along the dependency direction."
        };
        beats.push(SceneBeat {
            title: title.to_string(),
            description: desc.to_string(),
            steps: steps_for(&path, edges),
            nodes: path,
        });
    }
    let mut degree: Vec<(usize, usize)> = (0..n)
        .map(|i| (i, edges.iter().filter(|e| e.from == i || e.to == i).count()))
        .collect();
    degree.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for &(hub, d) in degree.iter().take(3) {
        if d < 2 {
            break;
        }
        let mut list = vec![hub];
        for e in edges {
            for other in [e.from, e.to] {
                if other != hub
                    && (e.from == hub || e.to == hub)
                    && !list.contains(&other)
                    && list.len() < 7
                {
                    list.push(other);
                }
            }
        }
        let name = &nodes[hub].label.text;
        let title = if tr {
            format!("{name} çevresi")
        } else {
            format!("Around {name}")
        };
        beats.push(SceneBeat {
            title,
            description: String::new(),
            steps: steps_for(&list, edges),
            nodes: list,
        });
    }
    beats
}

fn longest_path(n: usize, edges: &[SceneEdge]) -> Vec<usize> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in edges {
        if e.from != e.to {
            adj[e.from].push(e.to);
        }
    }
    // DFS with memo; cycles broken by the on-stack marker
    fn dfs(
        u: usize,
        adj: &[Vec<usize>],
        memo: &mut [Option<Vec<usize>>],
        on: &mut [bool],
    ) -> Vec<usize> {
        if let Some(m) = &memo[u] {
            return m.clone();
        }
        on[u] = true;
        let mut best: Vec<usize> = Vec::new();
        for &v in &adj[u] {
            if on[v] {
                continue;
            }
            let p = dfs(v, adj, memo, on);
            if p.len() > best.len() {
                best = p;
            }
        }
        on[u] = false;
        let mut path = vec![u];
        path.extend(best);
        memo[u] = Some(path.clone());
        path
    }
    let mut memo: Vec<Option<Vec<usize>>> = vec![None; n];
    let mut on = vec![false; n];
    let mut best: Vec<usize> = Vec::new();
    for u in 0..n {
        let p = dfs(u, &adj, &mut memo, &mut on);
        if p.len() > best.len() {
            best = p;
        }
    }
    best.truncate(8);
    best
}
