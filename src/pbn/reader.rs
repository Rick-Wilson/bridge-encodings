//! PBN file reader.

use crate::error::Result;
use bridge_types::{Board, Deal, Direction, Vulnerability};

/// A parsed PBN tag pair
#[derive(Debug, Clone)]
pub struct TagPair {
    pub name: String,
    pub value: String,
}

/// Parse a tag pair from a line: [TagName "value"]
fn parse_tag_pair(line: &str) -> Option<TagPair> {
    let line = line.trim();
    if !line.starts_with('[') || !line.ends_with(']') {
        return None;
    }

    let inner = &line[1..line.len() - 1];

    // Find the space between tag name and quoted value
    let space_pos = inner.find(' ')?;
    let name = inner[..space_pos].trim().to_string();
    let rest = inner[space_pos..].trim();

    // Extract quoted value
    if !rest.starts_with('"') || !rest.ends_with('"') {
        return None;
    }
    let value = rest[1..rest.len() - 1].to_string();

    Some(TagPair { name, value })
}

/// Read boards from PBN content
pub fn read_pbn(content: &str) -> Result<Vec<Board>> {
    let mut boards = Vec::new();
    let mut current_board = Board::new();
    let mut has_content = false;
    let mut in_commentary = false;

    for line in content.lines() {
        let line = line.trim();

        // Track multi-line commentary blocks { ... }
        if in_commentary {
            if line.contains('}') {
                in_commentary = false;
            }
            continue;
        }

        // Check for start of commentary
        if line.starts_with('{') {
            if !line.contains('}') {
                in_commentary = true;
            }
            continue;
        }

        // Empty line may signal end of board
        if line.is_empty() {
            if has_content {
                boards.push(current_board);
                current_board = Board::new();
                has_content = false;
            }
            continue;
        }

        // Skip line comments and directives
        if line.starts_with(';') || line.starts_with('%') {
            continue;
        }

        // Parse tag pair
        if line.starts_with('[') {
            if let Some(tag) = parse_tag_pair(line) {
                has_content = true;
                apply_tag_to_board(&mut current_board, &tag);
            }
        }
    }

    // Don't forget the last board
    if has_content {
        boards.push(current_board);
    }

    Ok(boards)
}

/// Apply a parsed tag to a board
fn apply_tag_to_board(board: &mut Board, tag: &TagPair) {
    match tag.name.as_str() {
        "Board" => {
            if let Ok(num) = tag.value.parse::<u32>() {
                board.number = Some(num);
            }
        }
        "Dealer" => {
            if let Some(c) = tag.value.chars().next() {
                board.dealer = Direction::from_char(c);
            }
        }
        "Vulnerable" => {
            board.vulnerable = Vulnerability::from_pbn(&tag.value).unwrap_or_default();
        }
        "Deal" => {
            if let Some(deal) = Deal::from_pbn(&tag.value) {
                board.deal = deal;
            }
        }
        "Event" => {
            if !tag.value.is_empty() {
                board.event = Some(tag.value.clone());
            }
        }
        "Site" => {
            if !tag.value.is_empty() {
                board.site = Some(tag.value.clone());
            }
        }
        "Date" => {
            if !tag.value.is_empty() {
                board.date = Some(tag.value.clone());
            }
        }
        "DoubleDummyTricks" => {
            board.double_dummy_tricks = Some(tag.value.clone());
        }
        "OptimumScore" => {
            board.optimum_score = Some(tag.value.clone());
        }
        "ParContract" => {
            board.par_contract = Some(tag.value.clone());
        }
        _ => {
            // Ignore other tags
        }
    }
}

/// Read boards from a PBN file
pub fn read_pbn_file(path: &std::path::Path) -> Result<Vec<Board>> {
    let content = std::fs::read_to_string(path)?;
    read_pbn(&content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag_pair() {
        let tag = parse_tag_pair("[Board \"1\"]").unwrap();
        assert_eq!(tag.name, "Board");
        assert_eq!(tag.value, "1");

        let tag = parse_tag_pair("[Vulnerable \"NS\"]").unwrap();
        assert_eq!(tag.name, "Vulnerable");
        assert_eq!(tag.value, "NS");
    }

    #[test]
    fn test_read_simple_pbn() {
        let pbn = r#"
[Board "1"]
[Dealer "N"]
[Vulnerable "None"]
[Deal "N:K843.T542.J6.863 AQJ7.K.Q75.AT942 962.AJ7.KT82.J75 T5.Q9863.A943.KQ"]
"#;
        let boards = read_pbn(pbn).unwrap();
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].number, Some(1));
        assert_eq!(boards[0].dealer, Some(Direction::North));
        assert_eq!(boards[0].vulnerable, Vulnerability::None);
    }

    #[test]
    fn test_read_multiple_boards() {
        let pbn = r#"
[Board "1"]
[Dealer "N"]
[Vulnerable "None"]
[Deal "N:K843.T542.J6.863 AQJ7.K.Q75.AT942 962.AJ7.KT82.J75 T5.Q9863.A943.KQ"]

[Board "2"]
[Dealer "E"]
[Vulnerable "NS"]
[Deal "E:Q7.AKT9.JT3.JT96 J653.QJ8.A.AQ732 K92.654.K954.K84 AT84.732.Q8762.5"]
"#;
        let boards = read_pbn(pbn).unwrap();
        assert_eq!(boards.len(), 2);
        assert_eq!(boards[0].number, Some(1));
        assert_eq!(boards[1].number, Some(2));
        assert_eq!(boards[1].dealer, Some(Direction::East));
        assert_eq!(boards[1].vulnerable, Vulnerability::NorthSouth);
    }

    #[test]
    fn test_read_pbn_with_commentary() {
        let pbn = r#"
[Board "1"]
[Dealer "N"]
[Vulnerable "None"]
[Deal "N:K843.T542.J6.863 AQJ7.K.Q75.AT942 962.AJ7.KT82.J75 T5.Q9863.A943.KQ"]
{This is a multi-line
commentary that spans
several lines.}

[Board "2"]
[Dealer "E"]
[Vulnerable "NS"]
[Deal "E:Q7.AKT9.JT3.JT96 J653.QJ8.A.AQ732 K92.654.K954.K84 AT84.732.Q8762.5"]
"#;
        let boards = read_pbn(pbn).unwrap();
        assert_eq!(boards.len(), 2);
    }
}
