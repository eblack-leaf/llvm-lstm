use std::fmt;

use serde::{Deserialize, Serialize};

/// A selectable optimization pass.
///
/// Index layout:
///   Primary (always):    0-27  (28 transforms) + Stop at 28   → count = 29
///   Secondary (feature): 0-27  (28 primary transforms)
///                        29-70 (42 secondary transforms)
///                        + Stop at 71                          → count = 72
///
/// Index 28 is a gap (invalid action) when secondary_passes is enabled.
/// Stop is at 28 (primary only) or 71 (secondary enabled).
///
/// ── Why primary vs secondary ─────────────────────────────────────────────────
///
/// PRIMARY passes satisfy all three:
///   1. Appear in -O3's inner devirt<4> function loop (the core repeated kernel)
///      or are a direct prerequisite that the inner loop depends on.
///   2. Have demonstrable, broad impact on typical C compute code specifically.
///   3. Cover a distinct optimization category — no two primaries are
///      redundant substitutes for each other.
///
/// SECONDARY passes fail at least one:
///   - Language-specific (coro-*, openmp-*): irrelevant for plain C.
///   - Interprocedural/module-level analytics (deadargelim, ipsccp, etc.):
///     useful but marginal for function-level hot-path tuning.
///   - Niche or rarely decisive standalone (bdce, float2int, vector-combine,
///     constraint-elimination): catches things other passes miss but rarely
///     worth a dedicated action in a constrained search space.
///   - Infrastructure / diagnostic (verify, ee-instrument, annotation-*).
///   - O3-parameterized variant whose base pass is already primary but the
///     variant's gain is modest enough to not justify primary space
///     (simplifycfg-o3, early-cse plain, argpromotion, globalopt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pass {
    // ── Primary passes (indices 0–26) ─────────────────────────────────────────
    //
    // Core instruction cleanup
    Instcombine,                // 0  — instruction combining; foundational for all IR clean-up
    Mem2reg,                    // 1  — alloca→SSA; must run early, everything depends on it
    Adce,                       // 2  — aggressive DCE; prunes dead computation after transforms
    Dse,                        // 3  — dead store elimination; critical for C pointer writes
    Sccp,                       // 4  — sparse conditional constant prop; resolves branch conditions
    Reassociate,                // 5  — arithmetic re-association; enables constant folding downstream
    JumpThreading,              // 6  — threads jumps; eliminates branch chains common in C
    Gvn,                        // 7  — global value numbering; eliminates cross-block redundancies
    //
    // Memory / struct promotion (two variants so model learns when CFG mods help)
    Sroa,                       // 8  — scalar replacement of aggregates (conservative, no CFG mods)
    SroaModifyCfg,              // 9  — sroa<modify-cfg>; O3's form, allows CFG restructuring
    Memcpyopt,                  // 10 — merges memcpy/stores; C code is full of struct copies
    //
    // CFG simplification
    Simplifycfg,                // 11 — simplifies CFG; needed after nearly every transform
    //
    // Inlining
    Inline,                     // 12 — function inlining; single biggest win for C call-heavy code
    //
    // CSE (only the MemSSA form; plain early-cse is strictly weaker for C)
    EarlyCseMemssa,             // 13 — early-cse<memssa>; eliminates redundant loads/stores via MemSSA
    //
    // Loop infrastructure (rotation must precede LICM, indvars, vectorization)
    LoopRotate,                 // 14 — loop rotation (conservative)
    LoopRotateHeaderDup,        // 15 — loop-rotate<header-duplication;no-prepare-for-lto>; O3 form
    //
    // Loop-invariant code motion (two variants; model learns when speculation helps)
    Licm,                       // 16 — LICM without speculation (conservative, safe for aliased loads)
    LicmAllowSpeculation,       // 17 — licm<allowspeculation>; O3's second LICM pass; hoists aggressively
    //
    // Loop canonicalization (prerequisite for vectorization and unrolling)
    IndVars,                    // 18 — canonicalizes induction variables
    LoopIdiom,                  // 19 — recognises memset/memcpy idioms; very common in C
    LoopDeletion,               // 20 — deletes provably empty or infinite-but-unused loops
    //
    // Loop unswitching (two variants)
    SimpleLoopUnswitch,         // 21 — trivial unswitching only (cheap, always safe)
    SimpleLoopUnswitchNontrivial, // 22 — simple-loop-unswitch<nontrivial;trivial>; O3 form, duplicates body
    //
    // Loop unrolling (two variants; model learns when aggressive thresholds hurt vs help)
    LoopUnroll,                 // 23 — partial unrolling at default thresholds
    LoopUnrollO3,               // 24 — loop-unroll<O3>; much higher threshold; O3's late unroll pass
    //
    // Vectorization
    LoopVectorize,              // 25 — auto-vectorizes counted loops; huge for C compute arrays
    SlpVectorizer,              // 26 — superword-level parallelism; vectorises straight-line C code
    Tailcallelim,               // 27 — tail call elimination; primary because tail_recursive.c and
    //      interpreter/recursive-descent patterns in the benchmark set benefit
    //      directly; the LSTM can learn to skip it for non-recursive benchmarks

    // ── Secondary passes (indices 29–84) ──────────────────────────────────────
    //
    // DEMOTED FROM OLD BASE — useful but not primary-tier for C:

    /// Plain early-cse without MemSSA.  Weaker than EarlyCseMemssa for
    /// pointer-heavy C; kept as secondary so the model can use the cheaper form.
    #[cfg(feature = "secondary_passes")]
    EarlyCse,                   // 29

    /// Bit-tracking dead code elimination.  Catches things instcombine misses
    /// (e.g. dead high bits) but rarely moves the needle alone.
    #[cfg(feature = "secondary_passes")]
    Bdce,                       // 30

    /// Argument promotion (pointer→value).  CGSCC-level; useful when inlining
    /// exposes pointer args that can be passed by value, but secondary because
    /// it requires callgraph context to matter much.
    #[cfg(feature = "secondary_passes")]
    Argpromotion,               // 31

    /// Global variable optimisation.  Module-level; rarely in the hot path of
    /// compute-focused C benchmarks which mostly operate on local/heap data.
    #[cfg(feature = "secondary_passes")]
    GlobalOpt,                  // 32

    /// Correlated value propagation.  Propagates value ranges across branches;
    /// overlaps significantly with SCCP and GVN already in primary.
    #[cfg(feature = "secondary_passes")]
    CorrelatedPropagation,      // 33

    /// Aggressive instcombine.  A heavier instcombine sweep for patterns the
    /// standard pass misses.  Marginal gain over primary Instcombine for C.
    #[cfg(feature = "secondary_passes")]
    AggressiveInstcombine,      // 34

    /// Constraint-based range elimination.  Newer pass; niche use cases;
    /// overlaps with SCCP for most C programs.
    #[cfg(feature = "secondary_passes")]
    ConstraintElimination,      // 35

    /// Vector combine.  Recombines vector operations after SLP/loop-vectorize;
    /// only useful when vectorization already fired.
    #[cfg(feature = "secondary_passes")]
    VectorCombine,              // 36

    /// Float-to-int conversion.  Detects loops that accidentally use float
    /// arithmetic for integer semantics; very niche for hand-written C.
    #[cfg(feature = "secondary_passes")]
    Float2int,                  // 37

    // OLD EXTENDED SET — module-level analytics:

    #[cfg(feature = "secondary_passes")]
    CalledValuePropagation,     // 38
    #[cfg(feature = "secondary_passes")]
    ConstMerge,                 // 39
    #[cfg(feature = "secondary_passes")]
    Deadargelim,                // 40
    #[cfg(feature = "secondary_passes")]
    ElimAvailExtern,            // 41
    #[cfg(feature = "secondary_passes")]
    GlobalDce,                  // 42
    #[cfg(feature = "secondary_passes")]
    InferAttrs,                 // 43
    #[cfg(feature = "secondary_passes")]
    Ipsccp,                     // 44
    #[cfg(feature = "secondary_passes")]
    RelLookupTableConverter,    // 45
    #[cfg(feature = "secondary_passes")]
    RpoFunctionAttrs,           // 46
    // CGSCC-level
    #[cfg(feature = "secondary_passes")]
    AlwaysInline,               // 47
    #[cfg(feature = "secondary_passes")]
    FunctionAttrs,              // 48
    // Loop-level
    #[cfg(feature = "secondary_passes")]
    ExtraSimpleLoopUnswitchPasses, // 49
    #[cfg(feature = "secondary_passes")]
    LoopInstsimplify,           // 50
    #[cfg(feature = "secondary_passes")]
    LoopSimplifycfg,            // 51
    #[cfg(feature = "secondary_passes")]
    LoopUnrollFull,             // 52
    // Function-level
    #[cfg(feature = "secondary_passes")]
    AlignmentFromAssumptions,   // 53
    #[cfg(feature = "secondary_passes")]
    CallsiteSplitting,          // 54
    #[cfg(feature = "secondary_passes")]
    Chr,                        // 55
    #[cfg(feature = "secondary_passes")]
    DivRemPairs,                // 56
    #[cfg(feature = "secondary_passes")]
    InferAlignment,             // 57
    #[cfg(feature = "secondary_passes")]
    InjectTliMappings,          // 58
    #[cfg(feature = "secondary_passes")]
    InstSimplify,               // 59
    #[cfg(feature = "secondary_passes")]
    LibcallsShrinkwrap,         // 60
    #[cfg(feature = "secondary_passes")]
    LoopDistribute,             // 61
    #[cfg(feature = "secondary_passes")]
    LoopLoadElim,               // 62
    #[cfg(feature = "secondary_passes")]
    LoopSink,                   // 63
    #[cfg(feature = "secondary_passes")]
    LowerConstantIntrinsics,    // 64
    #[cfg(feature = "secondary_passes")]
    LowerExpect,                // 65
    #[cfg(feature = "secondary_passes")]
    MldstMotion,                // 66
    #[cfg(feature = "secondary_passes")]
    MoveAutoInit,               // 67
    #[cfg(feature = "secondary_passes")]
    SpeculativeExecution,       // 68
    #[cfg(feature = "secondary_passes")]
    LoopUnrollAndJam,           // 69
    // O3-parameterised variant relegated to secondary
    /// simplifycfg with the full O3 flag set.  The speculate-blocks and
    /// simplify-cond-branch flags help most for irregular control flow; for
    /// compute-heavy C with regular loops the gain over plain simplifycfg is
    /// modest — not worth consuming a primary slot.
    #[cfg(feature = "secondary_passes")]
    SimplifycfgO3,              // 70

    // ── Terminal action ───────────────────────────────────────────────────────
    Stop, // 28 (primary only) or 71 (secondary enabled)
}

