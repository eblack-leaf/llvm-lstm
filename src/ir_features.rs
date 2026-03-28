use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// IR features extracted by text-parsing an LLVM `.ll` file.
/// Designed to run in <50ms on typical benchmark IR.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrFeatures {
    // Instruction counts by category
    pub add_count: u32,
    pub mul_count: u32,
    pub load_count: u32,
    pub store_count: u32,
    pub br_count: u32,
    pub call_count: u32,
    pub phi_count: u32,
    pub alloca_count: u32,
    pub gep_count: u32,
    pub icmp_count: u32,
    pub fcmp_count: u32,
    pub ret_count: u32,
    pub select_count: u32,   // if-conversion / conditional-move opportunities
    pub bitwise_count: u32,  // and, or, xor, shl, lshr, ashr — instcombine targets
    pub cast_count: u32,     // zext, sext, trunc, bitcast, fp* conversions
    pub other_inst_count: u32,
    // Structural features
    pub basic_block_count: u32,
    pub total_instruction_count: u32,
    pub function_count: u32,
    pub loop_depth_approx: u32,
    // Derived ratios — scale-invariant signals
    pub load_store_ratio: f32,  // load / store (memory read vs write balance)
    pub mem_ratio: f32,         // (load+store) / total_inst (memory pressure)
    pub call_ratio: f32,        // call / total_inst (inlining opportunity signal)
    pub avg_bb_size: f32,       // total_inst / bb_count (block granularity)
}

impl IrFeatures {
    pub fn from_ll_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read IR file: {}", path.display()))?;
        Self::from_ll_str(&content)
    }

    pub fn from_ll_str(content: &str) -> Result<Self> {
        let mut f = IrFeatures::default();

        // Track labels we've seen to detect back-edges
        let mut _current_label: Option<String> = None;
        let mut label_order: Vec<String> = Vec::new();
        let mut in_function = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip comments and metadata-only lines
            if trimmed.starts_with(';') || trimmed.is_empty() {
                continue;
            }

            // Function definition — also count implicit entry block
            if trimmed.starts_with("define ") {
                f.function_count += 1;
                f.basic_block_count += 1; // implicit entry block
                in_function = true;
                label_order.clear();
                label_order.push("entry".to_string());
                _current_label = None;
                continue;
            }

            // End of function
            if trimmed == "}" {
                in_function = false;
                continue;
            }

            if !in_function {
                continue;
            }

            // Basic block label detection
            // LLVM IR labels: "entry:", "7:", "for.body:" — possibly followed by comment
            // The label is always at the start of the (non-whitespace) line
            if let Some(colon_pos) = trimmed.find(':') {
                let before = &trimmed[..colon_pos];
                // Labels: no spaces before colon, no special prefix chars
                if !before.is_empty()
                    && !before.contains(' ')
                    && !before.starts_with('%')
                    && !before.starts_with('@')
                    && !before.starts_with('!')
                    && !before.contains('(')
                    && !trimmed.contains(" = ")
                {
                    f.basic_block_count += 1;
                    let label = before.to_string();
                    label_order.push(label.clone());
                    _current_label = Some(label);
                    continue;
                }
            }

            // Parse instruction opcode
            // Typical forms:
            //   %var = opcode ...
            //   opcode ...  (for terminators like br, ret, store)
            let opcode = extract_opcode(trimmed);
            if let Some(op) = opcode {
                f.total_instruction_count += 1;
                match op {
                    "add" | "fadd" | "sub" | "fsub" => f.add_count += 1,
                    "mul" | "fmul" | "udiv" | "sdiv" | "fdiv" | "urem" | "srem" | "frem" => {
                        f.mul_count += 1
                    }
                    "load" => f.load_count += 1,
                    "store" => f.store_count += 1,
                    "br" | "switch" | "indirectbr" => {
                        f.br_count += 1;
                        // Detect back-edges for loop approximation
                        if op == "br" {
                            detect_back_edge(trimmed, &label_order, &mut f.loop_depth_approx);
                        }
                    }
                    "call" | "invoke" => f.call_count += 1,
                    "phi" => f.phi_count += 1,
                    "alloca" => f.alloca_count += 1,
                    "getelementptr" => f.gep_count += 1,
                    "icmp" => f.icmp_count += 1,
                    "fcmp" => f.fcmp_count += 1,
                    "ret" => f.ret_count += 1,
                    "select" => f.select_count += 1,
                    "and" | "or" | "xor" | "shl" | "lshr" | "ashr" => f.bitwise_count += 1,
                    "zext" | "sext" | "trunc" | "bitcast" | "fpext" | "fptrunc"
                    | "fptosi" | "fptoui" | "sitofp" | "uitofp"
                    | "ptrtoint" | "inttoptr" | "addrspacecast" => f.cast_count += 1,
                    _ => f.other_inst_count += 1,
                }
            }
        }

        // Compute derived ratios
        f.load_store_ratio = if f.store_count > 0 {
            f.load_count as f32 / f.store_count as f32
        } else if f.load_count > 0 {
            f.load_count as f32
        } else {
            0.0
        };
        let total = f.total_instruction_count as f32;
        f.mem_ratio  = if total > 0.0 { (f.load_count + f.store_count) as f32 / total } else { 0.0 };
        f.call_ratio = if total > 0.0 { f.call_count as f32 / total } else { 0.0 };
        f.avg_bb_size = if f.basic_block_count > 0 {
            total / f.basic_block_count as f32
        } else {
            0.0
        };

        Ok(f)
    }

    /// Convert to fixed-size feature vector for model input.
    ///
    /// Raw counts are log-transformed (ln(1+x)) so large and small functions
    /// land on a comparable scale. Derived ratios are already in [0, ∞) and
    /// left as-is (they're bounded or near-bounded in practice).
    pub fn to_vec(&self) -> Vec<f32> {
        let ln = |x: u32| (1.0 + x as f32).ln();
        vec![
            // Log-transformed instruction counts
            ln(self.add_count),
            ln(self.mul_count),
            ln(self.load_count),
            ln(self.store_count),
            ln(self.br_count),
            ln(self.call_count),
            ln(self.phi_count),
            ln(self.alloca_count),
            ln(self.gep_count),
            ln(self.icmp_count),
            ln(self.fcmp_count),
            ln(self.ret_count),
            ln(self.select_count),
            ln(self.bitwise_count),
            ln(self.cast_count),
            ln(self.other_inst_count),
            // Log-transformed structural counts
            ln(self.basic_block_count),
            ln(self.total_instruction_count),
            ln(self.function_count),
            ln(self.loop_depth_approx),
            // Derived ratios (scale-invariant)
            self.load_store_ratio,
            self.mem_ratio,
            self.call_ratio,
            self.avg_bb_size,
        ]
    }

    /// Number of features in the vector representation.
    pub fn feature_count() -> usize {
        24
    }
}

