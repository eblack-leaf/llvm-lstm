use crate::llvm::benchmark::Baselines;
use crate::llvm::ir::{Ir, Source};
use std::path::PathBuf;
#[derive(Clone)]
pub(crate) struct Functions {
    pub(crate) functions: Vec<Function>,
}
impl Functions {
    pub(crate) fn new(dir: &PathBuf) -> Self {
        let read_dir = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("failed to read functions dir {dir:?}: {e}"));
        let mut functions: Vec<Function> = Vec::new();
        for entry in read_dir {
            let path = entry.expect("read_dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("c") {
                continue;
            }
            let name = path
                .file_stem()
                .expect("file stem")
                .to_string_lossy()
                .into_owned();
            let ir = Ir {
                file: path.with_extension("ll"),
            };
            functions.push(Function {
                name,
                source: Source { file: path },
                ir,
                baselines: None,
                ir_features: None,
            });
        }
        functions.sort_by(|a, b| a.name.cmp(&b.name));
        Self { functions }
    }
}
#[derive(Clone)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) source: Source,
    pub(crate) ir: Ir,
    /// None until collect_baselines has run; always Some during training.
    pub(crate) baselines: Option<Baselines>,
    /// Pre-computed chunked opcode histogram (k * IR_VOCAB_SIZE floats).
    /// Populated during the baseline phase alongside baselines.
    pub(crate) ir_features: Option<Vec<f32>>,
}
