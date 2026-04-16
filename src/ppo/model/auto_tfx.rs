use crate::config::Cfg;
use crate::llvm::ir::IR_CATEGORY_COUNT;
use crate::ppo::model::{AutoActor, MlpHead, MlpHeadConfig};
use burn::nn::transformer::{TransformerEncoder, TransformerEncoderConfig, TransformerEncoderInput};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Config, Int, Module};
use burn::tensor::activation::softmax;
use burn::tensor::{Tensor, TensorData};

/// Autoregressive transformer actor.
///
/// At step t the sequence is: [ir_token_t | action_emb_0 … action_emb_{t-1}]
///
/// * `ir_token_t`        — linear projection of the *current* IR histogram (updated each step).
/// * `action_emb_0..t-1` — learned embeddings for the passes chosen in previous steps.
///
/// All tokens attend freely (no causal mask needed — we only decode the *next* step).
/// Policy head reads the last token; value head reads the IR token (position 0).
#[derive(Config, Debug)]
pub(crate) struct AutoTfxConfig {
    #[config(default = 4)]
    pub(crate) ir_chunks: usize,
    #[config(default = 29)]
    pub(crate) num_actions: usize,
    #[config(default = 128)]
    pub(crate) d_model: usize,
    #[config(default = 4)]
    pub(crate) n_heads: usize,
    #[config(default = 2)]
    pub(crate) n_layers: usize,
    #[config(default = 256)]
    pub(crate) d_ff: usize,
    #[config(default = 0.1)]
    pub(crate) dropout: f64,
    #[config(default = 40)]
    pub(crate) max_seq_len: usize,
    #[config(default = 64)]
    pub(crate) head_hidden: usize,
}

#[derive(Module, Debug)]
pub(crate) struct AutoTfxActor<B: Backend> {
    ir_proj: Linear<B>,
    action_embed: Embedding<B>,
    transformer: TransformerEncoder<B>,
    policy_head: MlpHead<B>,
    value_head: MlpHead<B>,
}

