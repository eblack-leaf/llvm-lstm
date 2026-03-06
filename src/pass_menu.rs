use std::fmt;

use serde::{Deserialize, Serialize};

/// A selectable optimization pass.
///
/// Index layout:
///   Base (always):             0-30  (31 transforms) + Stop at 31   → count = 32
///   Extended (full_o3_passes): 0-30  (31 transforms)
///                              32-76 (45 extended transforms)
///                              + Stop at 77                          → count = 78
///
/// Index 31 is unused / Stop when full_o3_passes is disabled.
/// Index 31 is a gap (invalid action) when full_o3_passes is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pass {
    // ── Base passes (indices 0–30) ────────────────────────────────────────────
    Instcombine,            // 0
    Inline,                 // 1
    LoopUnroll,             // 2
    Licm,                   // 3
    Gvn,                    // 4
    Sroa,                   // 5
    Mem2reg,                // 6
    Simplifycfg,            // 7
    Dse,                    // 8
    Reassociate,            // 9
    JumpThreading,          // 10
    LoopRotate,             // 11
    Adce,                   // 12
    EarlyCse,               // 13
    Tailcallelim,           // 14
    Sccp,                   // 15
    Bdce,                   // 16
    Memcpyopt,              // 17
    LoopDeletion,           // 18
    Argpromotion,           // 19
    GlobalOpt,              // 20
    IndVars,                // 21
    LoopVectorize,          // 22
    SlpVectorizer,          // 23
    CorrelatedPropagation,  // 24
    AggressiveInstcombine,  // 25
    ConstraintElimination,  // 26
    VectorCombine,          // 27
    Float2int,              // 28
    LoopIdiom,              // 29
    SimpleLoopUnswitch,     // 30

    // ── Extended passes matching opt -O3 (indices 32–76) ─────────────────────
    // Module-level
    #[cfg(feature = "full_o3_passes")]
    Annotation2Metadata,        // 32
    #[cfg(feature = "full_o3_passes")]
    CalledValuePropagation,     // 33
    #[cfg(feature = "full_o3_passes")]
    CgProfile,                  // 34
    #[cfg(feature = "full_o3_passes")]
    CoroAnnotationElide,        // 35
    #[cfg(feature = "full_o3_passes")]
    ConstMerge,                 // 36
    #[cfg(feature = "full_o3_passes")]
    CoroCleanup,                // 37
    #[cfg(feature = "full_o3_passes")]
    CoroEarly,                  // 38
    #[cfg(feature = "full_o3_passes")]
    Deadargelim,                // 39
    #[cfg(feature = "full_o3_passes")]
    ElimAvailExtern,            // 40
    #[cfg(feature = "full_o3_passes")]
    ForceAttrs,                 // 41
    #[cfg(feature = "full_o3_passes")]
    GlobalDce,                  // 42
    #[cfg(feature = "full_o3_passes")]
    InferAttrs,                 // 43
    #[cfg(feature = "full_o3_passes")]
    Ipsccp,                     // 44
    #[cfg(feature = "full_o3_passes")]
    RelLookupTableConverter,    // 45
    #[cfg(feature = "full_o3_passes")]
    RpoFunctionAttrs,           // 46
    // CGSCC-level
    #[cfg(feature = "full_o3_passes")]
    AlwaysInline,               // 47
    #[cfg(feature = "full_o3_passes")]
    CoroSplit,                  // 48
    #[cfg(feature = "full_o3_passes")]
    FunctionAttrs,              // 49
    #[cfg(feature = "full_o3_passes")]
    OpenMpOpt,                  // 50
    #[cfg(feature = "full_o3_passes")]
    OpenMpOptCgscc,             // 51
    // Loop-level
    #[cfg(feature = "full_o3_passes")]
    ExtraSimpleLoopUnswitchPasses, // 52
    #[cfg(feature = "full_o3_passes")]
    LoopInstsimplify,           // 53
    #[cfg(feature = "full_o3_passes")]
    LoopSimplifycfg,            // 54
    #[cfg(feature = "full_o3_passes")]
    LoopUnrollFull,             // 55
    // Function-level
    #[cfg(feature = "full_o3_passes")]
    AlignmentFromAssumptions,   // 56
    #[cfg(feature = "full_o3_passes")]
    AnnotationRemarks,          // 57
    #[cfg(feature = "full_o3_passes")]
    CallsiteSplitting,          // 58
    #[cfg(feature = "full_o3_passes")]
    Chr,                        // 59
    #[cfg(feature = "full_o3_passes")]
    CoroElide,                  // 60
    #[cfg(feature = "full_o3_passes")]
    DivRemPairs,                // 61
    #[cfg(feature = "full_o3_passes")]
    EeInstrument,               // 62
    #[cfg(feature = "full_o3_passes")]
    InferAlignment,             // 63
    #[cfg(feature = "full_o3_passes")]
    InjectTliMappings,          // 64
    #[cfg(feature = "full_o3_passes")]
    InstSimplify,               // 65
    #[cfg(feature = "full_o3_passes")]
    LibcallsShrinkwrap,         // 66
    #[cfg(feature = "full_o3_passes")]
    LoopDistribute,             // 67
    #[cfg(feature = "full_o3_passes")]
    LoopLoadElim,               // 68
    #[cfg(feature = "full_o3_passes")]
    LoopSink,                   // 69
    #[cfg(feature = "full_o3_passes")]
    LowerConstantIntrinsics,    // 70
    #[cfg(feature = "full_o3_passes")]
    LowerExpect,                // 71
    #[cfg(feature = "full_o3_passes")]
    MldstMotion,                // 72
    #[cfg(feature = "full_o3_passes")]
    MoveAutoInit,               // 73
    #[cfg(feature = "full_o3_passes")]
    SpeculativeExecution,       // 74
    #[cfg(feature = "full_o3_passes")]
    TransformWarning,           // 75
    #[cfg(feature = "full_o3_passes")]
    Verify,                     // 76
    #[cfg(feature = "full_o3_passes")]
    LoopUnrollAndJam,           // 77

    // ── Terminal action ───────────────────────────────────────────────────────
    Stop, // 31 (base) or 78 (full_o3_passes)
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
            Pass::IndVars => "indvars",
            Pass::LoopVectorize => "loop-vectorize",
            Pass::SlpVectorizer => "slp-vectorizer",
            Pass::CorrelatedPropagation => "correlated-propagation",
            Pass::AggressiveInstcombine => "aggressive-instcombine",
            Pass::ConstraintElimination => "constraint-elimination",
            Pass::VectorCombine => "vector-combine",
            Pass::Float2int => "float2int",
            Pass::LoopIdiom => "loop-idiom",
            Pass::SimpleLoopUnswitch => "simple-loop-unswitch",
            // Module
            #[cfg(feature = "full_o3_passes")]
            Pass::Annotation2Metadata => "annotation2metadata",
            #[cfg(feature = "full_o3_passes")]
            Pass::CalledValuePropagation => "called-value-propagation",
            #[cfg(feature = "full_o3_passes")]
            Pass::CgProfile => "cg-profile",
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroAnnotationElide => "coro-annotation-elide",
            #[cfg(feature = "full_o3_passes")]
            Pass::ConstMerge => "constmerge",
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroCleanup => "coro-cleanup",
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroEarly => "coro-early",
            #[cfg(feature = "full_o3_passes")]
            Pass::Deadargelim => "deadargelim",
            #[cfg(feature = "full_o3_passes")]
            Pass::ElimAvailExtern => "elim-avail-extern",
            #[cfg(feature = "full_o3_passes")]
            Pass::ForceAttrs => "forceattrs",
            #[cfg(feature = "full_o3_passes")]
            Pass::GlobalDce => "globaldce",
            #[cfg(feature = "full_o3_passes")]
            Pass::InferAttrs => "inferattrs",
            #[cfg(feature = "full_o3_passes")]
            Pass::Ipsccp => "ipsccp",
            #[cfg(feature = "full_o3_passes")]
            Pass::RelLookupTableConverter => "rel-lookup-table-converter",
            #[cfg(feature = "full_o3_passes")]
            Pass::RpoFunctionAttrs => "rpo-function-attrs",
            // CGSCC
            #[cfg(feature = "full_o3_passes")]
            Pass::AlwaysInline => "always-inline",
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroSplit => "coro-split",
            #[cfg(feature = "full_o3_passes")]
            Pass::FunctionAttrs => "function-attrs",
            #[cfg(feature = "full_o3_passes")]
            Pass::OpenMpOpt => "openmp-opt",
            #[cfg(feature = "full_o3_passes")]
            Pass::OpenMpOptCgscc => "openmp-opt-cgscc",
            // Loop
            #[cfg(feature = "full_o3_passes")]
            Pass::ExtraSimpleLoopUnswitchPasses => "extra-simple-loop-unswitch-passes",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopInstsimplify => "loop-instsimplify",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopSimplifycfg => "loop-simplifycfg",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopUnrollFull => "loop-unroll-full",
            // Function
            #[cfg(feature = "full_o3_passes")]
            Pass::AlignmentFromAssumptions => "alignment-from-assumptions",
            #[cfg(feature = "full_o3_passes")]
            Pass::AnnotationRemarks => "annotation-remarks",
            #[cfg(feature = "full_o3_passes")]
            Pass::CallsiteSplitting => "callsite-splitting",
            #[cfg(feature = "full_o3_passes")]
            Pass::Chr => "chr",
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroElide => "coro-elide",
            #[cfg(feature = "full_o3_passes")]
            Pass::DivRemPairs => "div-rem-pairs",
            #[cfg(feature = "full_o3_passes")]
            Pass::EeInstrument => "ee-instrument",
            #[cfg(feature = "full_o3_passes")]
            Pass::InferAlignment => "infer-alignment",
            #[cfg(feature = "full_o3_passes")]
            Pass::InjectTliMappings => "inject-tli-mappings",
            #[cfg(feature = "full_o3_passes")]
            Pass::InstSimplify => "instsimplify",
            #[cfg(feature = "full_o3_passes")]
            Pass::LibcallsShrinkwrap => "libcalls-shrinkwrap",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopDistribute => "loop-distribute",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopLoadElim => "loop-load-elim",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopSink => "loop-sink",
            #[cfg(feature = "full_o3_passes")]
            Pass::LowerConstantIntrinsics => "lower-constant-intrinsics",
            #[cfg(feature = "full_o3_passes")]
            Pass::LowerExpect => "lower-expect",
            #[cfg(feature = "full_o3_passes")]
            Pass::MldstMotion => "mldst-motion",
            #[cfg(feature = "full_o3_passes")]
            Pass::MoveAutoInit => "move-auto-init",
            #[cfg(feature = "full_o3_passes")]
            Pass::SpeculativeExecution => "speculative-execution",
            #[cfg(feature = "full_o3_passes")]
            Pass::TransformWarning => "transform-warning",
            #[cfg(feature = "full_o3_passes")]
            Pass::Verify => "verify",
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopUnrollAndJam => "loop-unroll-and-jam",

            Pass::Stop => "stop",
        }
    }

    /// Build a correctly-nested `opt -passes=` string for LLVM 20's new pass manager.
    pub fn to_opt_pipeline(passes: &[Pass]) -> String {
        let transforms: Vec<&Pass> = passes.iter().filter(|p| **p != Pass::Stop).collect();
        if transforms.is_empty() {
            return String::new();
        }

        let is_module = |p: &Pass| match p {
            Pass::GlobalOpt => true,
            #[cfg(feature = "full_o3_passes")]
            Pass::AlwaysInline          // runs at top-level in -O3, not inside cgscc
            | Pass::Annotation2Metadata
            | Pass::CalledValuePropagation
            | Pass::CgProfile
            | Pass::ConstMerge
            | Pass::CoroCleanup
            | Pass::CoroEarly
            | Pass::Deadargelim
            | Pass::ElimAvailExtern
            | Pass::ForceAttrs
            | Pass::GlobalDce
            | Pass::InferAttrs
            | Pass::Ipsccp
            | Pass::OpenMpOpt           // module-level; openmp-opt-cgscc is the cgscc variant
            | Pass::RelLookupTableConverter
            | Pass::RpoFunctionAttrs => true,
            _ => false,
        };

        let is_cgscc = |p: &Pass| match p {
            Pass::Inline | Pass::Argpromotion => true,
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroAnnotationElide   // inside cgscc(devirt<4>(...)) in -O3
            | Pass::CoroSplit
            | Pass::FunctionAttrs
            | Pass::OpenMpOptCgscc => true,
            _ => false,
        };

        let is_loop = |p: &Pass| match p {
            Pass::LoopRotate
            | Pass::LoopDeletion
            | Pass::IndVars
            | Pass::Licm
            | Pass::LoopIdiom
            | Pass::SimpleLoopUnswitch => true,
            #[cfg(feature = "full_o3_passes")]
            Pass::ExtraSimpleLoopUnswitchPasses
            | Pass::LoopInstsimplify
            | Pass::LoopSimplifycfg
            | Pass::LoopUnrollFull
            | Pass::LoopUnrollAndJam => true,
            _ => false,
        };

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

        let function_passes: Vec<String> = transforms
            .iter()
            .filter(|p| !is_module(p) && !is_cgscc(p))
            .map(|p| {
                if is_loop(p) {
                    match **p {
                        Pass::Licm => "loop-mssa(licm)".to_string(),
                        Pass::LoopRotate => "loop(loop-rotate)".to_string(),
                        Pass::LoopDeletion => "loop(loop-deletion)".to_string(),
                        Pass::IndVars => "loop(indvars)".to_string(),
                        Pass::LoopIdiom => "loop(loop-idiom)".to_string(),
                        Pass::SimpleLoopUnswitch => "loop(simple-loop-unswitch)".to_string(),
                        #[cfg(feature = "full_o3_passes")]
                        Pass::ExtraSimpleLoopUnswitchPasses => {
                            "loop(extra-simple-loop-unswitch-passes)".to_string()
                        }
                        #[cfg(feature = "full_o3_passes")]
                        Pass::LoopInstsimplify => "loop(loop-instsimplify)".to_string(),
                        #[cfg(feature = "full_o3_passes")]
                        Pass::LoopSimplifycfg => "loop(loop-simplifycfg)".to_string(),
                        #[cfg(feature = "full_o3_passes")]
                        Pass::LoopUnrollFull => "loop(loop-unroll-full)".to_string(),
                        #[cfg(feature = "full_o3_passes")]
                        Pass::LoopUnrollAndJam => "loop(loop-unroll-and-jam)".to_string(),
                        _ => unreachable!(),
                    }
                } else if **p == Pass::Instcombine {
                    "instcombine<no-verify-fixpoint>".to_string()
                } else {
                    p.opt_name().to_string()
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
                pipeline_parts.push(format!("function({})", function_passes.join(",")));
            } else {
                pipeline_parts.push(function_passes.join(","));
            }
        }

        pipeline_parts.join(",")
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0  => Pass::Instcombine,
            1  => Pass::Inline,
            2  => Pass::LoopUnroll,
            3  => Pass::Licm,
            4  => Pass::Gvn,
            5  => Pass::Sroa,
            6  => Pass::Mem2reg,
            7  => Pass::Simplifycfg,
            8  => Pass::Dse,
            9  => Pass::Reassociate,
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
            22 => Pass::LoopVectorize,
            23 => Pass::SlpVectorizer,
            24 => Pass::CorrelatedPropagation,
            25 => Pass::AggressiveInstcombine,
            26 => Pass::ConstraintElimination,
            27 => Pass::VectorCombine,
            28 => Pass::Float2int,
            29 => Pass::LoopIdiom,
            30 => Pass::SimpleLoopUnswitch,
            // Stop (base, no feature)
            #[cfg(not(feature = "full_o3_passes"))]
            31 => Pass::Stop,
            // Extended passes
            #[cfg(feature = "full_o3_passes")]
            32 => Pass::Annotation2Metadata,
            #[cfg(feature = "full_o3_passes")]
            33 => Pass::CalledValuePropagation,
            #[cfg(feature = "full_o3_passes")]
            34 => Pass::CgProfile,
            #[cfg(feature = "full_o3_passes")]
            35 => Pass::CoroAnnotationElide,
            #[cfg(feature = "full_o3_passes")]
            36 => Pass::ConstMerge,
            #[cfg(feature = "full_o3_passes")]
            37 => Pass::CoroCleanup,
            #[cfg(feature = "full_o3_passes")]
            38 => Pass::CoroEarly,
            #[cfg(feature = "full_o3_passes")]
            39 => Pass::Deadargelim,
            #[cfg(feature = "full_o3_passes")]
            40 => Pass::ElimAvailExtern,
            #[cfg(feature = "full_o3_passes")]
            41 => Pass::ForceAttrs,
            #[cfg(feature = "full_o3_passes")]
            42 => Pass::GlobalDce,
            #[cfg(feature = "full_o3_passes")]
            43 => Pass::InferAttrs,
            #[cfg(feature = "full_o3_passes")]
            44 => Pass::Ipsccp,
            #[cfg(feature = "full_o3_passes")]
            45 => Pass::RelLookupTableConverter,
            #[cfg(feature = "full_o3_passes")]
            46 => Pass::RpoFunctionAttrs,
            #[cfg(feature = "full_o3_passes")]
            47 => Pass::AlwaysInline,
            #[cfg(feature = "full_o3_passes")]
            48 => Pass::CoroSplit,
            #[cfg(feature = "full_o3_passes")]
            49 => Pass::FunctionAttrs,
            #[cfg(feature = "full_o3_passes")]
            50 => Pass::OpenMpOpt,
            #[cfg(feature = "full_o3_passes")]
            51 => Pass::OpenMpOptCgscc,
            #[cfg(feature = "full_o3_passes")]
            52 => Pass::ExtraSimpleLoopUnswitchPasses,
            #[cfg(feature = "full_o3_passes")]
            53 => Pass::LoopInstsimplify,
            #[cfg(feature = "full_o3_passes")]
            54 => Pass::LoopSimplifycfg,
            #[cfg(feature = "full_o3_passes")]
            55 => Pass::LoopUnrollFull,
            #[cfg(feature = "full_o3_passes")]
            56 => Pass::AlignmentFromAssumptions,
            #[cfg(feature = "full_o3_passes")]
            57 => Pass::AnnotationRemarks,
            #[cfg(feature = "full_o3_passes")]
            58 => Pass::CallsiteSplitting,
            #[cfg(feature = "full_o3_passes")]
            59 => Pass::Chr,
            #[cfg(feature = "full_o3_passes")]
            60 => Pass::CoroElide,
            #[cfg(feature = "full_o3_passes")]
            61 => Pass::DivRemPairs,
            #[cfg(feature = "full_o3_passes")]
            62 => Pass::EeInstrument,
            #[cfg(feature = "full_o3_passes")]
            63 => Pass::InferAlignment,
            #[cfg(feature = "full_o3_passes")]
            64 => Pass::InjectTliMappings,
            #[cfg(feature = "full_o3_passes")]
            65 => Pass::InstSimplify,
            #[cfg(feature = "full_o3_passes")]
            66 => Pass::LibcallsShrinkwrap,
            #[cfg(feature = "full_o3_passes")]
            67 => Pass::LoopDistribute,
            #[cfg(feature = "full_o3_passes")]
            68 => Pass::LoopLoadElim,
            #[cfg(feature = "full_o3_passes")]
            69 => Pass::LoopSink,
            #[cfg(feature = "full_o3_passes")]
            70 => Pass::LowerConstantIntrinsics,
            #[cfg(feature = "full_o3_passes")]
            71 => Pass::LowerExpect,
            #[cfg(feature = "full_o3_passes")]
            72 => Pass::MldstMotion,
            #[cfg(feature = "full_o3_passes")]
            73 => Pass::MoveAutoInit,
            #[cfg(feature = "full_o3_passes")]
            74 => Pass::SpeculativeExecution,
            #[cfg(feature = "full_o3_passes")]
            75 => Pass::TransformWarning,
            #[cfg(feature = "full_o3_passes")]
            76 => Pass::Verify,
            #[cfg(feature = "full_o3_passes")]
            77 => Pass::LoopUnrollAndJam,
            // Stop (extended)
            #[cfg(feature = "full_o3_passes")]
            78 => Pass::Stop,
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
            Pass::IndVars => 21,
            Pass::LoopVectorize => 22,
            Pass::SlpVectorizer => 23,
            Pass::CorrelatedPropagation => 24,
            Pass::AggressiveInstcombine => 25,
            Pass::ConstraintElimination => 26,
            Pass::VectorCombine => 27,
            Pass::Float2int => 28,
            Pass::LoopIdiom => 29,
            Pass::SimpleLoopUnswitch => 30,
            #[cfg(feature = "full_o3_passes")]
            Pass::Annotation2Metadata => 32,
            #[cfg(feature = "full_o3_passes")]
            Pass::CalledValuePropagation => 33,
            #[cfg(feature = "full_o3_passes")]
            Pass::CgProfile => 34,
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroAnnotationElide => 35,
            #[cfg(feature = "full_o3_passes")]
            Pass::ConstMerge => 36,
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroCleanup => 37,
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroEarly => 38,
            #[cfg(feature = "full_o3_passes")]
            Pass::Deadargelim => 39,
            #[cfg(feature = "full_o3_passes")]
            Pass::ElimAvailExtern => 40,
            #[cfg(feature = "full_o3_passes")]
            Pass::ForceAttrs => 41,
            #[cfg(feature = "full_o3_passes")]
            Pass::GlobalDce => 42,
            #[cfg(feature = "full_o3_passes")]
            Pass::InferAttrs => 43,
            #[cfg(feature = "full_o3_passes")]
            Pass::Ipsccp => 44,
            #[cfg(feature = "full_o3_passes")]
            Pass::RelLookupTableConverter => 45,
            #[cfg(feature = "full_o3_passes")]
            Pass::RpoFunctionAttrs => 46,
            #[cfg(feature = "full_o3_passes")]
            Pass::AlwaysInline => 47,
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroSplit => 48,
            #[cfg(feature = "full_o3_passes")]
            Pass::FunctionAttrs => 49,
            #[cfg(feature = "full_o3_passes")]
            Pass::OpenMpOpt => 50,
            #[cfg(feature = "full_o3_passes")]
            Pass::OpenMpOptCgscc => 51,
            #[cfg(feature = "full_o3_passes")]
            Pass::ExtraSimpleLoopUnswitchPasses => 52,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopInstsimplify => 53,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopSimplifycfg => 54,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopUnrollFull => 55,
            #[cfg(feature = "full_o3_passes")]
            Pass::AlignmentFromAssumptions => 56,
            #[cfg(feature = "full_o3_passes")]
            Pass::AnnotationRemarks => 57,
            #[cfg(feature = "full_o3_passes")]
            Pass::CallsiteSplitting => 58,
            #[cfg(feature = "full_o3_passes")]
            Pass::Chr => 59,
            #[cfg(feature = "full_o3_passes")]
            Pass::CoroElide => 60,
            #[cfg(feature = "full_o3_passes")]
            Pass::DivRemPairs => 61,
            #[cfg(feature = "full_o3_passes")]
            Pass::EeInstrument => 62,
            #[cfg(feature = "full_o3_passes")]
            Pass::InferAlignment => 63,
            #[cfg(feature = "full_o3_passes")]
            Pass::InjectTliMappings => 64,
            #[cfg(feature = "full_o3_passes")]
            Pass::InstSimplify => 65,
            #[cfg(feature = "full_o3_passes")]
            Pass::LibcallsShrinkwrap => 66,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopDistribute => 67,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopLoadElim => 68,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopSink => 69,
            #[cfg(feature = "full_o3_passes")]
            Pass::LowerConstantIntrinsics => 70,
            #[cfg(feature = "full_o3_passes")]
            Pass::LowerExpect => 71,
            #[cfg(feature = "full_o3_passes")]
            Pass::MldstMotion => 72,
            #[cfg(feature = "full_o3_passes")]
            Pass::MoveAutoInit => 73,
            #[cfg(feature = "full_o3_passes")]
            Pass::SpeculativeExecution => 74,
            #[cfg(feature = "full_o3_passes")]
            Pass::TransformWarning => 75,
            #[cfg(feature = "full_o3_passes")]
            Pass::Verify => 76,
            #[cfg(feature = "full_o3_passes")]
            Pass::LoopUnrollAndJam => 77,
            Pass::Stop => {
                #[cfg(not(feature = "full_o3_passes"))]
                { 31 }
                #[cfg(feature = "full_o3_passes")]
                { 78 }
            }
        }
    }

    /// Total number of actions (transforms + Stop).
    pub fn count() -> usize {
        #[cfg(not(feature = "full_o3_passes"))]
        { 32 }
        #[cfg(feature = "full_o3_passes")]
        { 79 }
    }

    /// All transform passes (excludes Stop).
    pub fn all_transforms() -> &'static [Pass] {
        #[cfg(not(feature = "full_o3_passes"))]
        {
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
                Pass::LoopVectorize,
                Pass::SlpVectorizer,
                Pass::CorrelatedPropagation,
                Pass::AggressiveInstcombine,
                Pass::ConstraintElimination,
                Pass::VectorCombine,
                Pass::Float2int,
                Pass::LoopIdiom,
                Pass::SimpleLoopUnswitch,
            ]
        }
        #[cfg(feature = "full_o3_passes")]
        {
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
                Pass::LoopVectorize,
                Pass::SlpVectorizer,
                Pass::CorrelatedPropagation,
                Pass::AggressiveInstcombine,
                Pass::ConstraintElimination,
                Pass::VectorCombine,
                Pass::Float2int,
                Pass::LoopIdiom,
                Pass::SimpleLoopUnswitch,
                // Extended
                Pass::Annotation2Metadata,
                Pass::CalledValuePropagation,
                Pass::CgProfile,
                Pass::CoroAnnotationElide,
                Pass::ConstMerge,
                Pass::CoroCleanup,
                Pass::CoroEarly,
                Pass::Deadargelim,
                Pass::ElimAvailExtern,
                Pass::ForceAttrs,
                Pass::GlobalDce,
                Pass::InferAttrs,
                Pass::Ipsccp,
                Pass::RelLookupTableConverter,
                Pass::RpoFunctionAttrs,
                Pass::AlwaysInline,
                Pass::CoroSplit,
                Pass::FunctionAttrs,
                Pass::OpenMpOpt,
                Pass::OpenMpOptCgscc,
                Pass::ExtraSimpleLoopUnswitchPasses,
                Pass::LoopInstsimplify,
                Pass::LoopSimplifycfg,
                Pass::LoopUnrollFull,
                Pass::AlignmentFromAssumptions,
                Pass::AnnotationRemarks,
                Pass::CallsiteSplitting,
                Pass::Chr,
                Pass::CoroElide,
                Pass::DivRemPairs,
                Pass::EeInstrument,
                Pass::InferAlignment,
                Pass::InjectTliMappings,
                Pass::InstSimplify,
                Pass::LibcallsShrinkwrap,
                Pass::LoopDistribute,
                Pass::LoopLoadElim,
                Pass::LoopSink,
                Pass::LowerConstantIntrinsics,
                Pass::LowerExpect,
                Pass::MldstMotion,
                Pass::MoveAutoInit,
                Pass::SpeculativeExecution,
                Pass::TransformWarning,
                Pass::Verify,
                Pass::LoopUnrollAndJam,
            ]
        }
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
            // index 31 is a gap (unused) when full_o3_passes is enabled
            #[cfg(feature = "full_o3_passes")]
            if i == 31 { continue; }
            assert_eq!(Pass::from_index(i).to_index(), i);
        }
    }

    #[test]
    fn test_pipeline_with_globalopt() {
        let passes = vec![Pass::GlobalOpt, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "module(globalopt),function(instcombine<no-verify-fixpoint>)");
    }

    #[test]
    fn test_pipeline_with_indvars() {
        let passes = vec![Pass::IndVars, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
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

    #[test]
    fn test_pipeline_with_vectorize() {
        let passes = vec![Pass::LoopVectorize, Pass::SlpVectorizer, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop-vectorize,slp-vectorizer,sroa");
    }

    #[test]
    fn test_pipeline_vectorize_with_inline() {
        let passes = vec![Pass::Inline, Pass::LoopVectorize, Pass::SlpVectorizer];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "cgscc(inline),function(loop-vectorize,slp-vectorizer)"
        );
    }

    #[cfg(not(feature = "full_o3_passes"))]
    #[test]
    fn test_base_count() {
        assert_eq!(Pass::count(), 32);
        assert_eq!(Pass::Stop.to_index(), 31);
        assert_eq!(Pass::all_transforms().len(), 31);
    }

    #[cfg(feature = "full_o3_passes")]
    #[test]
    fn test_extended_count() {
        assert_eq!(Pass::count(), 79);
        assert_eq!(Pass::Stop.to_index(), 78);
        assert_eq!(Pass::all_transforms().len(), 77);
    }

    #[cfg(feature = "full_o3_passes")]
    #[test]
    fn test_extended_module_passes() {
        let passes = vec![Pass::GlobalDce, Pass::ConstMerge, Pass::Ipsccp];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "module(globaldce,constmerge,ipsccp)");
    }

    #[cfg(feature = "full_o3_passes")]
    #[test]
    fn test_extended_cgscc_passes() {
        // always-inline is module-level; function-attrs is cgscc-level
        let passes = vec![Pass::AlwaysInline, Pass::FunctionAttrs, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "module(always-inline),cgscc(function-attrs),function(sroa)");
    }

    #[cfg(feature = "full_o3_passes")]
    #[test]
    fn test_extended_loop_passes() {
        let passes = vec![Pass::LoopInstsimplify, Pass::LoopUnrollFull];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop(loop-instsimplify),loop(loop-unroll-full)");
    }
}
