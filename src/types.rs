//! Type definitions for Mermaid graph structures

use std::collections::HashMap;

/// The direction of a flowchart/graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TD, // Top-Down (same as TB)
    TB, // Top-Bottom
    LR, // Left-Right
    BT, // Bottom-Top
    RL, // Right-Left
}

impl Direction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TD" => Some(Direction::TD),
            "TB" => Some(Direction::TB),
            "LR" => Some(Direction::LR),
            "BT" => Some(Direction::BT),
            "RL" => Some(Direction::RL),
            _ => None,
        }
    }
}

/// Shape of a node in the diagram
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Rectangle,   // [text]
    Rounded,     // (text)
    Diamond,     // {text}
    Stadium,     // ([text])
    Circle,      // ((text))
    Subroutine,  // [[text]]
    DoubleCircle, // (((text)))
    Hexagon,     // {{text}}
    Cylinder,    // [(text)]
    Asymmetric,  // >text]
    Trapezoid,   // [/text\]
    TrapezoidAlt, // [\text/]
    StateStart,  // filled circle (start pseudostate)
    StateEnd,    // bullseye circle (end pseudostate)
}

/// Style of an edge/connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
}

/// A node in the Mermaid graph
#[derive(Debug, Clone)]
pub struct MermaidNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
}

/// An edge between two nodes
#[derive(Debug, Clone)]
pub struct MermaidEdge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
    pub has_arrow_start: bool,
    pub has_arrow_end: bool,
}

/// A subgraph container
#[derive(Debug, Clone)]
pub struct MermaidSubgraph {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<String>,
    pub children: Vec<MermaidSubgraph>,
    pub direction: Option<Direction>,
}

/// The complete parsed Mermaid graph
#[derive(Debug, Clone)]
pub struct MermaidGraph {
    pub direction: Direction,
    pub nodes: HashMap<String, MermaidNode>,
    pub node_order: Vec<String>,  // Track insertion order
    pub edges: Vec<MermaidEdge>,
    pub subgraphs: Vec<MermaidSubgraph>,
    pub class_defs: HashMap<String, HashMap<String, String>>,
    pub class_assignments: HashMap<String, String>,
    pub node_styles: HashMap<String, HashMap<String, String>>,
}

impl MermaidGraph {
    pub fn new(direction: Direction) -> Self {
        Self {
            direction,
            nodes: HashMap::new(),
            node_order: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            class_defs: HashMap::new(),
            class_assignments: HashMap::new(),
            node_styles: HashMap::new(),
        }
    }
}

// ============================================================================
// Sequence diagram types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorType {
    Participant,
    Actor,
}

#[derive(Debug, Clone)]
pub struct Actor {
    pub id: String,
    pub label: String,
    pub actor_type: ActorType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowHead {
    Filled,
    Open,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub line_style: LineStyle,
    pub arrow_head: ArrowHead,
    pub activate: bool,
    pub deactivate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
    Rect,
}

#[derive(Debug, Clone)]
pub struct BlockDivider {
    pub index: usize,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub block_type: BlockType,
    pub label: String,
    pub start_index: usize,
    pub end_index: usize,
    pub dividers: Vec<BlockDivider>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotePosition {
    Left,
    Right,
    Over,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub actor_ids: Vec<String>,
    pub text: String,
    pub position: NotePosition,
    pub after_index: i32,
}

#[derive(Debug, Clone)]
pub struct SequenceDiagram {
    pub actors: Vec<Actor>,
    pub messages: Vec<Message>,
    pub blocks: Vec<Block>,
    pub notes: Vec<Note>,
}

impl SequenceDiagram {
    pub fn new() -> Self {
        Self {
            actors: Vec::new(),
            messages: Vec::new(),
            blocks: Vec::new(),
            notes: Vec::new(),
        }
    }
}

// ============================================================================
// Class diagram types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,    // +
    Private,   // -
    Protected, // #
    Package,   // ~
    None,
}

impl Visibility {
    pub fn from_char(c: char) -> Self {
        match c {
            '+' => Visibility::Public,
            '-' => Visibility::Private,
            '#' => Visibility::Protected,
            '~' => Visibility::Package,
            _ => Visibility::None,
        }
    }
    
