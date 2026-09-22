//! archify-ir: Strongly-typed JSON-IR schemas and semantic roles.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticRole {
    Frontend,
    Backend,
    Database,
    Cloud,
    Security,
    Messagebus,
    External,
}

impl SemanticRole {
    pub fn stroke_hex(&self) -> &'static str {
        match self {
            SemanticRole::Frontend => "#22d3ee",   // Verified Cyan
            SemanticRole::Backend => "#34d399",    // Proof Green
            SemanticRole::Database => "#a78bfa",   // Repository Violet
            SemanticRole::Cloud => "#fbbf24",      // Cloud Amber
            SemanticRole::Security => "#fb7185",   // Boundary Rose
            SemanticRole::Messagebus => "#fb923c", // Transit Orange
            SemanticRole::External => "#94a3b8",   // External Slate
        }
    }

    pub fn fill_rgba(&self) -> &'static str {
        match self {
            SemanticRole::Frontend => "rgba(8, 51, 68, 0.4)",
            SemanticRole::Backend => "rgba(6, 78, 59, 0.4)",
            SemanticRole::Database => "rgba(76, 29, 149, 0.4)",
            SemanticRole::Cloud => "rgba(120, 53, 15, 0.3)",
            SemanticRole::Security => "rgba(136, 19, 55, 0.4)",
            SemanticRole::Messagebus => "rgba(251, 146, 60, 0.3)",
            SemanticRole::External => "rgba(30, 41, 59, 0.5)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisualPreset {
    Classic,
    SignalFlow,
    Blueprint,
    Editorial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramMeta {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub locale: String, // "tr", "ar", "en"
    pub visual_preset: VisualPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sublabel: Option<String>,
    pub role: SemanticRole,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub line_style: String,
}

/// Bileşenleri çevreleyen çerçeve (ör. bir crate ya da üst klasör). Yalnız çizim
/// içindir: konumu köprü hesaplar, bileşenler kendi koordinatlarını taşır.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sublabel: Option<String>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryBeat {
    pub step: usize,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub highlighted_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDiagram {
    pub meta: DiagramMeta,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
    /// Boşsa çizim eskisi gibi düz kalır (geri uyum).
    #[serde(default)]
    pub regions: Vec<Region>,
    #[serde(default)]
    pub story_beats: Vec<StoryBeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub label: String,
    pub lane: String,
    pub phase: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDiagram {
    pub meta: DiagramMeta,
    pub lanes: Vec<String>,
    pub steps: Vec<WorkflowStep>,
    pub connections: Vec<Connection>,
}

// Dataflow Diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowNode {
    pub id: String,
    pub label: String,
    pub role: SemanticRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_rate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPipeline {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataflowDiagram {
    pub meta: DiagramMeta,
    pub nodes: Vec<DataflowNode>,
    pub pipelines: Vec<DataPipeline>,
}

// Lifecycle Diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleState {
    pub id: String,
    pub label: String,
    pub is_terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleDiagram {
    pub meta: DiagramMeta,
    pub states: Vec<LifecycleState>,
    pub transitions: Vec<StateTransition>,
}

// Sequence Diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceParticipant {
    pub id: String,
    pub label: String,
    pub role: SemanticRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceMessage {
    pub order: usize,
    pub from: String,
    pub to: String,
    pub action: String,
    pub is_async: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceDiagram {
    pub meta: DiagramMeta,
    pub participants: Vec<SequenceParticipant>,
    pub messages: Vec<SequenceMessage>,
}

// Delta Analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffStatus {
    Added,
    Removed,
    Modified,
    Unchanged,
}

impl DiffStatus {
    pub fn neon_hex(&self) -> &'static str {
        match self {
            DiffStatus::Added => "#22c55e",
            DiffStatus::Removed => "#f43f5e",
            DiffStatus::Modified => "#f59e0b",
            DiffStatus::Unchanged => "#475569",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaComponent {
    pub component: Component,
    pub status: DiffStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaConnection {
    pub connection: Connection,
    pub status: DiffStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaDiagram {
    pub meta: DiagramMeta,
    pub components: Vec<DeltaComponent>,
    pub connections: Vec<DeltaConnection>,
}
