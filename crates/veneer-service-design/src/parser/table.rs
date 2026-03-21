/// Represents a parsed markdown table with headers and rows.
#[derive(Debug, Clone)]
pub struct MarkdownTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Represents a markdown heading section with its content.
#[derive(Debug, Clone)]
pub struct Section {
    pub heading: String,
    pub level: u8,
    pub content: String,
}

/// Parse a single row of pipe-delimited cells, trimming whitespace.
fn parse_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip leading/trailing pipes
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or(trimmed);
    inner
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

/// Returns true if the line is a separator row (e.g., |---|---|).
fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return false;
    }
    let inner = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or(trimmed);
    inner.split('|').all(|cell| {
        let cell = cell.trim();
        !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':')
    })
}

/// Extract all markdown tables from content. Single-pass parsing.
pub fn extract_tables(content: &str) -> Vec<MarkdownTable> {
    let mut tables = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for a header row followed by a separator row
        if i + 1 < lines.len() && lines[i].trim().contains('|') && is_separator_row(lines[i + 1]) {
            let headers = parse_row(lines[i]);
            // Skip separator
            i += 2;

            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().contains('|') && !is_separator_row(lines[i]) {
                let row = parse_row(lines[i]);
                rows.push(row);
                i += 1;
            }

            tables.push(MarkdownTable { headers, rows });
        } else {
            i += 1;
        }
    }

    tables
}

/// Extract all heading-delimited sections from markdown content.
/// Each heading becomes a section whose content extends until the next
/// heading of the same or higher (lower number) level.
pub fn extract_sections(content: &str) -> Vec<Section> {
    let lines: Vec<&str> = content.lines().collect();

    // First pass: find all heading positions
    let mut heading_positions: Vec<(usize, u8, String)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(level) = heading_level(trimmed) {
            let heading = trimmed.trim_start_matches('#').trim().to_string();
            heading_positions.push((i, level, heading));
        }
    }

    // Second pass: build sections
    let mut sections = Vec::new();
    for (idx, (line_idx, level, heading)) in heading_positions.iter().enumerate() {
        let start = line_idx + 1;
        let end = heading_positions
            .get(idx + 1)
            .map(|(next_line, _, _)| *next_line)
            .unwrap_or(lines.len());

        let content_str = if start < end {
            lines[start..end].join("\n")
        } else {
            String::new()
        };

        sections.push(Section {
            heading: heading.clone(),
            level: *level,
            content: content_str,
        });
    }

    sections
}

/// Returns the heading level (1-6) if the line starts with # characters.
fn heading_level(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level > 6 {
        return None;
    }
    // Must have a space after the # characters
    if trimmed.len() > level && trimmed.as_bytes()[level] == b' ' {
        Some(level as u8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_table() {
        let md = "\
| Name | Age |
| --- | --- |
| Alice | 30 |
| Bob | 25 |
";
        let tables = extract_tables(md);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].headers, vec!["Name", "Age"]);
        assert_eq!(tables[0].rows.len(), 2);
        assert_eq!(tables[0].rows[0], vec!["Alice", "30"]);
    }

    #[test]
    fn extract_multiple_tables() {
        let md = "\
# Section 1

| A | B |
|---|---|
| 1 | 2 |

Some text

| X | Y | Z |
|---|---|---|
| a | b | c |
";
        let tables = extract_tables(md);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].headers.len(), 2);
        assert_eq!(tables[1].headers.len(), 3);
    }

    #[test]
    fn extract_sections_basic() {
        let md = "\
# Title

Intro text

## Section A

Content A

## Section B

Content B
";
        let sections = extract_sections(md);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].heading, "Title");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[1].heading, "Section A");
        assert_eq!(sections[1].level, 2);
        assert!(sections[1].content.contains("Content A"));
    }

    #[test]
    fn heading_level_detection() {
        assert_eq!(heading_level("# Title"), Some(1));
        assert_eq!(heading_level("## Sub"), Some(2));
        assert_eq!(heading_level("### Third"), Some(3));
        assert_eq!(heading_level("Not a heading"), None);
        assert_eq!(heading_level("#NoSpace"), None);
    }

    #[test]
    fn separator_row_detection() {
        assert!(is_separator_row("| --- | --- |"));
        assert!(is_separator_row("|---|---|"));
        assert!(is_separator_row("| :---: | ---: |"));
        assert!(!is_separator_row("| Name | Age |"));
        assert!(!is_separator_row("just text"));
    }

    #[test]
    fn nested_sections_all_returned() {
        let md = "\
## Parent

### Child

Child content

## Sibling

Sibling content
";
        let sections = extract_sections(md);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].heading, "Parent");
        assert_eq!(sections[1].heading, "Child");
        assert!(sections[1].content.contains("Child content"));
        assert_eq!(sections[2].heading, "Sibling");
        assert!(sections[2].content.contains("Sibling content"));
    }
}
