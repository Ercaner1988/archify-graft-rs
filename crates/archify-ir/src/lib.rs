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
            SemanticRole::Frontend => "#22d3ee",  // Verified Cyan
            SemanticRole::Backend => "#34d399",   // Proof Green
            SemanticRole::Database => "#a78bfa",  // Repository Violet
            SemanticRole::Cloud => "#fbbf24",     // Cloud Amber
            SemanticRole::Security => "#fb7185",  // Boundary Rose
            SemanticRole::Messagebus => "#fb923c",// Transit Orange
            SemanticRole::External => "#94a3b8",  // External Slate
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDiagram {
    pub meta: DiagramMeta,
    pub components: Vec<Component>,
    pub connections: Vec<Connection>,
}
