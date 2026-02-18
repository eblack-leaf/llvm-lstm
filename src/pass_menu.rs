use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pass {
    Instcombine,
    Inline,
    LoopUnroll,
    Licm,
    Gvn,
    Sroa,
    Mem2reg,
    Simplifycfg,
    Dse,
    Reassociate,
    JumpThreading,
    LoopRotate,
    Adce,
    EarlyCse,
    Tailcallelim,
    Sccp,
    Bdce,
    Memcpyopt,
    LoopDeletion,
    Argpromotion,
    GlobalOpt,
    IndVars,
    Stop,
}

impl Pass {
    pub fn opt_name(&self) -> &str {
        match self {
            Pass::Instcombine => "instcombine",
            Pass::Inline => "inline",
            Pass::LoopUnroll => "loop-unroll",
            Pass::Licm => "licm",
            Pass::Gvn => "gvn",
            Pass::Sroa => "sroa",
            Pass::Mem2reg => "mem2reg",
            Pass::Simplifycfg => "simplifycfg",
            Pass::Dse => "dse",
            Pass::Reassociate => "reassociate",
            Pass::JumpThreading => "jump-threading",
            Pass::LoopRotate => "loop-rotate",
            Pass::Adce => "adce",
            Pass::EarlyCse => "early-cse",
            Pass::Tailcallelim => "tailcallelim",
            Pass::Sccp => "sccp",
            Pass::Bdce => "bdce",
            Pass::Memcpyopt => "memcpyopt",
            Pass::LoopDeletion => "loop-deletion",
            Pass::Argpromotion => "argpromotion",
            Pass::GlobalOpt => "globalopt",
            Pass::IndVars   => "indvars",
            Pass::Stop => "stop",
        }
    }

    /// Build correctly-nested `opt -passes=` string for LLVM 20's new pass manager.
    ///
    /// Rules:
    /// - `inline` and `argpromotion` must be wrapped as `cgscc(...)` (CGSCC-level passes)
    /// - `licm` must be wrapped as `loop-mssa(licm)` inside the function pipeline
    /// - `loop-rotate` and `loop-deletion` are LoopPasses → wrap as `loop(...)`
    /// - When cgscc passes are present, function passes need `function(...)` wrapping
    /// - Other function passes go directly
    pub fn to_opt_pipeline(passes: &[Pass]) -> String {
        let transforms: Vec<&Pass> = passes.iter().filter(|p| **p != Pass::Stop).collect();
        if transforms.is_empty() {
            return String::new();
        }

        // Classify passes by their required pass manager level
        let is_module = |p: &Pass| matches!(p, Pass::GlobalOpt);
        let is_cgscc = |p: &Pass| matches!(p, Pass::Inline | Pass::Argpromotion);
        let is_loop  = |p: &Pass| matches!(p, Pass::LoopRotate | Pass::LoopDeletion | Pass::IndVars | Pass::Licm);

        // Collect passes for each level, preserving order
        let module_passes: Vec<String> = transforms
            .iter()
            .filter(|p| is_module(p))
            .map(|p| p.opt_name().to_string())
            .collect();

        let cgscc_passes: Vec<String> = transforms
            .iter()
            .filter(|p| is_cgscc(p))
            .map(|p| p.opt_name().to_string())
            .collect();

        // For function‑level passes (everything else except loop passes that will be nested)
        let function_passes: Vec<String> = transforms
            .iter()
            .filter(|p| !is_module(p) && !is_cgscc(p))
            .map(|p| {
                if is_loop(p) {
                    // Wrap loop passes appropriately
                    match **p {
                        Pass::Licm => "loop-mssa(licm)".to_string(),
                        Pass::LoopRotate => "loop(loop-rotate)".to_string(),
                        Pass::LoopDeletion => "loop(loop-deletion)".to_string(),
                        Pass::IndVars => "loop(indvars)".to_string(),
                        _ => unreachable!(),
                    }
                } else {
                    // Ordinary function passes
                    if **p == Pass::Instcombine {
                        "instcombine<no-verify-fixpoint>".to_string()
                    } else {
                        p.opt_name().to_string()
                    }
                }
            })
            .collect();

        let mut pipeline_parts = Vec::new();

        if !module_passes.is_empty() {
            pipeline_parts.push(format!("module({})", module_passes.join(",")));
        }

        if !cgscc_passes.is_empty() {
            pipeline_parts.push(format!("cgscc({})", cgscc_passes.join(",")));
        }

        if !function_passes.is_empty() {
            if !cgscc_passes.is_empty() || !module_passes.is_empty() {
                // If we have module or CGSCC passes, function passes need explicit wrapping
                pipeline_parts.push(format!("function({})", function_passes.join(",")));
            } else {
                // No higher‑level passes: function passes can be top‑level
                pipeline_parts.push(function_passes.join(","));
            }
        }

        pipeline_parts.join(",")
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Pass::Instcombine,
            1 => Pass::Inline,
            2 => Pass::LoopUnroll,
            3 => Pass::Licm,
            4 => Pass::Gvn,
            5 => Pass::Sroa,
            6 => Pass::Mem2reg,
            7 => Pass::Simplifycfg,
            8 => Pass::Dse,
            9 => Pass::Reassociate,
            10 => Pass::JumpThreading,
            11 => Pass::LoopRotate,
            12 => Pass::Adce,
            13 => Pass::EarlyCse,
            14 => Pass::Tailcallelim,
            15 => Pass::Sccp,
            16 => Pass::Bdce,
            17 => Pass::Memcpyopt,
            18 => Pass::LoopDeletion,
            19 => Pass::Argpromotion,
            20 => Pass::GlobalOpt,
            21 => Pass::IndVars,
            22 => Pass::Stop,
            _ => panic!("Invalid pass index: {i}"),
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            Pass::Instcombine => 0,
            Pass::Inline => 1,
            Pass::LoopUnroll => 2,
            Pass::Licm => 3,
            Pass::Gvn => 4,
            Pass::Sroa => 5,
            Pass::Mem2reg => 6,
            Pass::Simplifycfg => 7,
            Pass::Dse => 8,
            Pass::Reassociate => 9,
            Pass::JumpThreading => 10,
            Pass::LoopRotate => 11,
            Pass::Adce => 12,
            Pass::EarlyCse => 13,
            Pass::Tailcallelim => 14,
            Pass::Sccp => 15,
            Pass::Bdce => 16,
            Pass::Memcpyopt => 17,
            Pass::LoopDeletion => 18,
            Pass::Argpromotion => 19,
            Pass::GlobalOpt => 20,
            Pass::IndVars   => 21,
            Pass::Stop => 22,
        }
    }

    pub fn count() -> usize {
        23
    }

    pub fn all_transforms() -> &'static [Pass] {
        &[
            Pass::Instcombine,
            Pass::Inline,
            Pass::LoopUnroll,
            Pass::Licm,
            Pass::Gvn,
            Pass::Sroa,
            Pass::Mem2reg,
            Pass::Simplifycfg,
            Pass::Dse,
            Pass::Reassociate,
            Pass::JumpThreading,
            Pass::LoopRotate,
            Pass::Adce,
            Pass::EarlyCse,
            Pass::Tailcallelim,
            Pass::Sccp,
            Pass::Bdce,
            Pass::Memcpyopt,
            Pass::LoopDeletion,
            Pass::Argpromotion,
            Pass::GlobalOpt,
            Pass::IndVars,
        ]
    }
}

