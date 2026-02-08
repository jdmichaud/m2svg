//! Class diagram parser

use crate::types::{
    ClassDiagram, ClassMember, ClassNamespace, ClassNode, ClassNote, ClassRelationship,
    RelationshipType, Visibility,
};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    static ref RE_ANNOTATION: Regex = Regex::new(r"^<<(\w+)>>$").unwrap();
    static ref RE_SEPARATE_ANNOTATION: Regex = Regex::new(r"^<<(\w+)>>\s+(\S+)$").unwrap();
    static ref RE_NAMESPACE: Regex = Regex::new(r"^namespace\s+(\S+)\s*\{$").unwrap();
    static ref RE_CLASS_BLOCK: Regex = Regex::new(r"^class\s+(\S+?)(?:\s*~(\w+)~)?\s*\{$").unwrap();
    static ref RE_CLASS_ONLY: Regex = Regex::new(r"^class\s+(\S+?)(?:\s*~(\w+)~)?\s*$").unwrap();
    static ref RE_INLINE_ANNOT: Regex =
        Regex::new(r"^class\s+(\S+?)\s*\{\s*<<(\w+)>>\s*\}$").unwrap();
    static ref RE_INLINE_ATTR: Regex = Regex::new(r"^(\S+?)\s*:\s*(.+)$").unwrap();
    static ref RE_METHOD: Regex = Regex::new(r"^(.+?)\(([^)]*)\)(?:\s*(.+))?$").unwrap();
    static ref RE_ATTR: Regex = Regex::new(r"^(\S+)\s+(.+)$").unwrap();
    static ref RE_DIRECTION: Regex = Regex::new(r"^direction\s+(TB|BT|LR|RL)$").unwrap();
    static ref RE_NOTE_GENERAL: Regex = Regex::new(r#"^note\s+"([^"]+)"$"#).unwrap();
    static ref RE_NOTE_FOR: Regex = Regex::new(r#"^note\s+for\s+(\S+)\s+"([^"]+)"$"#).unwrap();
    static ref RE_LOLLIPOP_RIGHT: Regex = Regex::new(r"^(\S+)\s+--\(\)\s+(\S+)$").unwrap();
    static ref RE_LOLLIPOP_LEFT: Regex = Regex::new(r"^(\S+)\s+\(\)--\s+(\S+)$").unwrap();
    static ref RE_CLASS_INLINE_ANNOT: Regex = Regex::new(r"^class\s+(\S+?)\s+<<(\w+)>>$").unwrap();
}

/// Parse a Mermaid class diagram
pub fn parse_class_diagram(lines: &[&str]) -> Result<ClassDiagram, String> {
    let mut diagram = ClassDiagram::new();
    let mut class_map: HashMap<String, ClassNode> = HashMap::new();
    let mut class_order: Vec<String> = Vec::new(); // Track insertion order
    let mut current_namespace: Option<ClassNamespace> = None;
    let mut current_class: Option<String> = None;
    let mut brace_depth = 0;

    for line in lines.iter().skip(1) {
        let line = *line;

        // Inside a class body block
        if current_class.is_some() && brace_depth > 0 {
            if line == "}" {
                brace_depth -= 1;
                if brace_depth == 0 {
                    current_class = None;
                }
                continue;
            }

            // Check for annotation like <<interface>>
            if let Some(caps) = RE_ANNOTATION.captures(line) {
                if let Some(ref class_id) = current_class {
                    if let Some(cls) = class_map.get_mut(class_id) {
                        cls.annotation = Some(caps[1].to_string());
                    }
                }
                continue;
            }

            // Parse member
            if let Some(parsed) = parse_member(line) {
                if let Some(ref class_id) = current_class {
                    if let Some(cls) = class_map.get_mut(class_id) {
                        if parsed.is_method {
                            cls.methods.push(parsed.member);
                        } else {
                            cls.attributes.push(parsed.member);
                        }
                    }
                }
            }
            continue;
        }

        // Namespace block start
        if let Some(caps) = RE_NAMESPACE.captures(line) {
            current_namespace = Some(ClassNamespace {
                name: caps[1].to_string(),
                class_ids: Vec::new(),
            });
            continue;
        }

        // Namespace end
        if line == "}" && current_namespace.is_some() {
            if let Some(ns) = current_namespace.take() {
                diagram.namespaces.push(ns);
            }
            continue;
        }

        // Direction keyword: `direction RL`
        if let Some(caps) = RE_DIRECTION.captures(line) {
            diagram.direction = caps[1].to_string();
            continue;
        }

        // Note for a specific class: `note for Duck "can fly"`
        if let Some(caps) = RE_NOTE_FOR.captures(line) {
            diagram.notes.push(ClassNote {
                text: caps[2].to_string(),
                for_class: Some(caps[1].to_string()),
            });
            continue;
        }

        // General note: `note "This is a general note"`
        if let Some(caps) = RE_NOTE_GENERAL.captures(line) {
            diagram.notes.push(ClassNote {
                text: caps[1].to_string(),
                for_class: None,
            });
            continue;
        }

        // Class block start: `class ClassName {` or `class ClassName~Generic~ {`
        if let Some(caps) = RE_CLASS_BLOCK.captures(line) {
            let id = caps[1].to_string();
            let generic = caps.get(2).map(|m| m.as_str());

            let cls = ensure_class(&mut class_map, &mut class_order, &id);
            if let Some(g) = generic {
                cls.label = format!("{}<{}>", id, g);
            }
            current_class = Some(id.clone());
            brace_depth = 1;

            if let Some(ref mut ns) = current_namespace {
                ns.class_ids.push(id);
            }
            continue;
        }

        // Standalone class declaration (no body)
        if let Some(caps) = RE_CLASS_ONLY.captures(line) {
            let id = caps[1].to_string();
            let generic = caps.get(2).map(|m| m.as_str());

            let cls = ensure_class(&mut class_map, &mut class_order, &id);
            if let Some(g) = generic {
                cls.label = format!("{}<{}>", id, g);
            }

            if let Some(ref mut ns) = current_namespace {
                ns.class_ids.push(id);
            }
            continue;
        }

        // Inline annotation: `class ClassName { <<interface>> }`
        if let Some(caps) = RE_INLINE_ANNOT.captures(line) {
            let cls = ensure_class(&mut class_map, &mut class_order, &caps[1]);
            cls.annotation = Some(caps[2].to_string());
            continue;
        }

        // Class with inline annotation: `class Shape <<interface>>`
        if let Some(caps) = RE_CLASS_INLINE_ANNOT.captures(line) {
            let cls = ensure_class(&mut class_map, &mut class_order, &caps[1]);
            cls.annotation = Some(caps[2].to_string());
            continue;
        }

        // Separate annotation: `<<interface>> Shape`
        if let Some(caps) = RE_SEPARATE_ANNOTATION.captures(line) {
            let cls = ensure_class(&mut class_map, &mut class_order, &caps[2]);
            cls.annotation = Some(caps[1].to_string());
            continue;
        }

        // Lollipop interface: `Class01 --() bar`
        if let Some(caps) = RE_LOLLIPOP_RIGHT.captures(line) {
            let from = caps[1].to_string();
            let to = caps[2].to_string();
            ensure_class(&mut class_map, &mut class_order, &from);
            ensure_class(&mut class_map, &mut class_order, &to);
            diagram.relationships.push(ClassRelationship {
                from: from.clone(),
                to: to.clone(),
                rel_type: RelationshipType::Association,
                from_cardinality: None,
                to_cardinality: None,
                label: None,
                marker_at_from: false,
            });
            continue;
        }

        // Lollipop interface: `foo ()-- Class01`
        if let Some(caps) = RE_LOLLIPOP_LEFT.captures(line) {
            let from = caps[1].to_string();
            let to = caps[2].to_string();
            ensure_class(&mut class_map, &mut class_order, &from);
            ensure_class(&mut class_map, &mut class_order, &to);
            diagram.relationships.push(ClassRelationship {
                from: from.clone(),
                to: to.clone(),
                rel_type: RelationshipType::Association,
                from_cardinality: None,
                to_cardinality: None,
                label: None,
                marker_at_from: false,
            });
            continue;
        }

        // Inline attribute: `ClassName : +String name`
        if let Some(caps) = RE_INLINE_ATTR.captures(line) {
            let rest = &caps[2];
            // Make sure this isn't a relationship line
            if !rest.contains("<|--")
                && !rest.contains("--")
                && !rest.contains("*--")
                && !rest.contains("o--")
                && !rest.contains("-->")
                && !rest.contains("..>")
                && !rest.contains("..|>")
            {
                let cls = ensure_class(&mut class_map, &mut class_order, &caps[1]);
                if let Some(parsed) = parse_member(rest) {
                    if parsed.is_method {
                        cls.methods.push(parsed.member);
                    } else {
                        cls.attributes.push(parsed.member);
                    }
                }
                continue;
            }
        }

        // Relationship
        if let Some(rel) = parse_relationship(line) {
            ensure_class(&mut class_map, &mut class_order, &rel.from);
            ensure_class(&mut class_map, &mut class_order, &rel.to);
            diagram.relationships.push(rel);
            continue;
        }
    }

    // Convert to ordered list
    diagram.classes = class_order
        .iter()
        .filter_map(|id| class_map.remove(id))
        .collect();
    Ok(diagram)
}

fn ensure_class<'a>(
    class_map: &'a mut HashMap<String, ClassNode>,
    class_order: &mut Vec<String>,
    id: &str,
) -> &'a mut ClassNode {
    if !class_map.contains_key(id) {
        class_map.insert(
            id.to_string(),
            ClassNode {
                id: id.to_string(),
                label: id.to_string(),
                attributes: Vec::new(),
                methods: Vec::new(),
                annotation: None,
            },
        );
        class_order.push(id.to_string());
    }
    class_map.get_mut(id).unwrap()
}

