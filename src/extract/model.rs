//! Data models for extracted tokens and entity types.

/// The semantic classification of an extracted entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    /// Filesystem path, potentially with line and column suffixes (e.g. `src/main.rs:42:10`).
    Path,
    /// Web URL or URI schema (e.g. `https://example.com`).
    Url,
    /// Git commit SHA or hex hash (e.g. `a1b2c3d`).
    Hash,
    /// IPv4 or IPv6 network address.
    Ip,
    /// Universally unique identifier (UUID).
    Uuid,
    /// Shell command or executable invocation.
    Command,
    /// Single or double-quoted string literal.
    Quoted,
    /// Standalone numeric value or identifier.
    Number,
    /// A general token selected from terminal text.
    Word,
    /// A Docker image reference with an explicit registry and tag or digest.
    DockerImage,
    /// A Kubernetes resource in `kind/name` form.
    KubernetesResource,
    /// A three- or six-digit CSS hexadecimal color.
    HexColor,
    /// An IPFS content identifier (CIDv0 or CIDv1).
    IpfsCid,
}

impl EntityKind {
    pub const ALL: [Self; 13] = [
        Self::Path,
        Self::Url,
        Self::Hash,
        Self::Ip,
        Self::Uuid,
        Self::Command,
        Self::Quoted,
        Self::Number,
        Self::Word,
        Self::DockerImage,
        Self::KubernetesResource,
        Self::HexColor,
        Self::IpfsCid,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Url => "url",
            Self::Hash => "hash",
            Self::Ip => "ip",
            Self::Uuid => "uuid",
            Self::Command => "command",
            Self::Quoted => "quoted",
            Self::Number => "number",
            Self::Word => "word",
            Self::DockerImage => "docker-image",
            Self::KubernetesResource => "kubernetes-resource",
            Self::HexColor => "hex-color",
            Self::IpfsCid => "ipfs-cid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.label() == value)
    }
}

/// A parsed token extracted from pane scrollback or screen content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedToken {
    /// Semantic classification of the token.
    pub kind: EntityKind,
    /// Original raw text as captured from the terminal screen or scrollback.
    pub raw_text: String,
    /// Cleaned or normalized text suitable for opening, copying, or jumping.
    pub clean_text: String,
    /// Optional line number (e.g. parsed from compiler or ripgrep output).
    pub line_number: Option<usize>,
    /// Optional column number (e.g. parsed from compiler output).
    pub col_number: Option<usize>,
    /// Identifier of the tmux pane where this token was located.
    pub pane_id: String,
    /// 0-indexed vertical screen row on the terminal display.
    pub screen_row: usize,
    /// 0-indexed horizontal start column index (inclusive).
    pub col_start: usize,
    /// 0-indexed horizontal end column index (exclusive).
    pub col_end: usize,
}
