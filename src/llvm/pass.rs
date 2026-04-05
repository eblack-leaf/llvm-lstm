#[derive(PartialEq, Copy, Clone, Eq, Hash)]
pub(crate) enum Pass {
    Start,
    Instcombine,
    Mem2reg,
    Adce,
    Dse,
    Sccp,
    Reassociate,
    JumpThreading,
    Gvn,
    Sroa,
    SroaModifyCfg,
    Memcpyopt,
    Simplifycfg,
    Inline,
    EarlyCseMemssa,
    LoopRotate,
    LoopRotateHeaderDup,
    Licm,
    LicmAllowSpeculation,
    IndVars,
    LoopIdiom,
    LoopDeletion,
    SimpleLoopUnswitch,
    SimpleLoopUnswitchNontrivial,
    LoopUnroll,
    LoopUnrollO3,
    LoopVectorize,
    SlpVectorizer,
    Tailcallelim,
    Stop,
}
impl Pass {
    pub(crate) fn to_opt(self) -> &'static str {
        match self {
            Pass::Start => "start",
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
            Pass::Stop => "stop",
        }
    }
}

pub fn opt_pipeline(passes: &[Pass]) -> String {
    let transforms: Vec<&Pass> = passes.iter().filter(|p| **p != Pass::Stop).collect();
    if transforms.is_empty() {
        return String::new();
    }

    // Only `Inline` is a CGSCC pass in the primary set.
    let is_cgscc = |p: &Pass| matches!(p, Pass::Inline);

    // Loop‑level passes that must be wrapped in `loop(...)` or `loop-mssa(...)`.
    let is_loop = |p: &Pass| {
        matches!(
            p,
            Pass::LoopRotate
                | Pass::LoopRotateHeaderDup
                | Pass::LoopDeletion
                | Pass::IndVars
                | Pass::Licm
                | Pass::LicmAllowSpeculation
                | Pass::LoopIdiom
                | Pass::SimpleLoopUnswitch
                | Pass::SimpleLoopUnswitchNontrivial
        )
    };

    let cgscc_passes: Vec<String> = transforms
        .iter()
        .filter(|p| is_cgscc(p))
        .map(|p| p.to_opt().to_string())
        .collect();

    let function_passes: Vec<String> = transforms
        .iter()
        .filter(|p| !is_cgscc(p))
        .map(|p| {
            if is_loop(p) {
                match **p {
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
                    _ => unreachable!(),
                }
            } else if **p == Pass::Instcombine {
                "instcombine<no-verify-fixpoint>".to_string()
            } else {
                p.to_opt().to_string()
            }
        })
        .collect();

    let mut pipeline_parts = Vec::new();

    if !cgscc_passes.is_empty() {
        pipeline_parts.push(format!("cgscc({})", cgscc_passes.join(",")));
    }

    if !function_passes.is_empty() {
        if !cgscc_passes.is_empty() {
            pipeline_parts.push(format!("function({})", function_passes.join(",")));
        } else {
            pipeline_parts.push(function_passes.join(","));
        }
    }

    pipeline_parts.join(",")
}
