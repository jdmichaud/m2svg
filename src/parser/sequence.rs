//! Sequence diagram parser

use crate::types::{
    Actor, ActorType, ArrowHead, Block, BlockDivider, BlockType, LineStyle, Message, Note,
    NotePosition, SequenceDiagram,
};
use regex::Regex;
use std::collections::HashSet;

/// Parse a Mermaid sequence diagram
pub fn parse_sequence_diagram(lines: &[&str]) -> Result<SequenceDiagram, String> {
    let mut diagram = SequenceDiagram::new();
    let mut actor_ids: HashSet<String> = HashSet::new();
    let mut block_stack: Vec<BlockStackEntry> = Vec::new();

    for line in lines.iter().skip(1) {
        let line = *line;

        // Participant / Actor declaration
        let actor_re = Regex::new(r"^(participant|actor)\s+(\S+?)(?:\s+as\s+(.+))?$").unwrap();
        if let Some(caps) = actor_re.captures(line) {
            let type_str = &caps[1];
            let id = caps[2].to_string();
            let label = caps
                .get(3)
                .map(|m| m.as_str().trim())
                .unwrap_or(&id)
                .to_string();

            if !actor_ids.contains(&id) {
                actor_ids.insert(id.clone());
                diagram.actors.push(Actor {
                    id,
                    label,
                    actor_type: if type_str == "actor" {
                        ActorType::Actor
                    } else {
                        ActorType::Participant
                    },
                });
            }
            continue;
        }

        // Note
        let note_re =
            Regex::new(r"(?i)^Note\s+(left of|right of|over)\s+([^:]+):\s*(.+)$").unwrap();
        if let Some(caps) = note_re.captures(line) {
            let pos_str = caps[1].to_lowercase();
            let actors_str = caps[2].trim();
            let text = caps[3].trim().to_string();

            let note_actor_ids: Vec<String> = actors_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            // Ensure actors exist
            for aid in &note_actor_ids {
                ensure_actor(&mut diagram, &mut actor_ids, aid);
            }

            let position = match pos_str.as_str() {
                "left of" => NotePosition::Left,
                "right of" => NotePosition::Right,
                _ => NotePosition::Over,
            };

            diagram.notes.push(Note {
                actor_ids: note_actor_ids,
                text,
                position,
                after_index: diagram.messages.len() as i32 - 1,
            });
            continue;
        }

        // Block start
        let block_re = Regex::new(r"^(loop|alt|opt|par|critical|break|rect)\s*(.*)$").unwrap();
        if let Some(caps) = block_re.captures(line) {
            let block_type = match &caps[1] {
                "loop" => BlockType::Loop,
                "alt" => BlockType::Alt,
                "opt" => BlockType::Opt,
                "par" => BlockType::Par,
                "critical" => BlockType::Critical,
                "break" => BlockType::Break,
                "rect" => BlockType::Rect,
                _ => BlockType::Loop,
            };
            let label = caps
                .get(2)
                .map(|m| m.as_str().trim())
                .unwrap_or("")
                .to_string();

            block_stack.push(BlockStackEntry {
                block_type,
                label,
                start_index: diagram.messages.len(),
                dividers: Vec::new(),
            });
            continue;
        }

        // Block divider
        let divider_re = Regex::new(r"^(else|and)\s*(.*)$").unwrap();
        if let Some(caps) = divider_re.captures(line) {
            if let Some(current) = block_stack.last_mut() {
                let label = caps
                    .get(2)
                    .map(|m| m.as_str().trim())
                    .unwrap_or("")
                    .to_string();
                current.dividers.push(BlockDivider {
                    index: diagram.messages.len(),
                    label,
                });
            }
            continue;
        }

        // Block end
        if line == "end" {
            if let Some(completed) = block_stack.pop() {
                diagram.blocks.push(Block {
                    block_type: completed.block_type,
                    label: completed.label,
                    start_index: completed.start_index,
                    end_index: diagram
                        .messages
                        .len()
                        .saturating_sub(1)
                        .max(completed.start_index),
                    dividers: completed.dividers,
                });
            }
            continue;
        }

        // Message patterns
        let msg_re =
            Regex::new(r"^(\S+?)\s*(--?>?>|--?[)x]|--?>>|--?>)\s*([+-]?)(\S+?)\s*:\s*(.+)$")
                .unwrap();
        if let Some(caps) = msg_re.captures(line) {
            let from = caps[1].to_string();
            let arrow = &caps[2];
            let activation_mark = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let to = caps[4].to_string();
            let label = caps[5].trim().to_string();

            ensure_actor(&mut diagram, &mut actor_ids, &from);
            ensure_actor(&mut diagram, &mut actor_ids, &to);

            let line_style = if arrow.starts_with("--") {
                LineStyle::Dashed
            } else {
                LineStyle::Solid
            };
            let arrow_head = if arrow.contains(">>") || arrow.contains('x') {
                ArrowHead::Filled
            } else {
                ArrowHead::Open
            };

            diagram.messages.push(Message {
                from,
                to,
                label,
                line_style,
                arrow_head,
                activate: activation_mark == "+",
                deactivate: activation_mark == "-",
            });
            continue;
        }

        // Simplified message format
        let simple_msg_re =
            Regex::new(r"^(\S+?)\s*(->>|-->>|-\)|--\)|-x|--x|->|-->)\s*([+-]?)(\S+?)\s*:\s*(.+)$")
                .unwrap();
        if let Some(caps) = simple_msg_re.captures(line) {
            let from = caps[1].to_string();
            let arrow = &caps[2];
            let activation_mark = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let to = caps[4].to_string();
            let label = caps[5].trim().to_string();

            ensure_actor(&mut diagram, &mut actor_ids, &from);
            ensure_actor(&mut diagram, &mut actor_ids, &to);

            let line_style = if arrow.starts_with("--") {
                LineStyle::Dashed
            } else {
                LineStyle::Solid
            };
            let arrow_head = if arrow.contains(">>") || arrow.contains('x') {
                ArrowHead::Filled
            } else {
                ArrowHead::Open
            };

            diagram.messages.push(Message {
                from,
                to,
                label,
                line_style,
                arrow_head,
                activate: activation_mark == "+",
                deactivate: activation_mark == "-",
            });
            continue;
        }
    }

    Ok(diagram)
}

struct BlockStackEntry {
    block_type: BlockType,
    label: String,
    start_index: usize,
    dividers: Vec<BlockDivider>,
}

fn ensure_actor(diagram: &mut SequenceDiagram, actor_ids: &mut HashSet<String>, id: &str) {
    if !actor_ids.contains(id) {
        actor_ids.insert(id.to_string());
        diagram.actors.push(Actor {
            id: id.to_string(),
            label: id.to_string(),
            actor_type: ActorType::Participant,
        });
    }
}