struct ParsedMember {
    member: ClassMember,
    is_method: bool,
}

fn parse_member(line: &str) -> Option<ParsedMember> {
    let trimmed = line.trim().trim_end_matches(';');
    if trimmed.is_empty() {
        return None;
    }

    // Convert ~ generics to <> before parsing
    let converted = convert_tilde_generics(trimmed);
    let trimmed = converted.as_str();

    // Extract visibility prefix
    let (visibility, rest) = if let Some(first_char) = trimmed.chars().next() {
        if matches!(first_char, '+' | '-' | '#' | '~') {
            (Visibility::from_char(first_char), trimmed[1..].trim())
        } else {
            (Visibility::None, trimmed)
        }
    } else {
        (Visibility::None, trimmed)
    };

    // Check if it's a method (has parentheses)
    if let Some(caps) = RE_METHOD.captures(rest) {
        let name = caps[1].trim().to_string();
        let type_str = caps.get(3).map(|m| m.as_str().trim().to_string());

        let is_static = name.ends_with('$') || rest.contains('$');
        let is_abstract = name.ends_with('*') || rest.contains('*');

        return Some(ParsedMember {
            member: ClassMember {
                visibility,
                name: name.trim_end_matches(['$', '*']).to_string(),
                member_type: type_str,
                is_static,
                is_abstract,
            },
            is_method: true,
        });
    }

    // Attribute: might be "Type name" or "name : Type" or just "name"
    // Check for "name : type" format first (colon-separated)
    if rest.contains(" : ") {
        let parts: Vec<&str> = rest.splitn(2, " : ").collect();
        if parts.len() == 2 {
            let name = parts[0].trim();
            let type_str = parts[1].trim();
            let is_static = type_str.ends_with('$');
            let is_abstract = type_str.ends_with('*');
            return Some(ParsedMember {
                member: ClassMember {
                    visibility,
                    name: name.trim_end_matches(['$', '*']).to_string(),
                    member_type: Some(type_str.trim_end_matches(['$', '*']).to_string()),
                    is_static,
                    is_abstract,
                },
                is_method: false,
            });
        }
    }

    if let Some(caps) = RE_ATTR.captures(rest) {
        let first = caps[1].trim();
        let second = caps[2].trim();

        // "Type name" format
        let is_static = second.ends_with('$');
        let is_abstract = second.ends_with('*');

        return Some(ParsedMember {
            member: ClassMember {
                visibility,
                name: second.trim_end_matches(['$', '*']).to_string(),
                member_type: Some(first.to_string()),
                is_static,
                is_abstract,
            },
            is_method: false,
        });
    }

    // Just a name
    let is_static = rest.ends_with('$');
    let is_abstract = rest.ends_with('*');

    Some(ParsedMember {
        member: ClassMember {
            visibility,
            name: rest.trim_end_matches(['$', '*']).to_string(),
            member_type: None,
            is_static,
            is_abstract,
        },
        is_method: false,
    })
}

