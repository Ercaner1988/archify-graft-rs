//! The guided tour (`story_beats`): an ordered walk through the diagram with real crate
//! and module names, from where the program starts to what it stands on.

use crate::crates::CrateSummary;
use crate::labels::{pick, uses_label};
use archify_ir::{Component, Connection, SemanticRole, StoryBeat};
use std::collections::HashMap;

fn join(names: &[&str]) -> String {
    names.join(", ")
}

/// `a → b (kullanır ×N)` for the heaviest outgoing arrow of each of the first `max` boxes.
fn heaviest_out(s: &CrateSummary, ids: &[usize], max: usize, locale: &str) -> String {
    let mut parts = Vec::new();
    for &i in ids.iter().take(max) {
        let best = s
            .edges
            .iter()
            .filter(|e| e.from == i && !s.boxes[e.to].external)
            .max_by_key(|e| (e.uses, std::cmp::Reverse(e.to)));
        if let Some(e) = best {
            parts.push(format!(
                "{} → {} ({})",
                s.boxes[i].label,
                s.boxes[e.to].label,
                uses_label(e.uses, locale)
            ));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" {}.", parts.join("; "))
    }
}

fn beat(
    step: usize,
    title: &str,
    description: String,
    ids: &[usize],
    s: &CrateSummary,
) -> StoryBeat {
    StoryBeat {
        step,
        title: format!("{step}. {title}"),
        description: Some(description),
        highlighted_nodes: ids.iter().map(|&i| s.boxes[i].id.clone()).collect(),
    }
}

pub fn crate_tour(s: &CrateSummary, locale: &str) -> Vec<StoryBeat> {
    let local = |i: &usize| !s.boxes[*i].external;
    let dependents = |i: usize| s.edges.iter().filter(|e| e.to == i).count();
    let by_size = |v: &mut Vec<usize>| {
        v.sort_by(|a, b| s.boxes[*b].lines.cmp(&s.boxes[*a].lines).then(a.cmp(b)));
    };
    let all: Vec<usize> = (0..s.boxes.len()).filter(local).collect();

    let mut entries: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&i| {
            let b = &s.boxes[i];
            dependents(i) == 0 && (b.bin || b.role == SemanticRole::Frontend)
        })
        .collect();
    by_size(&mut entries);
    let mut storage: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&i| s.boxes[i].role == SemanticRole::Database)
        .collect();
    by_size(&mut storage);
    let mut ui: Vec<usize> = all
        .iter()
        .copied()
        .filter(|i| s.boxes[*i].role == SemanticRole::Frontend && !entries.contains(i))
        .collect();
    by_size(&mut ui);
    let mut core: Vec<usize> = all
        .iter()
        .copied()
        .filter(|i| {
            dependents(*i) > 0 && !entries.contains(i) && !storage.contains(i) && !ui.contains(i)
        })
        .collect();
    core.sort_by(|a, b| dependents(*b).cmp(&dependents(*a)).then(a.cmp(b)));
    core.truncate(4);
    let mut outside: Vec<usize> = (0..s.boxes.len()).filter(|i| !local(i)).collect();
    outside.sort_by(|a, b| dependents(*b).cmp(&dependents(*a)).then(a.cmp(b)));
    outside.truncate(6);

    let names = |ids: &[usize]| -> String {
        join(
            &ids.iter()
                .map(|&i| s.boxes[i].label.as_str())
                .collect::<Vec<_>>(),
        )
    };
    let mut beats = Vec::new();
    let mut add = |title: &str, text: String, ids: &[usize]| {
        if !ids.is_empty() {
            beats.push(beat(beats.len() + 1, title, text, ids, s));
        }
    };
    add(
        pick(locale, "Giriş noktaları", "نقاط الدخول", "Entry points"),
        format!(
            "{}: {}.{}",
            pick(
                locale,
                "Kullanıcının başlattığı sandıklar",
                "الصناديق التي يبدأها المستخدم",
                "Crates the user starts"
            ),
            names(&entries),
            heaviest_out(s, &entries, 3, locale)
        ),
        &entries,
    );
    let api: Vec<&str> = core
        .first()
        .map(|&i| s.boxes[i].api.iter().map(String::as_str).collect())
        .unwrap_or_default();
    add(
        pick(locale, "Çekirdek", "النواة", "Core"),
        format!(
            "{}: {}.{}{}",
            pick(
                locale,
                "Diğer sandıkların dayandığı merkez",
                "المركز الذي تعتمد عليه الصناديق الأخرى",
                "What the other crates stand on"
            ),
            names(&core),
            heaviest_out(s, &core, 2, locale),
            if api.is_empty() {
                String::new()
            } else {
                format!(
                    " {}: {}.",
                    pick(locale, "Genel API", "الواجهة العامة", "Public API"),
                    join(&api)
                )
            }
        ),
        &core,
    );
    add(
        pick(
            locale,
            "Depolama ve veri",
            "التخزين والبيانات",
            "Storage and data",
        ),
        format!(
            "{}: {}.{}",
            pick(
                locale,
                "Kalıcı veriyi tutan sandıklar",
                "الصناديق التي تحفظ البيانات",
                "Crates that keep persistent data"
            ),
            names(&storage),
            heaviest_out(s, &storage, 2, locale)
        ),
        &storage,
    );
    add(
        pick(locale, "Arayüz", "الواجهة", "Interface"),
        format!(
            "{}: {}.{}",
            pick(
                locale,
                "Kullanıcıya görünen katman",
                "الطبقة الظاهرة للمستخدم",
                "The layer the user sees"
            ),
            names(&ui),
            heaviest_out(s, &ui, 3, locale)
        ),
        &ui,
    );
    add(
        pick(
            locale,
            "Dış depolar",
            "المستودعات الخارجية",
            "Outside repositories",
        ),
        format!(
            "{}: {}.",
            pick(
                locale,
                "Bu çalışma alanının dayandığı kardeş depolar",
                "المستودعات الشقيقة التي يعتمد عليها",
                "Sibling repositories this workspace depends on"
            ),
            names(&outside)
        ),
        &outside,
    );
    if beats.is_empty() && !all.is_empty() {
        let mut every = all.clone();
        by_size(&mut every);
        beats.push(beat(
            1,
            pick(locale, "Genel bakış", "نظرة عامة", "Overview"),
            names(&every),
            &every,
            s,
        ));
    }
    beats
}

