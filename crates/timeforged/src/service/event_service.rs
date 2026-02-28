use sqlx::SqlitePool;
use uuid::Uuid;

use timeforged_core::api::{BatchEventRequest, BatchEventResponse, CreateEventRequest, EventResponse};
use timeforged_core::error::AppError;
use timeforged_core::models::Event;

use crate::storage::sqlite;

pub async fn create_event(
    pool: &SqlitePool,
    user_id: Uuid,
    req: CreateEventRequest,
) -> Result<EventResponse, AppError> {
    validate_event(&req)?;

    let event = Event {
        id: None,
        user_id,
        timestamp: req.timestamp,
        event_type: req.event_type.clone(),
        entity: req.entity.clone(),
        project: normalize_project(&req),
        language: normalize_language(&req),
        branch: req.branch.clone(),
        activity: req.activity.clone(),
        machine: req.machine.clone(),
        metadata: req.metadata.clone(),
        created_at: None,
    };

    let id = sqlite::insert_event(pool, &event).await?;

    Ok(EventResponse {
        id,
        timestamp: event.timestamp,
        event_type: event.event_type,
        entity: event.entity,
    })
}

pub async fn create_batch(
    pool: &SqlitePool,
    user_id: Uuid,
    req: BatchEventRequest,
) -> Result<BatchEventResponse, AppError> {
    if req.events.len() > 100 {
        return Err(AppError::BadRequest("batch size exceeds 100".into()));
    }

    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for event_req in req.events {
        match create_event(pool, user_id, event_req).await {
            Ok(_) => accepted += 1,
            Err(_) => rejected += 1,
        }
    }

    Ok(BatchEventResponse { accepted, rejected })
}

fn validate_event(req: &CreateEventRequest) -> Result<(), AppError> {
    if req.entity.is_empty() {
        return Err(AppError::Validation("entity cannot be empty".into()));
    }
    if req.entity.len() > 1024 {
        return Err(AppError::Validation("entity too long".into()));
    }
    Ok(())
}

fn normalize_project(req: &CreateEventRequest) -> Option<String> {
    if let Some(ref p) = req.project {
        if !p.is_empty() {
            return Some(p.clone());
        }
    }
    // Try to infer project from entity path
    infer_project_from_path(&req.entity)
}

fn normalize_language(req: &CreateEventRequest) -> Option<String> {
    if let Some(ref l) = req.language {
        if !l.is_empty() {
            return Some(l.clone());
        }
    }
    infer_language_from_path(&req.entity)
}

fn infer_project_from_path(entity: &str) -> Option<String> {
    // Try to find a project directory from common patterns
    let path = std::path::Path::new(entity);
    // Walk up looking for common project markers
    for ancestor in path.ancestors().skip(1) {
        let name = ancestor.file_name()?.to_str()?;
        // Skip common non-project directories
        if matches!(name, "src" | "lib" | "bin" | "test" | "tests" | "spec" | "node_modules" | ".git") {
            continue;
        }
        // If parent is home or root-like, this is likely the project
        if let Some(parent) = ancestor.parent() {
            let parent_name = parent.file_name().map(|n| n.to_str().unwrap_or(""));
            if matches!(parent_name, Some("home" | "Users" | "projects" | "repos" | "workspace" | "workSpace" | "code" | "dev" | "src"))
                || parent.parent().is_none()
            {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn infer_language_from_path(entity: &str) -> Option<String> {
    let path = std::path::Path::new(entity);

    // Check filename first (for dotfiles and extensionless files)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lang = match name {
            ".gitignore" | ".gitattributes" | ".gitmodules" => "Git Config",
            "Dockerfile" | "Containerfile" => "Docker",
            "Makefile" | "GNUmakefile" => "Makefile",
            "Justfile" | "justfile" => "Just",
            ".env" | ".env.local" | ".env.production" => "Env",
            "Cargo.toml" | "Cargo.lock" => "TOML",
            "package.json" | "tsconfig.json" => "JSON",
            _ => "",
        };
        if !lang.is_empty() {
            return Some(lang.to_string());
        }
    }

    let ext = path.extension()?.to_str()?;
    let lang = match ext {
        "rs" => "Rust",
        "py" => "Python",
        "js" => "JavaScript",
        "ts" => "TypeScript",
        "tsx" => "TypeScript",
        "jsx" => "JavaScript",
        "go" => "Go",
        "java" => "Java",
        "kt" => "Kotlin",
        "rb" => "Ruby",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "cs" => "C#",
        "swift" => "Swift",
        "php" => "PHP",
        "lua" => "Lua",
        "zig" => "Zig",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "html" | "htm" => "HTML",
        "css" | "scss" | "sass" | "less" => "CSS",
        "sql" => "SQL",
        "sh" | "bash" | "zsh" => "Shell",
        "toml" => "TOML",
        "yaml" | "yml" => "YAML",
        "json" | "jsonc" => "JSON",
        "md" | "markdown" => "Markdown",
        "xml" | "svg" => "XML",
        "graphql" | "gql" => "GraphQL",
        "proto" => "Protobuf",
        "dockerfile" => "Docker",
        "tf" | "hcl" => "Terraform",
        "r" => "R",
        "dart" => "Dart",
        "scala" => "Scala",
        "ex" | "exs" => "Elixir",
        "hs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "nix" => "Nix",
        "vim" => "Vim Script",
        "el" => "Emacs Lisp",
        _ => return None,
    };
    Some(lang.to_string())
}
