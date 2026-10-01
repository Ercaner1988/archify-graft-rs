//! archify-bridge: Translates Graft's CodeGraph into Archify diagrams.
//!
//! Two levels of architecture: [`GraftToArchifyBridge::compile`] draws one box per crate
//! (real Cargo dependencies, repositories as regions); [`compile_module_level`] opens a
//! crate into its modules. Trees without `Cargo.toml` get the module level directly.
//!
//! [`compile_module_level`]: GraftToArchifyBridge::compile_module_level

use archify_ir::{ArchitectureDiagram, DiagramMeta, VisualPreset};
use graft_model::CodeGraph;

mod crate_level;
mod dataflow;
mod module_level;
mod modules;
mod sequence;
mod tour;

pub(crate) use archify_crates::{crates, labels};
pub(crate) use archify_layout as layered;

pub struct GraftToArchifyBridge;

impl GraftToArchifyBridge {
    /// Level 1. Crates as boxes, Cargo dependencies as arrows (`uses ×N` = number of
    /// `use` paths), repositories / workspaces as regions, sibling repositories as
    /// external boxes, a guided tour in `story_beats`. A tree without any `Cargo.toml`
    /// is drawn at module level instead.
    ///
    /// Component ids of crates of the indexed tree are package names: pass one to
    /// [`Self::compile_module_level`] to open it. External boxes have ids `ext:<repo>/<name>`.
    pub fn compile(graph: &CodeGraph, title: &str, locale: &str) -> ArchitectureDiagram {
        crate_level::diagram(graph, title, locale)
            .unwrap_or_else(|| module_diagram(graph, None, title, locale))
    }

    /// Level 2. The modules of one crate (`crate_id` = package name = level-1 component
    /// id), or of the whole tree when `crate_id` is empty. `None` for an unknown or
    /// external crate, or one without source modules. Cheap enough to call on a click.
    pub fn compile_module_level(
        graph: &CodeGraph,
        crate_id: &str,
        title: &str,
        locale: &str,
    ) -> Option<ArchitectureDiagram> {
        if crate_id.is_empty() {
            return Some(module_diagram(graph, None, title, locale));
        }
        let summary = crates::summarize(graph)?;
        let b = summary
            .boxes
            .iter()
            .find(|b| !b.external && b.id == crate_id)?;
        let diagram = module_diagram(
            graph,
            Some((b.dir.as_str(), b.label.as_str())),
            title,
            locale,
        );
        (!diagram.components.is_empty()).then_some(diagram)
    }

    /// Data files and the functions that write / read them (see `dataflow`).
    pub fn compile_dataflow(
        graph: &CodeGraph,
        title: &str,
        locale: &str,
    ) -> archify_ir::DataflowDiagram {
        dataflow::compile(graph, title, locale)
    }

    pub fn compile_sequence(
        graph: &CodeGraph,
        root_fn: &str,
        title: &str,
        locale: &str,
    ) -> archify_ir::SequenceDiagram {
        sequence::compile(graph, root_fn, title, locale)
    }
}

fn module_diagram(
    graph: &CodeGraph,
    scope: Option<(&str, &str)>,
    title: &str,
    locale: &str,
) -> ArchitectureDiagram {
    let laid = module_level::layout(&modules::summarize_in(graph, scope), locale);
    let story_beats = tour::flow_tour(&laid.components, &laid.connections, locale);
    ArchitectureDiagram {
        meta: DiagramMeta {
            title: title.to_string(),
            subtitle: Some(labels::subtitle(
                laid.shown,
                laid.total,
                laid.connections.len(),
                locale,
            )),
            locale: locale.to_string(),
            visual_preset: VisualPreset::Editorial,
        },
        components: laid.components,
        connections: laid.connections,
        regions: laid.regions,
        story_beats,
    }
}
