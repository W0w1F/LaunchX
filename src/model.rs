use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Url,
    File,
    Folder,
}

impl ResourceType {
    pub const ALL: [ResourceType; 3] =
        [ResourceType::Url, ResourceType::File, ResourceType::Folder];

    pub fn label(&self) -> &'static str {
        match self {
            ResourceType::Url => "URL",
            ResourceType::File => "File",
            ResourceType::Folder => "Folder",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            ResourceType::Url => "🌐",
            ResourceType::File => "📄",
            ResourceType::Folder => "📁",
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            ResourceType::Url => "https://github.com/me/repo",
            ResourceType::File => "/home/me/docs/spec.pdf",
            ResourceType::Folder => "/home/me/dev/project",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ResourceType,
    pub target: String,
    /// Optional app to open with, e.g. "code" for VS Code (folders).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_with: Option<String>,
}

impl Resource {
    pub fn new(kind: ResourceType) -> Self {
        Resource {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            kind,
            target: String::new(),
            open_with: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub color: String, // hex, e.g. "#6366f1"
    pub created_at: String,
    pub resources: Vec<Resource>,
}

impl Project {
    pub fn new() -> Self {
        Project {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            color: crate::ui::PROJECT_COLORS[0].to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            resources: Vec::new(),
        }
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for r in &self.resources {
            match r.kind {
                ResourceType::Url => c.0 += 1,
                ResourceType::File => c.1 += 1,
                ResourceType::Folder => c.2 += 1,
            }
        }
        c
    }
}

/// A reusable set of resources that can be merged into any project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub projects: Vec<Project>,
    #[serde(default)]
    pub templates: Vec<Template>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            version: 1,
            projects: Vec::new(),
            templates: Vec::new(),
        }
    }
}