impl Pass {
    pub fn opt_name(&self) -> &str {
        match self {
            // Primary
            Pass::Instcombine => "instcombine",
            Pass::Mem2reg => "mem2reg",
            Pass::Adce => "adce",
            Pass::Dse => "dse",
            Pass::Sccp => "sccp",
            Pass::Reassociate => "reassociate",
            Pass::JumpThreading => "jump-threading",
            Pass::Gvn => "gvn",
            Pass::Sroa => "sroa",
            Pass::SroaModifyCfg => "sroa<modify-cfg>",
            Pass::Memcpyopt => "memcpyopt",
            Pass::Simplifycfg => "simplifycfg",
            Pass::Inline => "inline",
            Pass::EarlyCseMemssa => "early-cse<memssa>",
            Pass::LoopRotate => "loop-rotate",
            Pass::LoopRotateHeaderDup => "loop-rotate<header-duplication;no-prepare-for-lto>",
            Pass::Licm => "licm",
            Pass::LicmAllowSpeculation => "licm<allowspeculation>",
            Pass::IndVars => "indvars",
            Pass::LoopIdiom => "loop-idiom",
            Pass::LoopDeletion => "loop-deletion",
            Pass::SimpleLoopUnswitch => "simple-loop-unswitch",
            Pass::SimpleLoopUnswitchNontrivial => "simple-loop-unswitch<nontrivial;trivial>",
            Pass::LoopUnroll => "loop-unroll",
            Pass::LoopUnrollO3 => "loop-unroll<O3>",
            Pass::LoopVectorize => "loop-vectorize",
            Pass::SlpVectorizer => "slp-vectorizer",
            Pass::Tailcallelim => "tailcallelim",
            // Secondary — demoted base passes
            #[cfg(feature = "secondary_passes")]
            Pass::EarlyCse => "early-cse",
            #[cfg(feature = "secondary_passes")]
            Pass::Bdce => "bdce",
            #[cfg(feature = "secondary_passes")]
            Pass::Argpromotion => "argpromotion",
            #[cfg(feature = "secondary_passes")]
            Pass::GlobalOpt => "globalopt",
            #[cfg(feature = "secondary_passes")]
            Pass::CorrelatedPropagation => "correlated-propagation",
            #[cfg(feature = "secondary_passes")]
            Pass::AggressiveInstcombine => "aggressive-instcombine",
            #[cfg(feature = "secondary_passes")]
            Pass::ConstraintElimination => "constraint-elimination",
            #[cfg(feature = "secondary_passes")]
            Pass::VectorCombine => "vector-combine",
            #[cfg(feature = "secondary_passes")]
            Pass::Float2int => "float2int",
            // Secondary — module-level
            #[cfg(feature = "secondary_passes")]
            Pass::CalledValuePropagation => "called-value-propagation",
            #[cfg(feature = "secondary_passes")]
            Pass::ConstMerge => "constmerge",
            #[cfg(feature = "secondary_passes")]
            Pass::Deadargelim => "deadargelim",
            #[cfg(feature = "secondary_passes")]
            Pass::ElimAvailExtern => "elim-avail-extern",
            #[cfg(feature = "secondary_passes")]
            Pass::GlobalDce => "globaldce",
            #[cfg(feature = "secondary_passes")]
            Pass::InferAttrs => "inferattrs",
            #[cfg(feature = "secondary_passes")]
            Pass::Ipsccp => "ipsccp",
            #[cfg(feature = "secondary_passes")]
            Pass::RelLookupTableConverter => "rel-lookup-table-converter",
            #[cfg(feature = "secondary_passes")]
            Pass::RpoFunctionAttrs => "rpo-function-attrs",
            // Secondary — CGSCC-level
            #[cfg(feature = "secondary_passes")]
            Pass::AlwaysInline => "always-inline",
            #[cfg(feature = "secondary_passes")]
            Pass::FunctionAttrs => "function-attrs",
            // Secondary — loop-level
            #[cfg(feature = "secondary_passes")]
            Pass::ExtraSimpleLoopUnswitchPasses => "extra-simple-loop-unswitch-passes",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopInstsimplify => "loop-instsimplify",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopSimplifycfg => "loop-simplifycfg",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopUnrollFull => "loop-unroll-full",
            // Secondary — function-level
            #[cfg(feature = "secondary_passes")]
            Pass::AlignmentFromAssumptions => "alignment-from-assumptions",
            #[cfg(feature = "secondary_passes")]
            Pass::CallsiteSplitting => "callsite-splitting",
            #[cfg(feature = "secondary_passes")]
            Pass::Chr => "chr",
            #[cfg(feature = "secondary_passes")]
            Pass::DivRemPairs => "div-rem-pairs",
            #[cfg(feature = "secondary_passes")]
            Pass::InferAlignment => "infer-alignment",
            #[cfg(feature = "secondary_passes")]
            Pass::InjectTliMappings => "inject-tli-mappings",
            #[cfg(feature = "secondary_passes")]
            Pass::InstSimplify => "instsimplify",
            #[cfg(feature = "secondary_passes")]
            Pass::LibcallsShrinkwrap => "libcalls-shrinkwrap",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopDistribute => "loop-distribute",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopLoadElim => "loop-load-elim",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopSink => "loop-sink",
            #[cfg(feature = "secondary_passes")]
            Pass::LowerConstantIntrinsics => "lower-constant-intrinsics",
            #[cfg(feature = "secondary_passes")]
            Pass::LowerExpect => "lower-expect",
            #[cfg(feature = "secondary_passes")]
            Pass::MldstMotion => "mldst-motion",
            #[cfg(feature = "secondary_passes")]
            Pass::MoveAutoInit => "move-auto-init",
            #[cfg(feature = "secondary_passes")]
            Pass::SpeculativeExecution => "speculative-execution",
            #[cfg(feature = "secondary_passes")]
            Pass::LoopUnrollAndJam => "loop-unroll-and-jam",
            #[cfg(feature = "secondary_passes")]
            Pass::SimplifycfgO3 => concat!(
                "simplifycfg<bonus-inst-threshold=1;no-forward-switch-cond;",
                "switch-range-to-icmp;no-switch-to-lookup;keep-loops;",
                "no-hoist-common-insts;no-hoist-loads-stores-with-cond-faulting;",
                "no-sink-common-insts;speculate-blocks;simplify-cond-branch;",
                "no-speculate-unpredictables>"
            ),

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
            #[cfg(feature = "secondary_passes")]
            Pass::GlobalOpt
            | Pass::AlwaysInline
            | Pass::CalledValuePropagation
            | Pass::ConstMerge
            | Pass::Deadargelim
            | Pass::ElimAvailExtern
            | Pass::GlobalDce
            | Pass::InferAttrs
            | Pass::Ipsccp
            | Pass::RelLookupTableConverter
            | Pass::RpoFunctionAttrs => true,
            _ => false,
        };

        let is_cgscc = |p: &Pass| match p {
            Pass::Inline => true,
            #[cfg(feature = "secondary_passes")]
            Pass::Argpromotion | Pass::FunctionAttrs => true,
            _ => false,
        };

        // Passes that must be wrapped in loop(...) or loop-mssa(...).
        // Note: LoopUnroll, LoopUnrollO3, LoopVectorize, SlpVectorizer are
        // function-level adaptors, NOT loop-level — they iterate loops
        // internally and do not need a loop(...) wrapper.
        let is_loop = |p: &Pass| match p {
            Pass::LoopRotate
            | Pass::LoopRotateHeaderDup
            | Pass::LoopDeletion
            | Pass::IndVars
            | Pass::Licm
            | Pass::LicmAllowSpeculation
            | Pass::LoopIdiom
            | Pass::SimpleLoopUnswitch
            | Pass::SimpleLoopUnswitchNontrivial => true,
            #[cfg(feature = "secondary_passes")]
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
                        // Primary loop passes
                        Pass::Licm => "loop-mssa(licm)".to_string(),
                        Pass::LicmAllowSpeculation => "loop-mssa(licm<allowspeculation>)".to_string(),
                        Pass::LoopRotate => "loop(loop-rotate)".to_string(),
                        Pass::LoopRotateHeaderDup => {
                            "loop(loop-rotate<header-duplication;no-prepare-for-lto>)".to_string()
                        }
                        Pass::LoopDeletion => "loop(loop-deletion)".to_string(),
                        Pass::IndVars => "loop(indvars)".to_string(),
                        Pass::LoopIdiom => "loop(loop-idiom)".to_string(),
                        Pass::SimpleLoopUnswitch => "loop(simple-loop-unswitch)".to_string(),
                        Pass::SimpleLoopUnswitchNontrivial => {
                            "loop(simple-loop-unswitch<nontrivial;trivial>)".to_string()
                        }
                        // Secondary loop passes
                        #[cfg(feature = "secondary_passes")]
                        Pass::ExtraSimpleLoopUnswitchPasses => {
                            "loop(extra-simple-loop-unswitch-passes)".to_string()
                        }
                        #[cfg(feature = "secondary_passes")]
                        Pass::LoopInstsimplify => "loop(loop-instsimplify)".to_string(),
                        #[cfg(feature = "secondary_passes")]
                        Pass::LoopSimplifycfg => "loop(loop-simplifycfg)".to_string(),
                        #[cfg(feature = "secondary_passes")]
                        Pass::LoopUnrollFull => "loop(loop-unroll-full)".to_string(),
                        #[cfg(feature = "secondary_passes")]
                        Pass::LoopUnrollAndJam => "loop(loop-unroll-and-jam)".to_string(),
                        _ => unreachable!(),
                    }
                } else if **p == Pass::Instcombine {
                    "instcombine<no-verify-fixpoint>".to_string()
                } else {
                    // All other function-level passes.  Parameterised variants
                    // (SroaModifyCfg, EarlyCseMemssa, LoopUnrollO3, SimplifycfgO3, …)
                    // carry their full opt string inside opt_name() already.
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
            // Primary
            0  => Pass::Instcombine,
            1  => Pass::Mem2reg,
            2  => Pass::Adce,
            3  => Pass::Dse,
            4  => Pass::Sccp,
            5  => Pass::Reassociate,
            6  => Pass::JumpThreading,
            7  => Pass::Gvn,
            8  => Pass::Sroa,
            9  => Pass::SroaModifyCfg,
            10 => Pass::Memcpyopt,
            11 => Pass::Simplifycfg,
            12 => Pass::Inline,
            13 => Pass::EarlyCseMemssa,
            14 => Pass::LoopRotate,
            15 => Pass::LoopRotateHeaderDup,
            16 => Pass::Licm,
            17 => Pass::LicmAllowSpeculation,
            18 => Pass::IndVars,
            19 => Pass::LoopIdiom,
            20 => Pass::LoopDeletion,
            21 => Pass::SimpleLoopUnswitch,
            22 => Pass::SimpleLoopUnswitchNontrivial,
            23 => Pass::LoopUnroll,
            24 => Pass::LoopUnrollO3,
            25 => Pass::LoopVectorize,
            26 => Pass::SlpVectorizer,
            27 => Pass::Tailcallelim,
            // Stop (primary only)
            #[cfg(not(feature = "secondary_passes"))]
            28 => Pass::Stop,
            // 28 is a gap when secondary_passes is enabled
            // Secondary (29–70)
            #[cfg(feature = "secondary_passes")]
            29 => Pass::EarlyCse,
            #[cfg(feature = "secondary_passes")]
            30 => Pass::Bdce,
            #[cfg(feature = "secondary_passes")]
            31 => Pass::Argpromotion,
            #[cfg(feature = "secondary_passes")]
            32 => Pass::GlobalOpt,
            #[cfg(feature = "secondary_passes")]
            33 => Pass::CorrelatedPropagation,
            #[cfg(feature = "secondary_passes")]
            34 => Pass::AggressiveInstcombine,
            #[cfg(feature = "secondary_passes")]
            35 => Pass::ConstraintElimination,
            #[cfg(feature = "secondary_passes")]
            36 => Pass::VectorCombine,
            #[cfg(feature = "secondary_passes")]
            37 => Pass::Float2int,
            #[cfg(feature = "secondary_passes")]
            38 => Pass::CalledValuePropagation,
            #[cfg(feature = "secondary_passes")]
            39 => Pass::ConstMerge,
            #[cfg(feature = "secondary_passes")]
            40 => Pass::Deadargelim,
            #[cfg(feature = "secondary_passes")]
            41 => Pass::ElimAvailExtern,
            #[cfg(feature = "secondary_passes")]
            42 => Pass::GlobalDce,
            #[cfg(feature = "secondary_passes")]
            43 => Pass::InferAttrs,
            #[cfg(feature = "secondary_passes")]
            44 => Pass::Ipsccp,
            #[cfg(feature = "secondary_passes")]
            45 => Pass::RelLookupTableConverter,
            #[cfg(feature = "secondary_passes")]
            46 => Pass::RpoFunctionAttrs,
            #[cfg(feature = "secondary_passes")]
            47 => Pass::AlwaysInline,
            #[cfg(feature = "secondary_passes")]
            48 => Pass::FunctionAttrs,
            #[cfg(feature = "secondary_passes")]
            49 => Pass::ExtraSimpleLoopUnswitchPasses,
            #[cfg(feature = "secondary_passes")]
            50 => Pass::LoopInstsimplify,
            #[cfg(feature = "secondary_passes")]
            51 => Pass::LoopSimplifycfg,
            #[cfg(feature = "secondary_passes")]
            52 => Pass::LoopUnrollFull,
            #[cfg(feature = "secondary_passes")]
            53 => Pass::AlignmentFromAssumptions,
            #[cfg(feature = "secondary_passes")]
            54 => Pass::CallsiteSplitting,
            #[cfg(feature = "secondary_passes")]
            55 => Pass::Chr,
            #[cfg(feature = "secondary_passes")]
            56 => Pass::DivRemPairs,
            #[cfg(feature = "secondary_passes")]
            57 => Pass::InferAlignment,
            #[cfg(feature = "secondary_passes")]
            58 => Pass::InjectTliMappings,
            #[cfg(feature = "secondary_passes")]
            59 => Pass::InstSimplify,
            #[cfg(feature = "secondary_passes")]
            60 => Pass::LibcallsShrinkwrap,
            #[cfg(feature = "secondary_passes")]
            61 => Pass::LoopDistribute,
            #[cfg(feature = "secondary_passes")]
            62 => Pass::LoopLoadElim,
            #[cfg(feature = "secondary_passes")]
            63 => Pass::LoopSink,
            #[cfg(feature = "secondary_passes")]
            64 => Pass::LowerConstantIntrinsics,
            #[cfg(feature = "secondary_passes")]
            65 => Pass::LowerExpect,
            #[cfg(feature = "secondary_passes")]
            66 => Pass::MldstMotion,
            #[cfg(feature = "secondary_passes")]
            67 => Pass::MoveAutoInit,
            #[cfg(feature = "secondary_passes")]
            68 => Pass::SpeculativeExecution,
            #[cfg(feature = "secondary_passes")]
            69 => Pass::LoopUnrollAndJam,
            #[cfg(feature = "secondary_passes")]
            70 => Pass::SimplifycfgO3,
            // Stop (secondary) — secondary block is 29–70, so Stop = 71
            #[cfg(feature = "secondary_passes")]
            71 => Pass::Stop,
            _ => panic!("Invalid pass index: {i}"),
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            // Primary
            Pass::Instcombine => 0,
            Pass::Mem2reg => 1,
            Pass::Adce => 2,
            Pass::Dse => 3,
            Pass::Sccp => 4,
            Pass::Reassociate => 5,
            Pass::JumpThreading => 6,
            Pass::Gvn => 7,
            Pass::Sroa => 8,
            Pass::SroaModifyCfg => 9,
            Pass::Memcpyopt => 10,
            Pass::Simplifycfg => 11,
            Pass::Inline => 12,
            Pass::EarlyCseMemssa => 13,
            Pass::LoopRotate => 14,
            Pass::LoopRotateHeaderDup => 15,
            Pass::Licm => 16,
            Pass::LicmAllowSpeculation => 17,
            Pass::IndVars => 18,
            Pass::LoopIdiom => 19,
            Pass::LoopDeletion => 20,
            Pass::SimpleLoopUnswitch => 21,
            Pass::SimpleLoopUnswitchNontrivial => 22,
            Pass::LoopUnroll => 23,
            Pass::LoopUnrollO3 => 24,
            Pass::LoopVectorize => 25,
            Pass::SlpVectorizer => 26,
            Pass::Tailcallelim => 27,
            // Secondary
            #[cfg(feature = "secondary_passes")]
            Pass::EarlyCse => 29,
            #[cfg(feature = "secondary_passes")]
            Pass::Bdce => 30,
            #[cfg(feature = "secondary_passes")]
            Pass::Argpromotion => 31,
            #[cfg(feature = "secondary_passes")]
            Pass::GlobalOpt => 32,
            #[cfg(feature = "secondary_passes")]
            Pass::CorrelatedPropagation => 33,
            #[cfg(feature = "secondary_passes")]
            Pass::AggressiveInstcombine => 34,
            #[cfg(feature = "secondary_passes")]
            Pass::ConstraintElimination => 35,
            #[cfg(feature = "secondary_passes")]
            Pass::VectorCombine => 36,
            #[cfg(feature = "secondary_passes")]
            Pass::Float2int => 37,
            #[cfg(feature = "secondary_passes")]
            Pass::CalledValuePropagation => 38,
            #[cfg(feature = "secondary_passes")]
            Pass::ConstMerge => 39,
            #[cfg(feature = "secondary_passes")]
            Pass::Deadargelim => 40,
            #[cfg(feature = "secondary_passes")]
            Pass::ElimAvailExtern => 41,
            #[cfg(feature = "secondary_passes")]
            Pass::GlobalDce => 42,
            #[cfg(feature = "secondary_passes")]
            Pass::InferAttrs => 43,
            #[cfg(feature = "secondary_passes")]
            Pass::Ipsccp => 44,
            #[cfg(feature = "secondary_passes")]
            Pass::RelLookupTableConverter => 45,
            #[cfg(feature = "secondary_passes")]
            Pass::RpoFunctionAttrs => 46,
            #[cfg(feature = "secondary_passes")]
            Pass::AlwaysInline => 47,
            #[cfg(feature = "secondary_passes")]
            Pass::FunctionAttrs => 48,
            #[cfg(feature = "secondary_passes")]
            Pass::ExtraSimpleLoopUnswitchPasses => 49,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopInstsimplify => 50,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopSimplifycfg => 51,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopUnrollFull => 52,
            #[cfg(feature = "secondary_passes")]
            Pass::AlignmentFromAssumptions => 53,
            #[cfg(feature = "secondary_passes")]
            Pass::CallsiteSplitting => 54,
            #[cfg(feature = "secondary_passes")]
            Pass::Chr => 55,
            #[cfg(feature = "secondary_passes")]
            Pass::DivRemPairs => 56,
            #[cfg(feature = "secondary_passes")]
            Pass::InferAlignment => 57,
            #[cfg(feature = "secondary_passes")]
            Pass::InjectTliMappings => 58,
            #[cfg(feature = "secondary_passes")]
            Pass::InstSimplify => 59,
            #[cfg(feature = "secondary_passes")]
            Pass::LibcallsShrinkwrap => 60,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopDistribute => 61,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopLoadElim => 62,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopSink => 63,
            #[cfg(feature = "secondary_passes")]
            Pass::LowerConstantIntrinsics => 64,
            #[cfg(feature = "secondary_passes")]
            Pass::LowerExpect => 65,
            #[cfg(feature = "secondary_passes")]
            Pass::MldstMotion => 66,
            #[cfg(feature = "secondary_passes")]
            Pass::MoveAutoInit => 67,
            #[cfg(feature = "secondary_passes")]
            Pass::SpeculativeExecution => 68,
            #[cfg(feature = "secondary_passes")]
            Pass::LoopUnrollAndJam => 69,
            #[cfg(feature = "secondary_passes")]
            Pass::SimplifycfgO3 => 70,
            Pass::Stop => {
                #[cfg(not(feature = "secondary_passes"))]
                { 28 }
                #[cfg(feature = "secondary_passes")]
                { 71 }
            }
        }
    }

    /// Total number of actions (transforms + Stop).
    pub fn count() -> usize {
        #[cfg(not(feature = "secondary_passes"))]
        { 29 }
        #[cfg(feature = "secondary_passes")]
        { 72 }
    }

    /// All transform passes (excludes Stop).
    pub fn all_transforms() -> &'static [Pass] {
        #[cfg(not(feature = "secondary_passes"))]
        {
            &[
                Pass::Instcombine,
                Pass::Mem2reg,
                Pass::Adce,
                Pass::Dse,
                Pass::Sccp,
                Pass::Reassociate,
                Pass::JumpThreading,
                Pass::Gvn,
                Pass::Sroa,
                Pass::SroaModifyCfg,
                Pass::Memcpyopt,
                Pass::Simplifycfg,
                Pass::Inline,
                Pass::EarlyCseMemssa,
                Pass::LoopRotate,
                Pass::LoopRotateHeaderDup,
                Pass::Licm,
                Pass::LicmAllowSpeculation,
                Pass::IndVars,
                Pass::LoopIdiom,
                Pass::LoopDeletion,
                Pass::SimpleLoopUnswitch,
                Pass::SimpleLoopUnswitchNontrivial,
                Pass::LoopUnroll,
                Pass::LoopUnrollO3,
                Pass::LoopVectorize,
                Pass::SlpVectorizer,
                Pass::Tailcallelim,
            ]
        }
        #[cfg(feature = "secondary_passes")]
        {
            &[
                // Primary
                Pass::Instcombine,
                Pass::Mem2reg,
                Pass::Adce,
                Pass::Dse,
                Pass::Sccp,
                Pass::Reassociate,
                Pass::JumpThreading,
                Pass::Gvn,
                Pass::Sroa,
                Pass::SroaModifyCfg,
                Pass::Memcpyopt,
                Pass::Simplifycfg,
                Pass::Inline,
                Pass::EarlyCseMemssa,
                Pass::LoopRotate,
                Pass::LoopRotateHeaderDup,
                Pass::Licm,
                Pass::LicmAllowSpeculation,
                Pass::IndVars,
                Pass::LoopIdiom,
                Pass::LoopDeletion,
                Pass::SimpleLoopUnswitch,
                Pass::SimpleLoopUnswitchNontrivial,
                Pass::LoopUnroll,
                Pass::LoopUnrollO3,
                Pass::LoopVectorize,
                Pass::SlpVectorizer,
                Pass::Tailcallelim,
                // Secondary
                Pass::EarlyCse,
                Pass::Bdce,
                Pass::Argpromotion,
                Pass::GlobalOpt,
                Pass::CorrelatedPropagation,
                Pass::AggressiveInstcombine,
                Pass::ConstraintElimination,
                Pass::VectorCombine,
                Pass::Float2int,
                Pass::CalledValuePropagation,
                Pass::ConstMerge,
                Pass::Deadargelim,
                Pass::ElimAvailExtern,
                Pass::GlobalDce,
                Pass::InferAttrs,
                Pass::Ipsccp,
                Pass::RelLookupTableConverter,
                Pass::RpoFunctionAttrs,
                Pass::AlwaysInline,
                Pass::FunctionAttrs,
                Pass::ExtraSimpleLoopUnswitchPasses,
                Pass::LoopInstsimplify,
                Pass::LoopSimplifycfg,
                Pass::LoopUnrollFull,
                Pass::AlignmentFromAssumptions,
                Pass::CallsiteSplitting,
                Pass::Chr,
                Pass::DivRemPairs,
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
                Pass::LoopUnrollAndJam,
                Pass::SimplifycfgO3,
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
    fn test_pipeline_with_licm_allow_speculation() {
        let passes = vec![Pass::LicmAllowSpeculation, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "loop-mssa(licm<allowspeculation>),instcombine<no-verify-fixpoint>"
        );
    }

    #[test]
    fn test_licm_variants_coexist() {
        let passes = vec![Pass::Licm, Pass::LicmAllowSpeculation];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop-mssa(licm),loop-mssa(licm<allowspeculation>)");
    }

    #[test]
    fn test_sroa_variants() {
        let passes = vec![Pass::Sroa, Pass::SroaModifyCfg];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "sroa,sroa<modify-cfg>");
    }

    #[test]
    fn test_pipeline_with_loop_rotate_variants() {
        let passes = vec![Pass::LoopRotate, Pass::LoopRotateHeaderDup];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "loop(loop-rotate),loop(loop-rotate<header-duplication;no-prepare-for-lto>)"
        );
    }

    #[test]
    fn test_simple_loop_unswitch_variants() {
        let passes = vec![Pass::SimpleLoopUnswitch, Pass::SimpleLoopUnswitchNontrivial];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "loop(simple-loop-unswitch),loop(simple-loop-unswitch<nontrivial;trivial>)"
        );
    }

    #[test]
    fn test_loop_unroll_variants() {
        // Both are function-level adaptors, not wrapped in loop(...)
        let passes = vec![Pass::LoopUnroll, Pass::LoopUnrollO3];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop-unroll,loop-unroll<O3>");
    }

    #[test]
    fn test_early_cse_memssa() {
        let passes = vec![Pass::EarlyCseMemssa, Pass::Gvn];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "early-cse<memssa>,gvn");
    }

    #[test]
    fn test_pipeline_stop_filtered() {
        let passes = vec![Pass::Instcombine, Pass::Stop];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "instcombine<no-verify-fixpoint>");
    }

    #[test]
    fn test_pipeline_mixed_levels() {
        let passes = vec![Pass::Inline, Pass::IndVars, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "cgscc(inline),function(loop(indvars),instcombine<no-verify-fixpoint>)"
        );
    }

    #[test]
    fn test_pipeline_with_vectorize() {
        // LoopVectorize and SlpVectorizer are function-level, not loop(...)
        let passes = vec![Pass::LoopVectorize, Pass::SlpVectorizer, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop-vectorize,slp-vectorizer,sroa");
    }

    #[test]
    fn test_roundtrip_index() {
        for i in 0..Pass::count() {
            // index 28 is a gap (unused) when secondary_passes is enabled
            #[cfg(feature = "secondary_passes")]
            if i == 28 { continue; }
            assert_eq!(Pass::from_index(i).to_index(), i, "roundtrip failed at index {i}");
        }
    }

    #[cfg(not(feature = "secondary_passes"))]
    #[test]
    fn test_primary_count() {
        assert_eq!(Pass::count(), 29);
        assert_eq!(Pass::Stop.to_index(), 28);
        assert_eq!(Pass::all_transforms().len(), 28);
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_secondary_count() {
        assert_eq!(Pass::count(), 72);
        assert_eq!(Pass::Stop.to_index(), 71);
        // 28 primary + 42 secondary = 70 transforms
        assert_eq!(Pass::all_transforms().len(), 70);
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_secondary_module_passes() {
        let passes = vec![Pass::GlobalDce, Pass::ConstMerge, Pass::Ipsccp];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "module(globaldce,constmerge,ipsccp)");
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_secondary_cgscc_passes() {
        let passes = vec![Pass::AlwaysInline, Pass::FunctionAttrs, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "module(always-inline),cgscc(function-attrs),function(sroa)");
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_secondary_argpromotion() {
        let passes = vec![Pass::Argpromotion, Pass::Inline, Pass::Sroa];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "cgscc(argpromotion,inline),function(sroa)");
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_secondary_loop_passes() {
        let passes = vec![Pass::LoopInstsimplify, Pass::LoopUnrollFull];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(pipeline, "loop(loop-instsimplify),loop(loop-unroll-full)");
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_simplifycfg_o3() {
        let passes = vec![Pass::SimplifycfgO3];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert!(pipeline.starts_with("simplifycfg<bonus-inst-threshold=1;"));
        assert!(pipeline.contains("speculate-blocks"));
        assert!(pipeline.contains("simplify-cond-branch"));
        assert!(pipeline.contains("switch-range-to-icmp"));
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_secondary_globalopt_is_module() {
        let passes = vec![Pass::GlobalOpt, Pass::Instcombine];
        let pipeline = Pass::to_opt_pipeline(&passes);
        assert_eq!(
            pipeline,
            "module(globalopt),function(instcombine<no-verify-fixpoint>)"
        );
    }

    #[cfg(feature = "secondary_passes")]
    #[test]
    fn test_gap_index_28_not_valid() {
        assert_eq!(Pass::SlpVectorizer.to_index(), 26);
        assert_eq!(Pass::Tailcallelim.to_index(), 27);
        assert_eq!(Pass::EarlyCse.to_index(), 29); // 28 is the gap
        assert_eq!(Pass::Stop.to_index(), 71);
    }
}