impl fmt::Display for Pass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.opt_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_simple() {
        let passes = vec![Pass::Instcombine, Pass::Sroa, Pass::Simplifycfg];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "instcombine<no-verify-fixpoint>,sroa,simplifycfg");
    }

    #[test]
    fn test_pipeline_with_inline() {
        let passes = vec![Pass::Inline, Pass::Instcombine, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "cgscc(inline),function(instcombine<no-verify-fixpoint>,sroa)");
    }

    #[test]
    fn test_pipeline_with_licm() {
        let passes = vec![Pass::Licm, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop-mssa(licm),instcombine<no-verify-fixpoint>");
    }

    #[test]
    fn test_pipeline_with_inline_and_licm() {
        let passes = vec![Pass::Inline, Pass::Licm, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "cgscc(inline),function(loop-mssa(licm),instcombine<no-verify-fixpoint>)"
        );
    }

    #[test]
    fn test_pipeline_with_argpromotion() {
        let passes = vec![Pass::Argpromotion, Pass::Inline, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "cgscc(argpromotion,inline),function(sroa)");
    }

    #[test]
    fn test_pipeline_with_loop_deletion() {
        let passes = vec![Pass::LoopDeletion, Pass::LoopRotate];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop(loop-deletion),loop(loop-rotate)");
    }

    #[test]
    fn test_pipeline_stop_filtered() {
        let passes = vec![Pass::Instcombine, Pass::Stop];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "instcombine<no-verify-fixpoint>");
    }

    #[test]
    fn test_roundtrip_index() {
        for i in 0..Pass::count() {
            assert_eq!(Pass::from_index(i).to_index(), i);
        }
    }
    #[test]
    fn test_pipeline_with_globalopt() {
        let passes = vec![Pass::GlobalOpt, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        // module passes appear first
        assert_eq!(pipeline, "module(globalopt),instcombine<no-verify-fixpoint>");
    }

    #[test]
    fn test_pipeline_with_indvars() {
        let passes = vec![Pass::IndVars, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        // indvars is a loop pass → wrapped inside function(...)
        assert_eq!(pipeline, "loop(indvars),instcombine<no-verify-fixpoint>");
    }

    #[test]
    fn test_pipeline_mixed_levels() {
        let passes = vec![Pass::GlobalOpt, Pass::Inline, Pass::IndVars, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "module(globalopt),cgscc(inline),function(loop(indvars),instcombine<no-verify-fixpoint>)"
        );
    }
}
