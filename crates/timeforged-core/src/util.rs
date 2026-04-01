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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── infer_language_from_path ──

    #[test]
    fn infer_rust() {
        assert_eq!(infer_language_from_path("main.rs"), Some("Rust".into()));
        assert_eq!(infer_language_from_path("/home/user/project/src/lib.rs"), Some("Rust".into()));
    }

    #[test]
    fn infer_typescript() {
        assert_eq!(infer_language_from_path("app.ts"), Some("TypeScript".into()));
        assert_eq!(infer_language_from_path("Component.tsx"), Some("TypeScript".into()));
    }

    #[test]
    fn infer_javascript() {
        assert_eq!(infer_language_from_path("index.js"), Some("JavaScript".into()));
        assert_eq!(infer_language_from_path("App.jsx"), Some("JavaScript".into()));
    }

    #[test]
    fn infer_python() {
        assert_eq!(infer_language_from_path("script.py"), Some("Python".into()));
    }

    #[test]
    fn infer_special_files() {
        assert_eq!(infer_language_from_path("Dockerfile"), Some("Docker".into()));
        assert_eq!(infer_language_from_path("Makefile"), Some("Makefile".into()));
        assert_eq!(infer_language_from_path(".gitignore"), Some("Git Config".into()));
        assert_eq!(infer_language_from_path("Cargo.toml"), Some("TOML".into()));
        assert_eq!(infer_language_from_path("package.json"), Some("JSON".into()));
        assert_eq!(infer_language_from_path(".env"), Some("Env".into()));
    }

    #[test]
    fn infer_markup() {
        assert_eq!(infer_language_from_path("index.html"), Some("HTML".into()));
        assert_eq!(infer_language_from_path("style.css"), Some("CSS".into()));
        assert_eq!(infer_language_from_path("style.scss"), Some("CSS".into()));
        assert_eq!(infer_language_from_path("README.md"), Some("Markdown".into()));
    }

    #[test]
    fn infer_unknown_returns_none() {
        assert_eq!(infer_language_from_path("file.xyz"), None);
        assert_eq!(infer_language_from_path("noextension"), None);
    }

    #[test]
    fn infer_windows_paths() {
        assert_eq!(infer_language_from_path("C:\\Users\\dev\\project\\main.rs"), Some("Rust".into()));
        assert_eq!(infer_language_from_path("D:\\work\\app.ts"), Some("TypeScript".into()));
    }

    // ── is_ignored_path ──

    #[test]
    fn ignored_dirs() {
        assert!(is_ignored_path(&PathBuf::from("/project/.git/config")));
        assert!(is_ignored_path(&PathBuf::from("/project/node_modules/foo/bar.js")));
        assert!(is_ignored_path(&PathBuf::from("/project/target/debug/main")));
        assert!(is_ignored_path(&PathBuf::from("/project/__pycache__/mod.pyc")));
        assert!(is_ignored_path(&PathBuf::from("/project/.venv/bin/python")));
        assert!(is_ignored_path(&PathBuf::from("/project/.idea/workspace.xml")));
        assert!(is_ignored_path(&PathBuf::from("/project/dist/index.js")));
    }

    #[test]
    fn ignored_extensions() {
        assert!(is_ignored_path(&PathBuf::from("/project/Cargo.lock")));
        assert!(is_ignored_path(&PathBuf::from("/project/app.exe")));
        assert!(is_ignored_path(&PathBuf::from("/project/lib.so")));
        assert!(is_ignored_path(&PathBuf::from("/project/lib.dll")));
        assert!(is_ignored_path(&PathBuf::from("/project/module.wasm")));
        assert!(is_ignored_path(&PathBuf::from("/project/mod.pyc")));
    }

    #[test]
    fn not_ignored_normal_files() {
        assert!(!is_ignored_path(&PathBuf::from("/project/src/main.rs")));
        assert!(!is_ignored_path(&PathBuf::from("/project/README.md")));
        assert!(!is_ignored_path(&PathBuf::from("/project/app.ts")));
        assert!(!is_ignored_path(&PathBuf::from("/home/user/work/foo/bar.py")));
    }
}
