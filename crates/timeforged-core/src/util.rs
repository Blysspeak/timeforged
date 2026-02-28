use std::path::Path;

pub fn infer_language_from_path(entity: &str) -> Option<String> {
    let path = Path::new(entity);

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

pub fn is_ignored_path(path: &Path) -> bool {
    for component in path.components() {
        let name = component.as_os_str().to_str().unwrap_or("");
        if matches!(
            name,
            ".git"
                | "node_modules"
                | "target"
                | "__pycache__"
                | ".venv"
                | "venv"
                | ".idea"
                | ".vscode"
                | "dist"
                | "build"
                | ".next"
                | ".nuxt"
        ) {
            return true;
        }
    }

    // Ignore binary/lock files by extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if matches!(
            ext,
            "lock" | "exe" | "dll" | "so" | "dylib" | "o" | "a" | "pyc" | "pyo" | "class" | "wasm"
        ) {
            return true;
        }
    }

    false
}
