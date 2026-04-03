use crate::llvm::benchmark::Baselines;
use crate::llvm::ir::{Ir, Source};
use std::path::PathBuf;
#[derive(Clone)]
pub(crate) struct Functions {
    pub(crate) functions: Vec<Function>,
}
impl Functions {
    pub(crate) async fn new(dir: &PathBuf) -> Self {
        let mut read_dir = tokio::fs::read_dir(dir)
            .await
            .unwrap_or_else(|e| panic!("failed to read functions dir {dir:?}: {e}"));
        let mut functions: Vec<Function> = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.expect("read_dir entry") {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("c") {
                continue;
            }
            let name = path
                .file_stem()
                .expect("file stem")
                .to_string_lossy()
                .into_owned();
            let ir = Ir { file: path.with_extension("ll") };
            functions.push(Function {
                name,
                source: Source { file: path },
                ir,
                baselines: None,
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
}
