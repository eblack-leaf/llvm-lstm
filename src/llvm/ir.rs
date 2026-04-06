use anyhow::Result;
use std::path::PathBuf;
#[derive(Clone)]
pub(crate) struct Source {
    pub(crate) file: PathBuf,
}
pub(crate) struct Bin {
    pub(crate) file: PathBuf,
}
#[derive(Clone)]
pub(crate) struct Ir {
    pub(crate) file: PathBuf,
}
#[derive(Default)]
pub(crate) struct Features {
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
    pub select_count: u32,  // if-conversion / conditional-move opportunities
    pub bitwise_count: u32, // and, or, xor, shl, lshr, ashr — instcombine targets
    pub cast_count: u32,    // zext, sext, trunc, bitcast, fp* conversions
    pub other_inst_count: u32,
    // Structural features
    pub basic_block_count: u32,
    pub total_instruction_count: u32,
    pub function_count: u32,
    pub loop_depth_approx: u32,
    // Derived ratios — scale-invariant signals
    pub load_store_ratio: f32, // load / store (memory read vs write balance)
    pub mem_ratio: f32,        // (load+store) / total_inst (memory pressure)
    pub call_ratio: f32,       // call / total_inst (inlining opportunity signal)
    pub avg_bb_size: f32,      // total_inst / bb_count (block granularity)
    // Pass-opportunity indicators — metadata and structural signals
    pub unreachable_count: u32,   // dead terminators → ADCE / simplifycfg
    pub invoke_count: u32,        // exception-handling calls → inlining cost signal
    pub switch_count: u32,        // switch stmts → jump-threading / lowering
    pub intrinsic_count: u32,     // llvm.* calls (memcpy/memset/…) → loop-idiom / memcpyopt
    pub tbaa_count: u32,          // !tbaa refs → alias-analysis richness (LICM/DSE quality)
    pub loop_metadata_count: u32, // llvm.loop refs → loop hints present (vectorise/unroll)
    pub noalias_count: u32,       // noalias attrs → pointer-aliasing provable (LICM/DSE)
    pub phi_ratio: f32,           // phi / bb → SSA density (GVN/instcombine readiness)
    // Loop structure
    pub cond_br_count: u32, // conditional branches (br i1 ...) — control flow density
    pub max_loop_nest_approx: u32, // approx max loop nesting depth from back-edge positions
    // Opt-metadata enrichment
    pub vector_inst_count: u32, // <N x type> typed values — vectorization readiness
    pub entry_alloca_count: u32, // allocas in entry block only — direct mem2reg opportunity
    pub tail_call_count: u32,   // tail/musttail calls — tailcallelim signal
    pub nsw_nuw_count: u32,     // instructions carrying nsw/nuw flags — instcombine strength
    pub prof_metadata_count: u32, // !prof branch weights present — jump-threading/licm quality
    pub freeze_count: u32,      // freeze instructions — post-SROA/instcombine marker
}
impl Features {
    pub fn from_ll_str(content: &str) -> Result<Self> {
        let mut f = Features::default();

        // Track labels we've seen to detect back-edges and loop nesting
        let mut _current_label: Option<String> = None;
        let mut label_order: Vec<String> = Vec::new();
        let mut loop_header_positions: Vec<usize> = Vec::new(); // positions of loop header labels
        let mut in_function = false;
        let mut in_entry_block = false; // true until first non-entry label

        // Whole-file metadata counts — scan before the per-line loop.
        for line in content.lines() {
            let t = line.trim();
            if t.contains("!tbaa") {
                f.tbaa_count += 1;
            }
            if t.contains("llvm.loop") {
                f.loop_metadata_count += 1;
            }
            if t.contains("noalias") {
                f.noalias_count += 1;
            }
            if t.contains("!prof") {
                f.prof_metadata_count += 1;
            }
            // Vector type: <N x type> — count occurrences across all instructions/metadata
            let mut rest = t;
            while let Some(pos) = rest.find('<') {
                rest = &rest[pos + 1..];
                // Expect a digit immediately after '<' to distinguish from other uses
                if rest.starts_with(|c: char| c.is_ascii_digit()) {
                    f.vector_inst_count += 1;
                }
            }
        }

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
                in_entry_block = true;
                label_order.clear();
                loop_header_positions.clear();
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
                    in_entry_block = false; // any named block after entry exits entry block
                    let label = before.to_string();
                    label_order.push(label.clone());
                    _current_label = Some(label);
                    // Nesting depth at this block = number of loop headers at earlier positions
                    let current_pos = label_order.len() - 1;
                    let depth = loop_header_positions
                        .iter()
                        .filter(|&&p| p < current_pos)
                        .count();
                    f.max_loop_nest_approx = f.max_loop_nest_approx.max(depth as u32);
                    continue;
                }
            }

            // Parse instruction opcode
            // Typical forms:
            //   %var = opcode ...
            //   opcode ...  (for terminators like br, ret, store)
            // nsw / nuw flags on any instruction
            if trimmed.contains(" nsw") || trimmed.contains(" nuw") {
                f.nsw_nuw_count += 1;
            }

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
                        if op == "br" {
                            // Conditional branch: "br i1 ..."
                            if trimmed.contains("i1 ") {
                                f.cond_br_count += 1;
                            }
                            detect_back_edge(
                                trimmed,
                                &label_order,
                                &mut f.loop_depth_approx,
                                &mut loop_header_positions,
                            );
                        }
                        if op == "switch" {
                            f.switch_count += 1;
                        }
                    }
                    "unreachable" => {
                        f.total_instruction_count += 1;
                        f.unreachable_count += 1;
                    }
                    "call" | "invoke" => {
                        f.call_count += 1;
                        if op == "invoke" {
                            f.invoke_count += 1;
                        }
                        // Intrinsics: call target begins with @llvm.
                        if trimmed.contains("@llvm.") {
                            f.intrinsic_count += 1;
                        }
                        // tail / musttail calls — tailcallelim signal
                        if trimmed.starts_with("tail ") || trimmed.starts_with("musttail ") {
                            f.tail_call_count += 1;
                        }
                    }
                    "phi" => f.phi_count += 1,
                    "alloca" => {
                        f.alloca_count += 1;
                        if in_entry_block {
                            f.entry_alloca_count += 1;
                        }
                    }
                    "freeze" => f.freeze_count += 1,
                    "getelementptr" => f.gep_count += 1,
                    "icmp" => f.icmp_count += 1,
                    "fcmp" => f.fcmp_count += 1,
                    "ret" => f.ret_count += 1,
                    "select" => f.select_count += 1,
                    "and" | "or" | "xor" | "shl" | "lshr" | "ashr" => f.bitwise_count += 1,
                    "zext" | "sext" | "trunc" | "bitcast" | "fpext" | "fptrunc" | "fptosi"
                    | "fptoui" | "sitofp" | "uitofp" | "ptrtoint" | "inttoptr"
                    | "addrspacecast" => f.cast_count += 1,
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
        f.mem_ratio = if total > 0.0 {
            (f.load_count + f.store_count) as f32 / total
        } else {
            0.0
        };
        f.call_ratio = if total > 0.0 {
            f.call_count as f32 / total
        } else {
            0.0
        };
        f.avg_bb_size = if f.basic_block_count > 0 {
            total / f.basic_block_count as f32
        } else {
            0.0
        };
        f.phi_ratio = if f.basic_block_count > 0 {
            f.phi_count as f32 / f.basic_block_count as f32
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
            // Pass-opportunity indicators
            ln(self.unreachable_count),
            ln(self.invoke_count),
            ln(self.switch_count),
            ln(self.intrinsic_count),
            ln(self.tbaa_count),
            ln(self.loop_metadata_count),
            ln(self.noalias_count),
            self.phi_ratio,
            // Loop structure
            ln(self.cond_br_count),
            ln(self.max_loop_nest_approx),
            // Opt-metadata enrichment
            ln(self.vector_inst_count),
            ln(self.entry_alloca_count),
            ln(self.tail_call_count),
            ln(self.nsw_nuw_count),
            ln(self.prof_metadata_count),
            ln(self.freeze_count),
        ]
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
/// Records the header's position in `loop_header_positions` for nesting depth estimation.
fn detect_back_edge(
    br_line: &str,
    label_order: &[String],
    loop_count: &mut u32,
    loop_header_positions: &mut Vec<usize>,
) {
    for part in br_line.split("label %") {
        if part == br_line {
            continue;
        }
        let target: String = part
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();

        if !target.is_empty() {
            let current_idx = label_order.len().saturating_sub(1);
            for (i, label) in label_order.iter().enumerate() {
                if i < current_idx && *label == target {
                    *loop_count += 1;
                    if !loop_header_positions.contains(&i) {
                        loop_header_positions.push(i);
                    }
                    return;
                }
            }
        }
    }
}