impl<B: Backend<FloatElem = f32>> AutoActor<B> for AutoTfxActor<B> {
    type Config = AutoTfxConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        let ir_dim = cfg.ir_chunks * IR_CATEGORY_COUNT;
        Self {
            ir_proj: LinearConfig::new(ir_dim, cfg.d_model).init(device),
            action_embed: EmbeddingConfig::new(cfg.num_actions, cfg.d_model).init(device),
            transformer: TransformerEncoderConfig::new(
                cfg.d_model,
                cfg.d_ff,
                cfg.n_heads,
                cfg.n_layers,
            )
            .with_dropout(cfg.dropout)
            .init(device),
            policy_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, cfg.num_actions)
                .init(device),
            value_head: MlpHeadConfig::new(cfg.d_model, cfg.head_hidden, 1).init(device),
        }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        AutoTfxConfig::new()
            .with_ir_chunks(cfg.ir_chunks)
            .with_max_seq_len(cfg.max_seq_len)
    }

    fn infer_step(
        &self,
        ir_features_so_far: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Vec<f32>, f32) {
        let t = ir_features_so_far.len() - 1;
        let ir_feat = &ir_features_so_far[t];
        let feat_dim = ir_feat.len();

        let ir_feat_t = Tensor::<B, 2>::from_data(
            TensorData::new(ir_feat.clone(), [1, feat_dim]),
            device,
        );
        let ir_tok = self.ir_proj.forward(ir_feat_t).unsqueeze_dim(1); // [1, 1, d_model]

        let tokens = if taken_actions.is_empty() {
            ir_tok
        } else {
            let hist_ids = Tensor::<B, 1, Int>::from_data(
                TensorData::new(
                    taken_actions.iter().map(|&x| x as i64).collect::<Vec<_>>(),
                    [taken_actions.len()],
                ),
                device,
            )
            .unsqueeze_dim(0); // [1, t]
            let act_emb = self.action_embed.forward(hist_ids); // [1, t, d_model]
            Tensor::cat(vec![ir_tok, act_emb], 1) // [1, 1+t, d_model]
        };

        let out = self
            .transformer
            .forward(TransformerEncoderInput::new(tokens)); // [1, 1+t, d_model]
        let seq_len = out.dims()[1];
        let d_model = out.dims()[2];

        let last = out.clone().narrow(1, seq_len - 1, 1).reshape([1, d_model]);
        let first = out.narrow(1, 0, 1).reshape([1, d_model]);

        let logits_t = self.policy_head.forward(last); // [1, A]
        let value_t = self.value_head.forward(first).flatten::<1>(0, 1); // [1]

        let logits_vec: Vec<f32> = logits_t
            .flatten::<1>(0, 1)
            .into_data()
            .to_vec()
            .unwrap();
        let value_scalar: f32 = value_t.into_scalar();

        (logits_vec, value_scalar)
    }

    fn replay_episode(
        &self,
        ir_features_per_step: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Tensor<B, 2>, Tensor<B, 1>) {
        let ep_len = taken_actions.len();
        assert_eq!(ir_features_per_step.len(), ep_len);

        let mut logits_list: Vec<Tensor<B, 2>> = Vec::with_capacity(ep_len);
        let mut values_list: Vec<Tensor<B, 1>> = Vec::with_capacity(ep_len);

        for t in 0..ep_len {
            // At step t: history = taken_actions[0..t]
            let feat_dim = ir_features_per_step[t].len();
            let ir_feat_t = Tensor::<B, 2>::from_data(
                TensorData::new(ir_features_per_step[t].clone(), [1, feat_dim]),
                device,
            );
            let ir_tok = self.ir_proj.forward(ir_feat_t).unsqueeze_dim(1); // [1, 1, d]

            let tokens = if t == 0 {
                ir_tok
            } else {
                let hist_ids = Tensor::<B, 1, Int>::from_data(
                    TensorData::new(
                        taken_actions[..t]
                            .iter()
                            .map(|&x| x as i64)
                            .collect::<Vec<_>>(),
                        [t],
                    ),
                    device,
                )
                .unsqueeze_dim(0); // [1, t]
                let act_emb = self.action_embed.forward(hist_ids); // [1, t, d]
                Tensor::cat(vec![ir_tok, act_emb], 1) // [1, 1+t, d]
            };

            let out = self
                .transformer
                .forward(TransformerEncoderInput::new(tokens));
            let seq_len = out.dims()[1];
            let d = out.dims()[2];

            let last = out.clone().narrow(1, seq_len - 1, 1).reshape([1, d]);
            let first = out.narrow(1, 0, 1).reshape([1, d]);

            logits_list.push(self.policy_head.forward(last)); // [1, A]
            values_list.push(self.value_head.forward(first).flatten::<1>(0, 1)); // [1]
        }

        let logits = Tensor::cat(logits_list, 0); // [ep_len, A]
        let values = Tensor::cat(values_list, 0); // [ep_len]

        (logits, values)
    }

    // ── Batched PPO replay: K transformer calls, each over all active episodes ─

    /// At each step position t, batch together every episode that has a step t.
    /// This reduces the number of transformer invocations from N×K to K, where K is
    /// the maximum episode length in the mini-batch.
    fn replay_batch(
        &self,
        batch_ir_features: &[&[Vec<f32>]],
        batch_taken_actions: &[&[usize]],
        device: &B::Device,
    ) -> (Tensor<B, 2>, Tensor<B, 1>) {
        let n = batch_ir_features.len();
        assert_eq!(n, batch_taken_actions.len());
        assert!(n > 0);

        let ep_lens: Vec<usize> = batch_taken_actions.iter().map(|a| a.len()).collect();
        let max_ep_len = *ep_lens.iter().max().unwrap();

        // Per-episode accumulators: ep_logits[i] = list of [1, A] tensors (one per step).
        let mut ep_logits: Vec<Vec<Tensor<B, 2>>> = (0..n).map(|_| Vec::new()).collect();
        let mut ep_values: Vec<Vec<Tensor<B, 1>>> = (0..n).map(|_| Vec::new()).collect();

        for t in 0..max_ep_len {
            // Indices of episodes that have a step at position t.
            let active: Vec<usize> =
                (0..n).filter(|&i| ep_lens[i] > t).collect();
            let n_t = active.len();

            // Stack IR features for active episodes at step t.
            let feat_dim = batch_ir_features[active[0]][t].len();
            let ir_flat: Vec<f32> = active
                .iter()
                .flat_map(|&i| batch_ir_features[i][t].iter().copied())
                .collect();
            let ir_batch = Tensor::<B, 2>::from_data(
                TensorData::new(ir_flat, [n_t, feat_dim]),
                device,
            );
            let ir_tok = self.ir_proj.forward(ir_batch).unsqueeze_dim(1); // [n_t, 1, d]

            // Build token sequence [n_t, 1+t, d].
            let tokens: Tensor<B, 3> = if t == 0 {
                ir_tok
            } else {
                // Action history for active episodes: all have exactly t prev actions.
                let act_flat: Vec<i64> = active
                    .iter()
                    .flat_map(|&i| batch_taken_actions[i][..t].iter().map(|&a| a as i64))
                    .collect();
                let act_ids = Tensor::<B, 2, Int>::from_data(
                    TensorData::new(act_flat, [n_t, t]),
                    device,
                );
                let act_emb = self.action_embed.forward(act_ids); // [n_t, t, d]
                Tensor::cat(vec![ir_tok, act_emb], 1) // [n_t, 1+t, d]
            };

            let out = self
                .transformer
                .forward(TransformerEncoderInput::new(tokens)); // [n_t, 1+t, d]
            let d = out.dims()[2];

            // Policy from last token, value from IR token (position 0).
            let last = out
                .clone()
                .narrow(1, t, 1)    // [n_t, 1, d]
                .reshape([n_t, d]);
            let first = out
                .narrow(1, 0, 1)    // [n_t, 1, d]
                .reshape([n_t, d]);

            let step_logits = self.policy_head.forward(last); // [n_t, A]
            let step_values = self.value_head.forward(first).flatten::<1>(0, 1); // [n_t]

            // Route outputs back to their episodes.
            for (j, &ep_i) in active.iter().enumerate() {
                ep_logits[ep_i].push(step_logits.clone().narrow(0, j, 1)); // [1, A]
                ep_values[ep_i].push(step_values.clone().narrow(0, j, 1)); // [1]
            }
        }

        // Assemble flat episode-contiguous output.
        let mut logits_list: Vec<Tensor<B, 2>> = Vec::with_capacity(n);
        let mut values_list: Vec<Tensor<B, 1>> = Vec::with_capacity(n);
        for i in 0..n {
            logits_list.push(Tensor::cat(ep_logits[i].drain(..).collect(), 0)); // [ep_len, A]
            values_list.push(Tensor::cat(ep_values[i].drain(..).collect(), 0)); // [ep_len]
        }

        (Tensor::cat(logits_list, 0), Tensor::cat(values_list, 0))
    }
}
