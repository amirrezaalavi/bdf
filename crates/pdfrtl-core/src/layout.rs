//! Spatial layout models and block segmentation for agents.
//!
//! Hierarchical structure:
//! `PageLayout` -> `TextBlock` -> `TextLine` -> `TextSpan`.
//!
//! Every element carries a bounding box `[x0, y0, x1, y1]` in standard PDF coordinates.

use serde::{Deserialize, Serialize};

/// A positioned text span (word or contiguous glyph run).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    pub text: String,
    pub bbox: [f64; 4],
}

/// A line of text composed of positioned spans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLine {
    pub text: String,
    pub bbox: [f64; 4],
    pub spans: Vec<TextSpan>,
}

/// A block of paragraphs or column text composed of lines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub bbox: [f64; 4],
    pub lines: Vec<TextLine>,
}

/// The spatial layout for an entire page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageLayout {
    pub page: u32,
    pub bbox: [f64; 4],
    pub blocks: Vec<TextBlock>,
}

impl PageLayout {
    /// Compute the overall bounding box from a slice of blocks.
    pub fn compute_bbox(blocks: &[TextBlock]) -> [f64; 4] {
        if blocks.is_empty() {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for block in blocks {
            x0 = x0.min(block.bbox[0]);
            y0 = y0.min(block.bbox[1]);
            x1 = x1.max(block.bbox[2]);
            y1 = y1.max(block.bbox[3]);
        }
        if x0.is_infinite() {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            [x0, y0, x1, y1]
        }
    }
}

/// Computes the bounding box of a slice of text lines.
pub fn compute_lines_bbox(lines: &[TextLine]) -> [f64; 4] {
    if lines.is_empty() {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let mut x0 = f64::INFINITY;
    let mut y0 = f64::INFINITY;
    let mut x1 = f64::NEG_INFINITY;
    let mut y1 = f64::NEG_INFINITY;
    for line in lines {
        x0 = x0.min(line.bbox[0]);
        y0 = y0.min(line.bbox[1]);
        x1 = x1.max(line.bbox[2]);
        y1 = y1.max(line.bbox[3]);
    }
    if x0.is_infinite() {
        [0.0, 0.0, 0.0, 0.0]
    } else {
        [x0, y0, x1, y1]
    }
}

/// Segments lines into cohesive reading blocks based on baseline vertical spacing.
pub fn group_lines_into_blocks(lines: Vec<TextLine>) -> Vec<TextBlock> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut blocks: Vec<TextBlock> = Vec::new();
    let mut current_lines: Vec<TextLine> = Vec::new();

    for line in lines {
        if line.text.trim().is_empty() {
            continue;
        }
        if let Some(prev) = current_lines.last() {
            let prev_y = prev.bbox[1];
            let curr_y = line.bbox[1];
            let dy = (prev_y - curr_y).abs();
            let avg_height = (prev.bbox[3] - prev.bbox[1]).max(line.bbox[3] - line.bbox[1]);
            let threshold = if avg_height > 0.0 {
                avg_height * 2.5
            } else {
                24.0
            };
            if dy > threshold {
                let block_bbox = compute_lines_bbox(&current_lines);
                blocks.push(TextBlock {
                    bbox: block_bbox,
                    lines: std::mem::take(&mut current_lines),
                });
            }
        }
        current_lines.push(line);
    }

    if !current_lines.is_empty() {
        let block_bbox = compute_lines_bbox(&current_lines);
        blocks.push(TextBlock {
            bbox: block_bbox,
            lines: current_lines,
        });
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_grouping_clusters_adjacent_lines() {
        let line1 = TextLine {
            text: "Line 1".to_string(),
            bbox: [50.0, 700.0, 150.0, 712.0],
            spans: vec![TextSpan {
                text: "Line 1".to_string(),
                bbox: [50.0, 700.0, 150.0, 712.0],
            }],
        };
        let line2 = TextLine {
            text: "Line 2".to_string(),
            bbox: [50.0, 686.0, 150.0, 698.0],
            spans: vec![TextSpan {
                text: "Line 2".to_string(),
                bbox: [50.0, 686.0, 150.0, 698.0],
            }],
        };
        // Far away line: paragraph gap
        let line3 = TextLine {
            text: "Line 3".to_string(),
            bbox: [50.0, 600.0, 150.0, 612.0],
            spans: vec![TextSpan {
                text: "Line 3".to_string(),
                bbox: [50.0, 600.0, 150.0, 612.0],
            }],
        };

        let blocks = group_lines_into_blocks(vec![line1, line2, line3]);
        assert_eq!(
            blocks.len(),
            2,
            "Expected 2 blocks separated by vertical gap"
        );
        assert_eq!(blocks[0].lines.len(), 2);
        assert_eq!(blocks[1].lines.len(), 1);
        assert_eq!(blocks[0].bbox, [50.0, 686.0, 150.0, 712.0]);
    }
}
