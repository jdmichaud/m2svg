//! ER diagram parser

use crate::types::{Cardinality, ErAttribute, ErDiagram, ErEntity, ErKey, ErRelationship};
use regex::Regex;
use std::collections::HashMap;

/// Parse a Mermaid ER diagram
pub fn parse_er_diagram(lines: &[&str]) -> Result<ErDiagram, String> {
    let mut diagram = ErDiagram::new();
    let mut entity_map: HashMap<String, ErEntity> = HashMap::new();
    let mut current_entity: Option<String> = None;
    
    for line in lines.iter().skip(1) {
        let line = *line;
        
        // Inside entity body
        if let Some(ref entity_id) = current_entity {
            if line == "}" {
                current_entity = None;
                continue;
            }
            
            // Attribute line: type name [PK|FK|UK] ["comment"]
            if let Some(attr) = parse_attribute(line) {
                if let Some(entity) = entity_map.get_mut(entity_id) {
                    entity.attributes.push(attr);
                }
            }
            continue;
        }
        
        // Entity block start: `ENTITY_NAME {`
        let entity_block_re = Regex::new(r"^(\S+)\s*\{$").unwrap();
        if let Some(caps) = entity_block_re.captures(line) {
            let id = caps[1].to_string();
            ensure_entity(&mut entity_map, &id);
            current_entity = Some(id);
            continue;
        }
        
        // Relationship: `ENTITY1 cardinality1--cardinality2 ENTITY2 : label`
        if let Some(rel) = parse_relationship_line(line) {
            ensure_entity(&mut entity_map, &rel.entity1);
            ensure_entity(&mut entity_map, &rel.entity2);
            diagram.relationships.push(rel);
            continue;
        }
    }
    
    diagram.entities = entity_map.into_values().collect();
    Ok(diagram)
}

fn ensure_entity(entity_map: &mut HashMap<String, ErEntity>, id: &str) {
    if !entity_map.contains_key(id) {
        entity_map.insert(id.to_string(), ErEntity {
            id: id.to_string(),
            label: id.to_string(),
            attributes: Vec::new(),
        });
    }
}

fn parse_attribute(line: &str) -> Option<ErAttribute> {
    // Format: type name [PK|FK|UK [...]] ["comment"]
    let re = Regex::new(r"^(\S+)\s+(\S+)(?:\s+(.+))?$").unwrap();
    let caps = re.captures(line)?;
    
    let attr_type = caps[1].to_string();
    let name = caps[2].to_string();
    let rest = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");
    
    // Extract quoted comment first
    let comment_re = Regex::new(r#""([^"]*)""#).unwrap();
    let comment = comment_re.captures(rest).map(|c| c[1].to_string());
    
    // Extract key constraints
    let rest_without_comment = comment_re.replace_all(rest, "");
    let mut keys = Vec::new();
    for part in rest_without_comment.split_whitespace() {
        let upper = part.to_uppercase();
        match upper.as_str() {
            "PK" => keys.push(ErKey::PK),
            "FK" => keys.push(ErKey::FK),
            "UK" => keys.push(ErKey::UK),
            _ => {}
        }
    }
    
    Some(ErAttribute {
        attr_type,
        name,
        keys,
        comment,
    })
}

fn parse_relationship_line(line: &str) -> Option<ErRelationship> {
    // Match: ENTITY1 <cardinality_and_line> ENTITY2 : label
    let re = Regex::new(r"^(\S+)\s+([|o}{]+(?:--|\.\.)[|o}{]+)\s+(\S+)\s*:\s*(.+)$").unwrap();
    let caps = re.captures(line)?;
    
    let entity1 = caps[1].to_string();
    let cardinality_str = &caps[2];
    let entity2 = caps[3].to_string();
    let label = caps[4].trim().to_string();
    
    // Split the cardinality string into left side, line style, right side
    let line_re = Regex::new(r"^([|o}{]+)(--|\.\.?)([|o}{]+)$").unwrap();
    let line_caps = line_re.captures(cardinality_str)?;
    
    let left_str = &line_caps[1];
    let line_style = &line_caps[2];
    let right_str = &line_caps[3];
    
    let cardinality1 = parse_cardinality(left_str)?;
    let cardinality2 = parse_cardinality(right_str)?;
    let identifying = line_style == "--";
    
    Some(ErRelationship {
        entity1,
        entity2,
        cardinality1,
        cardinality2,
        label,
        identifying,
    })
}

fn parse_cardinality(s: &str) -> Option<Cardinality> {
    // Normalize: sort the characters to handle both orders
    let mut chars: Vec<char> = s.chars().collect();
    chars.sort();
    let sorted: String = chars.iter().collect();
    
    match sorted.as_str() {
        "||" => Some(Cardinality::One),
        "|o" | "o|" => Some(Cardinality::ZeroOne),
        "|}" | "{|" => Some(Cardinality::Many),
        "{o" | "o{" => Some(Cardinality::ZeroMany),
        _ => None,
    }
}