/// Module-level tour: sources (entry), the busiest hubs, then the leaves they end in.
pub fn flow_tour(
    components: &[Component],
    connections: &[Connection],
    locale: &str,
) -> Vec<StoryBeat> {
    let label: HashMap<&str, &str> = components
        .iter()
        .map(|c| (c.id.as_str(), c.label.as_str()))
        .collect();
    let (mut incoming, mut outgoing) = (HashMap::<&str, u32>::new(), HashMap::<&str, u32>::new());
    for c in connections {
        *outgoing.entry(c.from.as_str()).or_default() += 1;
        *incoming.entry(c.to.as_str()).or_default() += 1;
    }
    let deg =
        |id: &str| incoming.get(id).copied().unwrap_or(0) + outgoing.get(id).copied().unwrap_or(0);
    let ids: Vec<&str> = components
        .iter()
        .map(|c| c.id.as_str())
        .filter(|id| deg(id) > 0)
        .collect();
    let pick_ids = |keep: &dyn Fn(&str) -> bool, max: usize| -> Vec<String> {
        let mut v: Vec<&str> = ids.iter().copied().filter(|id| keep(id)).collect();
        v.sort_by(|a, b| deg(b).cmp(&deg(a)).then(a.cmp(b)));
        v.into_iter().take(max).map(str::to_string).collect()
    };
    let sources = pick_ids(&|id| !incoming.contains_key(id), 4);
    let leaves = pick_ids(&|id| !outgoing.contains_key(id), 4);
    let hubs = pick_ids(
        &|id| incoming.contains_key(id) && outgoing.contains_key(id),
        4,
    );
    let text = |ids: &[String]| join(&ids.iter().map(|i| label[i.as_str()]).collect::<Vec<_>>());
    let mut beats = Vec::new();
    for (title, lead, group) in [
        (
            pick(locale, "Giriş", "المدخل", "Entry"),
            pick(
                locale,
                "Başkasının kullanmadığı, işin başladığı modüller",
                "وحدات لا يستخدمها أحد وتبدأ منها العملية",
                "Modules nobody else uses, where work starts",
            ),
            sources,
        ),
        (
            pick(locale, "Orta katman", "الطبقة الوسطى", "Middle layer"),
            pick(
                locale,
                "Hem kullanılan hem kullanan merkez modüller",
                "وحدات مركزية تستخدم وتُستخدم",
                "Hub modules that both use and are used",
            ),
            hubs,
        ),
        (
            pick(locale, "Temel", "الأساس", "Foundation"),
            pick(
                locale,
                "Başka modül kullanmayan, yaprak modüller",
                "وحدات ورقية لا تستخدم غيرها",
                "Leaf modules that use nothing else",
            ),
            leaves,
        ),
    ] {
        if !group.is_empty() {
            beats.push(StoryBeat {
                step: beats.len() + 1,
                title: format!("{}. {title}", beats.len() + 1),
                description: Some(format!("{lead}: {}.", text(&group))),
                highlighted_nodes: group,
            });
        }
    }
    beats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crates::{CrateBox, CrateEdge};

    fn b(id: &str, role: SemanticRole, bin: bool, lines: usize, external: bool) -> CrateBox {
        CrateBox {
            id: id.into(),
            label: id.into(),
            group: "g".into(),
            external,
            lines,

            bin,
            lib: !bin,
            role,
            api: vec!["graf_kur".into()],
            dir: String::new(),
        }
    }

    fn edge(from: usize, to: usize, uses: u32) -> CrateEdge {
        CrateEdge {
            from,
            to,
            uses,
            optional: false,
        }
    }

    #[test]
    fn the_tour_goes_entry_core_storage_interface_outside_with_real_names() {
        let s = CrateSummary {
            boxes: vec![
                b("pasli", SemanticRole::Frontend, true, 400, false),
                b("cekirdek", SemanticRole::Backend, false, 9000, false),
                b("depo", SemanticRole::Database, false, 800, false),
                b("pencere", SemanticRole::Frontend, false, 3000, false),
                b("katla", SemanticRole::External, false, 0, true),
            ],
            edges: vec![
                edge(0, 1, 39),
                edge(1, 2, 12),
                edge(3, 1, 24),
                edge(1, 4, 5),
            ],
        };
        let t = crate_tour(&s, "tr");
        let titles: Vec<&str> = t.iter().map(|x| x.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "1. Giriş noktaları",
                "2. Çekirdek",
                "3. Depolama ve veri",
                "4. Dış depolar"
            ]
        );
        assert_eq!(t[0].highlighted_nodes, ["pencere", "pasli"]);
        let d1 = t[0].description.as_deref().unwrap();
        assert!(d1.contains("pasli → cekirdek (kullanır ×39)"), "{d1}");
        assert!(t[1]
            .description
            .as_deref()
            .unwrap()
            .contains("Genel API: graf_kur"));
        assert!(crate_tour(&s, "en")[0].title.starts_with("1. Entry points"));
    }

    #[test]
    fn module_tour_walks_sources_hubs_then_leaves() {
        let comp = |id: &str| Component {
            id: id.into(),
            label: id.into(),
            sublabel: None,
            role: SemanticRole::Backend,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        };
        let conn = |a: &str, b: &str| Connection {
            from: a.into(),
            to: b.into(),
            label: None,
            line_style: String::new(),
        };
        let t = flow_tour(
            &[comp("a"), comp("b"), comp("c")],
            &[conn("a", "b"), conn("b", "c")],
            "en",
        );
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].highlighted_nodes, ["a"]);
        assert_eq!(t[1].highlighted_nodes, ["b"]);
        assert_eq!(t[2].highlighted_nodes, ["c"]);
    }
}
