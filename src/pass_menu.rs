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
            Pass::Stop => "stop",
        }
    }

    /// Build correctly-nested `opt -passes=` string for LLVM 20's new pass manager.
    ///
    /// Rules:
    /// - `inline` must be wrapped as `cgscc(inline)`
    /// - `licm` must be wrapped as `loop-mssa(licm)` inside the function pipeline
    /// - When cgscc passes are present, function passes need `function(...)` wrapping
    /// - Other function passes go directly
    pub fn to_opt_pipeline(passes: &[Pass]) -> String {
        let transforms: Vec<&Pass> = passes
            .iter()
            .filter(|p| **p != Pass::Stop)
            .collect();

        if transforms.is_empty() {
            return String::new();
        }

        let has_inline = transforms.iter().any(|p| **p == Pass::Inline);

        // Build function-level pass list
        let func_passes: Vec<String> = transforms
            .iter()
            .filter(|p| ***p != Pass::Inline)
            .map(|p| {
                match **p {
                    Pass::Licm => "loop-mssa(licm)".to_string(),
                    Pass::LoopRotate => "loop(loop-rotate)".to_string(),
                    Pass::Instcombine => "instcombine<no-verify-fixpoint>".to_string(),
                    _ => p.opt_name().to_string(),
                }
            })
            .collect();

        let mut pipeline_parts: Vec<String> = Vec::new();

        if has_inline {
            pipeline_parts.push("cgscc(inline)".to_string());
        }

        if !func_passes.is_empty() {
            if has_inline {
                // When cgscc passes are present, function passes need explicit wrapping
                pipeline_parts.push(format!("function({})", func_passes.join(",")));
            } else {
                // No cgscc passes, function passes can go at top level
                pipeline_parts.push(func_passes.join(","));
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
            15 => Pass::Stop,
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
            Pass::Stop => 15,
        }
    }

    pub fn count() -> usize {
        16
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
}
