use tree_sitter::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageId {
    Rust,
    Python,
    Javascript,
    Typescript,
    Tsx,
    Json,
    Bash,
    C,
    Cpp,
    Go,
    Lua,
    Ruby,
    Zig,
    Php,
    Java,
}
impl LanguageId {
    pub const ALL: [Self; 15] = [
        Self::Rust,
        Self::Python,
        Self::Javascript,
        Self::Typescript,
        Self::Tsx,
        Self::Json,
        Self::Bash,
        Self::C,
        Self::Cpp,
        Self::Go,
        Self::Lua,
        Self::Ruby,
        Self::Zig,
        Self::Php,
        Self::Java,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::Javascript => "javascript",
            Self::Typescript => "typescript",
            Self::Tsx => "tsx",
            Self::Json => "json",
            Self::Bash => "bash",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Go => "go",
            Self::Lua => "lua",
            Self::Ruby => "ruby",
            Self::Zig => "zig",
            Self::Php => "php",
            Self::Java => "java",
        }
    }
    pub fn fixture(self) -> &'static str {
        match self {
            Self::Rust => "rust.rs",
            Self::Python => "python.py",
            Self::Javascript => "javascript.js",
            Self::Typescript => "typescript.ts",
            Self::Tsx => "tsx.tsx",
            Self::Json => "json.json",
            Self::Bash => "bash.sh",
            Self::C => "c.c",
            Self::Cpp => "cpp.cpp",
            Self::Go => "go.go",
            Self::Lua => "lua.lua",
            Self::Ruby => "ruby.rb",
            Self::Zig => "zig.zig",
            Self::Php => "php.php",
            Self::Java => "java.java",
        }
    }
    pub fn grammar(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Javascript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Zig => tree_sitter_zig::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
        }
    }
}