    pub fn to_char(&self) -> char {
        match self {
            Visibility::Public => '+',
            Visibility::Private => '-',
            Visibility::Protected => '#',
            Visibility::Package => '~',
            Visibility::None => ' ',
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClassMember {
    pub visibility: Visibility,
    pub name: String,
    pub member_type: Option<String>,
    pub is_static: bool,
    pub is_abstract: bool,
}

#[derive(Debug, Clone)]
pub struct ClassNode {
    pub id: String,
    pub label: String,
    pub attributes: Vec<ClassMember>,
    pub methods: Vec<ClassMember>,
    pub annotation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipType {
    Inheritance,   // <|--
    Composition,   // *--
    Aggregation,   // o--
    Association,   // -->
    Dependency,    // ..>
    Realization,   // ..|>
}

#[derive(Debug, Clone)]
pub struct ClassRelationship {
    pub from: String,
    pub to: String,
    pub rel_type: RelationshipType,
    pub from_cardinality: Option<String>,
    pub to_cardinality: Option<String>,
    pub label: Option<String>,
    pub marker_at_from: bool,  // true = marker at 'from' end, false = marker at 'to' end
}

#[derive(Debug, Clone)]
pub struct ClassNamespace {
    pub name: String,
    pub class_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ClassDiagram {
    pub classes: Vec<ClassNode>,
    pub relationships: Vec<ClassRelationship>,
    pub namespaces: Vec<ClassNamespace>,
}

impl ClassDiagram {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            relationships: Vec::new(),
            namespaces: Vec::new(),
        }
    }
}

// ============================================================================
// ER diagram types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErKey {
    PK, // Primary Key
    FK, // Foreign Key
    UK, // Unique Key
}

#[derive(Debug, Clone)]
pub struct ErAttribute {
    pub attr_type: String,
    pub name: String,
    pub keys: Vec<ErKey>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ErEntity {
    pub id: String,
    pub label: String,
    pub attributes: Vec<ErAttribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    One,      // ||   exactly one
    ZeroOne,  // o|   zero or one
    Many,     // }|   one or more
    ZeroMany, // o{   zero or more
}

impl Cardinality {
    pub fn to_str(&self) -> &'static str {
        match self {
            Cardinality::One => "||",
            Cardinality::ZeroOne => "o|",
            Cardinality::Many => "}|",
            Cardinality::ZeroMany => "o{",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErRelationship {
    pub entity1: String,
    pub entity2: String,
    pub cardinality1: Cardinality,
    pub cardinality2: Cardinality,
    pub label: String,
    pub identifying: bool,
}

#[derive(Debug, Clone)]
pub struct ErDiagram {
    pub entities: Vec<ErEntity>,
    pub relationships: Vec<ErRelationship>,
}

impl ErDiagram {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
        }
    }
}

// ============================================================================
// GitGraph types
// ============================================================================

/// Direction of the git graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitGraphDirection {
    LR, // Left to Right (default, horizontal)
    TB, // Top to Bottom (vertical)
    BT, // Bottom to Top (vertical, reversed)
}

impl GitGraphDirection {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "LR" => Some(GitGraphDirection::LR),
            "TB" => Some(GitGraphDirection::TB),
            "BT" => Some(GitGraphDirection::BT),
            _ => None,
        }
    }
}

/// Type of commit (affects visual styling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitType {
    Normal,
    Reverse,
    Highlight,
}

/// A commit in the git graph
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub id: String,           // Auto-generated (A, B, C...) or custom
    pub commit_type: CommitType,
    pub tag: Option<String>,
    pub branch: String,       // Which branch this commit is on
    pub parent_ids: Vec<String>, // Parent commit IDs (1 for normal, 2 for merge)
    pub is_merge: bool,
    pub is_cherry_pick: bool,
    pub cherry_pick_source: Option<String>,
}

/// A branch in the git graph
#[derive(Debug, Clone)]
pub struct GitBranch {
    pub name: String,
    pub order: Option<i32>,    // Custom ordering
    pub commit_ids: Vec<String>, // Commits on this branch
    pub source_commit: Option<String>, // The commit this branch was created from
}

/// The complete parsed GitGraph
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub direction: GitGraphDirection,
    pub commits: Vec<GitCommit>,
    pub branches: Vec<GitBranch>,
    pub current_branch: String,
}

impl GitGraph {
    pub fn new(direction: GitGraphDirection) -> Self {
        Self {
            direction,
            commits: Vec::new(),
            branches: vec![GitBranch {
                name: "main".to_string(),
                order: None,
                commit_ids: Vec::new(),
                source_commit: None,
            }],
            current_branch: "main".to_string(),
        }
    }
}

// ============================================================================
// Diagram type enum for dispatch
// ============================================================================

#[derive(Debug, Clone)]
pub enum DiagramType {
    Flowchart(MermaidGraph),
    Sequence(SequenceDiagram),
    Class(ClassDiagram),
    Er(ErDiagram),
    GitGraph(GitGraph),
}
