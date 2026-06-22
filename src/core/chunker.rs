use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chunk {
    pub index: usize,
    pub content: String,
    pub start_char: usize,
    pub token_count: usize,
    pub header_breadcrumb: String,
}

pub fn chunk_text(
    content: &str,
    chunk_size: usize,
    overlap: usize,
    min_tokens: usize,
) -> Vec<Chunk> {
    if content.trim().is_empty() {
        return vec![];
    }

    let paragraphs: Vec<&str> = content
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    let mut chunks: Vec<Chunk> = vec![];
    let mut current_blocks: Vec<&str> = vec![];
    let mut current_tokens = 0usize;
    let mut current_start = 0usize;
    let mut char_pos = 0usize;
    let mut header_stack: Vec<(usize, String)> = vec![];

    for para in paragraphs {
        let para_tokens = estimate_tokens(para);
        if para.starts_with('#') {
            let level = para.chars().take_while(|&c| c == '#').count();
            let heading = para.trim_start_matches('#').trim().to_string();
            header_stack.retain(|&(l, _)| l < level);
            header_stack.push((level, heading));
        }

        if current_tokens + para_tokens > chunk_size && !current_blocks.is_empty() {
            let text = current_blocks.join("\n\n");
            let token_ct = estimate_tokens(&text);
            if token_ct >= min_tokens {
                chunks.push(Chunk {
                    index: chunks.len(),
                    content: text,
                    start_char: current_start,
                    token_count: token_ct,
                    header_breadcrumb: header_stack
                        .iter()
                        .map(|(_, t)| t.as_str())
                        .collect::<Vec<_>>()
                        .join(" > "),
                });
            }
            let (overlap_blocks, overlap_tokens) = get_overlap(&current_blocks, overlap);
            current_blocks = overlap_blocks;
            current_tokens = overlap_tokens;
            current_start = char_pos;
        }

        current_blocks.push(para);
        current_tokens += para_tokens;
        char_pos += para.len() + 2;
    }

    if !current_blocks.is_empty() {
        let text = current_blocks.join("\n\n");
        let token_ct = estimate_tokens(&text);
        if token_ct >= min_tokens {
            chunks.push(Chunk {
                index: chunks.len(),
                content: text,
                start_char: current_start,
                token_count: token_ct,
                header_breadcrumb: header_stack
                    .iter()
                    .map(|(_, t)| t.as_str())
                    .collect::<Vec<_>>()
                    .join(" > "),
            });
        }
    }

    chunks
}

fn estimate_tokens(text: &str) -> usize {
    std::cmp::max(1, text.len() / 4)
}

fn get_overlap<'a>(blocks: &[&'a str], target_tokens: usize) -> (Vec<&'a str>, usize) {
    let mut result = vec![];
    let mut tokens = 0;
    for block in blocks.iter().rev() {
        let bt = estimate_tokens(block);
        if tokens + bt > target_tokens {
            break;
        }
        result.insert(0, *block);
        tokens += bt;
    }
    (result, tokens)
}
