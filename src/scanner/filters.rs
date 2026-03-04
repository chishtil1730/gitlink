/// Directories that are build/tool artifacts and should never be scanned.
pub const IGNORED_DIRS: &[&str] = &[
    // Version control
    ".git", ".svn", ".hg",
    // Rust
    "target", ".cargo",
    // JavaScript / Node
    "node_modules", ".next", ".nuxt", ".turbo", ".parcel-cache",
    // Python
    "__pycache__", ".venv", "venv", "env", ".env",
    ".mypy_cache", ".pytest_cache", ".ruff_cache", ".tox",
    // Java / Kotlin / Android
    ".gradle", ".idea", "build", "out", ".android",
    // Go
    "vendor",
    // Ruby
    ".bundle",
    // Editors & IDEs
    ".vscode", ".vs",
    // Frontend build outputs
    "dist", ".output", ".svelte-kit",
    // Infrastructure / Cloud
    ".terraform", ".serverless", ".pulumi",
    // CI / misc caches
    ".cache", "coverage", ".nyc_output",
];

/// File extensions that are binary, media, archive, or compiler-generated.
pub const IGNORED_EXTENSIONS: &[&str] = &[
    // Compiled / binary
    "exe", "dll", "so", "dylib", "lib", "bin", "o", "a", "obj",
    "class", "jar", "war", "ear", "wasm", "pdb", "ilk", "exp",
    // Rust / LLVM compiler artifacts
    "rlib", "rmeta",
    // Make dependency files (.d) — list of file paths, triggers entropy false positives
    "d",
    // Python bytecode
    "pyc", "pyo", "pyd",
    // Images
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "tiff", "avif",
    // Audio / video
    "mp3", "mp4", "wav", "ogg", "flac", "avi", "mov", "mkv", "webm",
    // Archives
    "zip", "tar", "gz", "bz2", "xz", "zst", "7z", "rar",
    // Documents
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    // Fonts
    "ttf", "otf", "woff", "woff2", "eot",
    // Database / misc binary
    "db", "sqlite", "sqlite3", "dat", "pak",
    // Source maps and generated JS bundles
    "map",
];

/// Specific filenames (exact match, any directory) that produce false positives.
pub const IGNORED_FILES: &[&str] = &[
    // Dependency lock files — full of hashes, not secrets
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "go.sum",
    "mix.lock",
    "pubspec.lock",
    // Cargo internals
    "Cargo.toml.orig",
    ".cargo-lock",
];

/// Path segments (any component in the full path) that indicate generated artifacts.
/// These catch subdirectories inside `target/` that slip through if `target` itself
/// is somehow not matched (e.g. symlinks, unusual roots).
pub const IGNORED_PATH_SEGMENTS: &[&str] = &[
    // Cargo build internals
    ".fingerprint",
    "incremental",
    "deps",     // target/release/deps — compiled crate artifacts (.d, .rlib, etc.)
    // Generic caches
    ".cache",
];