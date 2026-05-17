use crate::models::BlockType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitBlock {
    pub block_type: BlockType,
    pub content: String,
    pub indent: u8,
}

pub fn split_markdown(md: &str) -> Vec<SplitBlock> {
    let mut out = Vec::new();
    let mut paragraph = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut code_indent = 0u8;

    fn indent_of(line: &str) -> (u8, &str) {
        let mut spaces = 0;
        let mut chars = line.chars();
        let mut consumed = 0;
        for c in chars.by_ref() {
            match c {
                ' ' => {
                    spaces += 1;
                    consumed += 1;
                }
                '\t' => {
                    spaces += 2;
                    consumed += 1;
                }
                _ => break,
            }
        }
        ((spaces / 2) as u8, &line[consumed..])
    }

    let flush_para = |out: &mut Vec<SplitBlock>, para: &mut String, indent: u8| {
        let trimmed = para.trim_end();
        if !trimmed.is_empty() {
            out.push(SplitBlock {
                block_type: BlockType::Text,
                content: trimmed.to_string(),
                indent,
            });
        }
        para.clear();
    };

    for raw in md.lines() {
        if in_code {
            if raw.trim_start().starts_with("```") {
                out.push(SplitBlock {
                    block_type: BlockType::Code,
                    content: code_buf.trim_end().to_string(),
                    indent: code_indent,
                });
                code_buf.clear();
                in_code = false;
            } else {
                code_buf.push_str(raw);
                code_buf.push('\n');
            }
            continue;
        }

        let (indent, rest) = indent_of(raw);

        if rest.starts_with("```") {
            flush_para(&mut out, &mut paragraph, indent);
            in_code = true;
            code_indent = indent;
            continue;
        }
        if rest.trim().is_empty() {
            flush_para(&mut out, &mut paragraph, indent);
            continue;
        }
        if rest.starts_with("---") && rest.trim() == "---" {
            flush_para(&mut out, &mut paragraph, indent);
            out.push(SplitBlock {
                block_type: BlockType::Divider,
                content: String::new(),
                indent,
            });
            continue;
        }
        if let Some(hashes) = rest.strip_prefix('#') {
            let mut level = 1;
            let mut tail = hashes;
            while let Some(rest2) = tail.strip_prefix('#') {
                level += 1;
                tail = rest2;
                if level >= 6 {
                    break;
                }
            }
            if let Some(text) = tail.strip_prefix(' ') {
                flush_para(&mut out, &mut paragraph, indent);
                out.push(SplitBlock {
                    block_type: BlockType::Heading,
                    content: format!("{} {}", "#".repeat(level), text),
                    indent,
                });
                continue;
            }
        }
        if rest.starts_with("- [ ] ") || rest.starts_with("- [x] ") || rest.starts_with("- [X] ") {
            flush_para(&mut out, &mut paragraph, indent);
            out.push(SplitBlock {
                block_type: BlockType::Todo,
                content: rest.to_string(),
                indent,
            });
            continue;
        }
        if let Some(quote_body) = rest.strip_prefix("> ") {
            flush_para(&mut out, &mut paragraph, indent);
            out.push(SplitBlock {
                block_type: BlockType::Quote,
                content: quote_body.to_string(),
                indent,
            });
            continue;
        }

        if !paragraph.is_empty() {
            paragraph.push('\n');
        }
        paragraph.push_str(rest);
    }
    flush_para(&mut out, &mut paragraph, 0);
    if in_code && !code_buf.is_empty() {
        out.push(SplitBlock {
            block_type: BlockType::Code,
            content: code_buf.trim_end().to_string(),
            indent: code_indent,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_becomes_text_block() {
        let out = split_markdown("hello world");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].block_type, BlockType::Text);
        assert_eq!(out[0].content, "hello world");
    }

    #[test]
    fn blank_line_separates_paragraphs() {
        let out = split_markdown("a\n\nb");
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].content, "b");
    }

    #[test]
    fn heading_is_detected() {
        let out = split_markdown("# Title\n\nbody");
        assert_eq!(out[0].block_type, BlockType::Heading);
        assert_eq!(out[1].block_type, BlockType::Text);
    }

    #[test]
    fn todo_is_detected() {
        let out = split_markdown("- [ ] buy milk\n- [x] done");
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|b| b.block_type == BlockType::Todo));
    }

    #[test]
    fn fenced_code_becomes_single_block() {
        let md = "```rust\nfn main() {}\n```";
        let out = split_markdown(md);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].block_type, BlockType::Code);
        assert_eq!(out[0].content, "fn main() {}");
    }

    #[test]
    fn divider_is_detected() {
        let out = split_markdown("a\n\n---\n\nb");
        assert_eq!(
            out.iter()
                .filter(|b| b.block_type == BlockType::Divider)
                .count(),
            1
        );
    }

    #[test]
    fn indent_increments_with_two_spaces() {
        let out = split_markdown("- [ ] outer\n  - [ ] inner");
        assert_eq!(out[0].indent, 0);
        assert_eq!(out[1].indent, 1);
    }
}
