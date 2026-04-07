use crate::config::Cfg;
use crate::ppo::model::{Actor, Input, MlpHead, MlpHeadConfig, Output};
use burn::nn::transformer::{
    TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput,
};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Bool, Config, Int, Module};
use burn::tensor::Tensor;

#[derive(Config, Debug)]
pub(crate) struct ConclaveActorConfig {
    #[config(default = 40)]
    pub(crate) input_dim: usize,
    #[config(default = 29)]
    pub(crate) num_passes: usize,
    #[config(default = 256)]
    pub(crate) d_model: usize,
    #[config(default = 4)]
    pub(crate) n_heads: usize,
    #[config(default = 3)]
    pub(crate) n_layers: usize,
    #[config(default = 512)]
    pub(crate) d_ff: usize,
    #[config(default = 0.1)]
    pub(crate) dropout: f64,
    #[config(default = 40)]
    pub(crate) max_seq_len: usize,
    #[config(default = 128)]
    pub(crate) head_hidden: usize,
}

/// ConclaveActor — passes convene with full knowledge of the slot ordering context,
/// slots listen to the resolved pass deliberation without coordinating with each other.
///
/// Joint sequence: [pass_0 .. pass_28 | slot_0 .. slot_{K-1}]
///
/// Attention mask:
///   pass → pass : open   — passes deliberate bidirectionally, all seeing all
///   pass → slot : open   — passes know which positions are asking
///   slot → pass : open   — slots absorb the full pass deliberation
///   slot → slot : closed — slots don't coordinate; each reads the conclave independently
///
/// This means:
///   - instcombine resolves against inline knowing slot 0 is querying nearby — ordering
///     context flows into the pass negotiation, not bolted on after
///   - each slot's policy and value are conditioned on passes that resolved WITH that
///     slot's positional identity present — credit flows back to pass embeddings, not
///     to position embeddings
///   - gradient for pass_i accumulates from every slot that attended to it across all
///     episodes — one coherent signal per pass identity, not one per (pass, position) pair
#[derive(Module, Debug)]
pub(crate) struct ConclaveActor<B: Backend> {
    ir_proj: Linear<B>,
    pass_embed: Embedding<B>,
    slot_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    policy_head: MlpHead<B>,
    value_head: MlpHead<B>,
}

impl<B: Backend> Actor<B> for ConclaveActor<B> {
    type Config = ConclaveActorConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        Self {
            ir_proj: LinearConfig::new(cfg.input_dim, cfg.d_model).init(device),
            pass_embed: EmbeddingConfig::new(cfg.num_passes, cfg.d_model).init(device),
            slot_embed: EmbeddingConfig::new(cfg.max_seq_len, cfg.d_model).init(device),
            transformer: TransformerEncoderConfig::new(
                cfg.d_model,
                cfg.d_ff,
                cfg.n_heads,
                cfg.n_layers,
            )
            .with_dropout(cfg.dropout)
            .init(device),
            policy_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, cfg.num_passes)
                .init(device),
            value_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, 1).init(device),
        }
    }

    fn forward(&self, cfg: &Cfg, input: Input<B>) -> Output<B> {
        let n = input.ir_features.dims()[0]; // batch (episodes)
        let k = cfg.max_seq_len;             // slots per episode
        let device = input.ir_features.device();
        let np = 29usize; // number of passes (fixed)
        let seq = np + k; // joint sequence length

        // IR embedding: project IR features directly — [N, nf] → [N, d_model]
        let ir_emb = self.ir_proj.forward(input.ir_features); // [N, d_model]

        // Pass nodes: learned pass identity + IR conditioning, broadcast over batch.
        // [1, np] → embed → [1, np, d_model] + ir_emb [N, 1, d_model] → [N, np, d_model]
        let pass_ids = Tensor::<B, 1, Int>::arange(0..np as i64, &device).unsqueeze_dim(0); // [1, np]
        let pass_emb = self.pass_embed.forward(pass_ids)                   // [1, np, d_model]
            .repeat(&[n, 1, 1])                                              // [N, np, d_model]
            + ir_emb.clone().unsqueeze_dim(1).repeat(&[1, np, 1]); // broadcast IR

        // Slot nodes: positional identity, broadcast over batch.
        // [1, K] → embed → [1, K, d_model] → [N, K, d_model]
        let slot_ids = Tensor::<B, 1, Int>::arange(0..k as i64, &device).unsqueeze_dim(0); // [1, K]
        let slot_emb = self
            .slot_embed
            .forward(slot_ids) // [1, K, d_model]
            .repeat(&[n, 1, 1]); // [N, K, d_model]

        // Joint sequence: [pass_0..pass_28 | slot_0..slot_{K-1}]
        let tokens = Tensor::cat(vec![pass_emb, slot_emb], 1); // [N, np+K, d_model]

        // Attention mask [1, seq, seq]:  true = blocked.
        //   pass i → pass j : always open  (i < np, j < np)
        //   pass i → slot j : always open  (i < np, j >= np)
        //   slot i → pass j : always open  (i >= np, j < np)
        //   slot i → slot j : blocked      (i >= np, j >= np, i != j)
        let mask_data: Vec<bool> = (0..seq)
            .flat_map(|i| {
                (0..seq).map(move |j| {
                    let i_is_slot = i >= np;
                    let j_is_slot = j >= np;
                    i_is_slot && j_is_slot && i != j // slots cannot see other slots
                })
            })
            .collect();
        let mask = Tensor::<B, 3, Bool>::from_data(
            burn::tensor::TensorData::new(mask_data, [1, seq, seq]),
            &device,
        );

        let enc_input = TransformerEncoderInput::new(tokens).mask_attn(mask);
        let out = self.transformer.forward(enc_input); // [N, np+K, d_model]

        let d_model = out.dims()[2];

        // Slot outputs: positions np..np+K carry the resolved slot representations.
        // [N, K, d_model] → [N*K, d_model] → heads → reshape
        let slot_out = out.narrow(1, np, k).reshape([n * k, d_model]);
        let policy_flat = self.policy_head.forward(slot_out.clone()); // [N*K, num_passes]
        let value_flat = self.value_head.forward(slot_out); // [N*K, 1]
        let num_actions = policy_flat.dims()[1];
        let policy = policy_flat.reshape([n, k, num_actions]).unsqueeze_dim(2); // [N, K, 1, num_passes]
        let value = value_flat.reshape([n, k, 1]); // [N, K, 1]

        Output { policy, value }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        ConclaveActorConfig::new().with_max_seq_len(cfg.max_seq_len)
    }
}