/// Extract the opcode from an LLVM IR instruction line.
fn extract_opcode(line: &str) -> Option<&str> {
    let trimmed = line.trim();

    // Skip metadata, comments, attributes
    if trimmed.starts_with('!')
        || trimmed.starts_with(';')
        || trimmed.starts_with("attributes")
        || trimmed.starts_with("declare")
        || trimmed.starts_with("source_filename")
        || trimmed.starts_with("target")
        || trimmed.starts_with("@")
    {
        return None;
    }

    // Form: %var = opcode ...
    if let Some(eq_pos) = trimmed.find(" = ") {
        let after_eq = trimmed[eq_pos + 3..].trim();
        // The opcode is the first word (possibly after optional flags like 'nsw', 'nuw')
        let first_word = after_eq.split_whitespace().next()?;
        // Handle 'tail call', 'musttail call' etc.
        if first_word == "tail" || first_word == "musttail" || first_word == "notail" {
            return Some("call");
        }
        return Some(first_word);
    }

    // Terminator or void instruction: opcode ...
    let first_word = trimmed.split_whitespace().next()?;
    match first_word {
        "br" | "ret" | "switch" | "indirectbr" | "unreachable" | "store" | "call" | "invoke"
        | "resume" | "fence" | "tail" | "musttail" | "notail" => {
            if first_word == "tail" || first_word == "musttail" || first_word == "notail" {
                Some("call")
            } else {
                Some(first_word)
            }
        }
        _ => None,
    }
}

/// Check if a br instruction branches back to an earlier label (indicating a loop).
fn detect_back_edge(br_line: &str, label_order: &[String], loop_count: &mut u32) {
    // Extract branch targets from: br i1 %cond, label %target1, label %target2
    // or: br label %target
    for part in br_line.split("label %") {
        if part == br_line {
            // No "label %" found
            continue;
        }
        // Extract label name (ends at comma, space, or newline)
        let target: String = part
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();

        if !target.is_empty() {
            // Check if target appears before current position in label_order
            // (excluding the last entry which is current block)
            let current_idx = label_order.len().saturating_sub(1);
            for (i, label) in label_order.iter().enumerate() {
                if i < current_idx && *label == target {
                    *loop_count += 1;
                    return; // Count each br as at most one back-edge
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_ir() {
        let ir = r#"
define i32 @main() {
entry:
  %x = alloca i32
  store i32 42, ptr %x
  %v = load i32, ptr %x
  %r = add i32 %v, 1
  ret i32 %r
}
"#;
        let features = IrFeatures::from_ll_str(ir).unwrap();
        assert_eq!(features.function_count, 1);
        assert_eq!(features.alloca_count, 1);
        assert_eq!(features.store_count, 1);
        assert_eq!(features.load_count, 1);
        assert_eq!(features.add_count, 1);
        assert_eq!(features.ret_count, 1);
        assert_eq!(features.total_instruction_count, 5);
    }

    #[test]
    fn test_feature_vec_length() {
        let features = IrFeatures::default();
        assert_eq!(features.to_vec().len(), IrFeatures::feature_count());
    }
}