/// Convert Mermaid `~` generic notation to `<>` notation.
/// e.g., `List~int~` → `List<int>`, `List~List~int~~` → `List<List<int>>`
fn convert_tilde_generics(s: &str) -> String {
    if !s.contains('~') {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut depth = 0;

    while i < chars.len() {
        if chars[i] == '~' {
            // Look at context: if next char is `~` or end-of-string, it's a closing `>`
            // If next char is alphanumeric or `(`, it's an opening `<`
            if depth > 0
                && (i + 1 >= chars.len()
                    || chars[i + 1] == '~'
                    || chars[i + 1] == ')'
                    || chars[i + 1] == ' '
                    || chars[i + 1] == ',')
            {
                result.push('>');
                depth -= 1;
            } else {
                result.push('<');
                depth += 1;
            }
        } else {
            result.push(chars[i]);
        }
        i += 1;
    }

    result
}

fn parse_relationship(line: &str) -> Option<ClassRelationship> {
    // Pattern: [FROM] ["card1"] ARROW ["card2"] [TO] [: label]
    // Arrows: <|--, *--, o--, -->, ..|>, ..>, --, ..
    // marker_at_from: true = marker at 'from' end, false = marker at 'to' end

    let arrow_patterns: &[(&str, RelationshipType, bool)] = &[
        // Prefix markers - marker at 'from' side
        ("<|--", RelationshipType::Inheritance, true),
        ("*--", RelationshipType::Composition, true),
        ("o--", RelationshipType::Aggregation, true),
        // Suffix markers - marker at 'to' side
        ("-->", RelationshipType::Association, false),
        ("..>", RelationshipType::Dependency, false),
        ("..|>", RelationshipType::Realization, false),
        // Reversed patterns
        ("--|>", RelationshipType::Inheritance, false),
        ("--*", RelationshipType::Composition, false),
        ("--o", RelationshipType::Aggregation, false),
        ("<--", RelationshipType::Association, true),
        ("<..", RelationshipType::Dependency, true),
        ("<|..", RelationshipType::Realization, true),
        // Plain links (must come after longer patterns)
        ("--", RelationshipType::Association, false),
        ("..", RelationshipType::Dependency, false),
    ];

    for &(arrow, rel_type, marker_at_from) in arrow_patterns {
        // Try to find the arrow in the line, with spaces around it
        // Must handle: FROM ARROW TO, FROM "card" ARROW "card" TO
        let arrow_escaped = regex::escape(arrow);
        let pattern = format!(
            r#"^(\S+)\s+(?:"([^"]*)")?\s*{}\s*(?:"([^"]*)")?\s*(\S+)(?:\s*:\s*(.+))?$"#,
            arrow_escaped
        );

        if let Some(caps) = Regex::new(&pattern).ok()?.captures(line) {
            let from = caps[1].to_string();
            let from_card = caps.get(2).map(|m| m.as_str().to_string());
            let to_card = caps.get(3).map(|m| m.as_str().to_string());
            let to = caps[4].to_string();
            let label = caps.get(5).map(|m| m.as_str().trim().to_string());

            return Some(ClassRelationship {
                from,
                to,
                rel_type,
                from_cardinality: from_card,
                to_cardinality: to_card,
                label,
                marker_at_from,
            });
        }
    }

    None
}
