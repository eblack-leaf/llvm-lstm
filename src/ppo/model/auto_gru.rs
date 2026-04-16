use crate::config::Cfg;
use crate::llvm::ir::IR_CATEGORY_COUNT;
use crate::ppo::model::{AutoActor, MlpHead, MlpHeadConfig};
use burn::nn::gru::{Gru, GruConfig};
use burn::nn::{Embedding, EmbeddingConfig, Linear, LinearConfig};
use burn::prelude::{Backend, Config, Int, Module};
use burn::tensor::{Tensor, TensorData};

/// "Previous action" token used at step 0. Reuses Stop (index 28) slot.
pub(crate) const GRU_START_IDX: usize = 28;

/// Autoregressive GRU actor.
///
/// Input at step t:  `ir_proj(ir_feat_t) + action_embed(prev_action_t)`
/// Initial hidden:   `h_0 = tanh(ir_init(ir_feat_0))`
///
/// Collection: `infer_step_stateful` threads the GRU hidden state → O(K) per episode.
/// PPO update: `replay_batch` runs one batched `[N, max_len, d_model]` GRU call → O(N·K).
#[derive(Config, Debug)]
pub(crate) struct AutoGruConfig {
    #[config(default = 4)]
    pub(crate) ir_chunks: usize,
    #[config(default = 29)]
    pub(crate) num_actions: usize,
    #[config(default = 256)]
    pub(crate) d_hidden: usize,
    #[config(default = 128)]
    pub(crate) d_model: usize,
    #[config(default = 40)]
    pub(crate) max_seq_len: usize,
    #[config(default = 128)]
    pub(crate) head_hidden: usize,
}

#[derive(Module, Debug)]
pub(crate) struct AutoGruActor<B: Backend> {
    ir_init: Linear<B>,
    ir_proj: Linear<B>,
    action_embed: Embedding<B>,
    gru: Gru<B>,
    policy_head: MlpHead<B>,
    value_head: MlpHead<B>,
}

impl<B: Backend<FloatElem = f32>> AutoActor<B> for AutoGruActor<B> {
    type Config = AutoGruConfig;

    fn init(cfg: Self::Config, device: &B::Device) -> Self {
        let ir_dim = cfg.ir_chunks * IR_CATEGORY_COUNT;
        Self {
            ir_init: LinearConfig::new(ir_dim, cfg.d_hidden).init(device),
            ir_proj: LinearConfig::new(ir_dim, cfg.d_model).init(device),
            action_embed: EmbeddingConfig::new(cfg.num_actions, cfg.d_model).init(device),
            gru: GruConfig::new(cfg.d_model, cfg.d_hidden, true).init(device),
            policy_head: MlpHeadConfig::new(cfg.d_hidden, cfg.head_hidden, cfg.num_actions)
                .init(device),
            value_head: MlpHeadConfig::new(cfg.d_hidden, cfg.head_hidden, 1).init(device),
        }
    }

    fn cfg(cfg: &Cfg) -> Self::Config {
        AutoGruConfig::new()
            .with_ir_chunks(cfg.ir_chunks)
            .with_max_seq_len(cfg.max_seq_len)
    }

    // ── O(1) stateful step — overrides the default O(K²) fallback ────────────

    fn infer_step_stateful(
        &self,
        ir_features_so_far: &[Vec<f32>],
        taken_actions: &[usize],
        hidden: Option<Vec<f32>>,
        device: &B::Device,
    ) -> (Vec<f32>, f32, Option<Vec<f32>>) {
        let t = ir_features_so_far.len() - 1;
        let ir_feat = &ir_features_so_far[t];
        let feat_dim = ir_feat.len();

        let ir_t = Tensor::<B, 2>::from_data(
            TensorData::new(ir_feat.clone(), [1, feat_dim]),
            device,
        );

        // h_prev from the threaded vec, or h_0 computed from the current IR at step 0.
        let h_prev: Tensor<B, 2> = match hidden {
            Some(h) => {
                let d_h = h.len();
                Tensor::<B, 2>::from_data(TensorData::new(h, [1, d_h]), device)
            }
            None => self.ir_init.forward(ir_t.clone()).tanh(),
        };

        let prev_action = taken_actions.last().copied().unwrap_or(GRU_START_IDX);

        let ir_p = self.ir_proj.forward(ir_t); // [1, d_model]
        let act_id = Tensor::<B, 1, Int>::from_data(
            TensorData::new(vec![prev_action as i64], [1]),
            device,
        )
        .unsqueeze_dim(0); // [1, 1]
        let act_e = self.action_embed.forward(act_id).squeeze_dim(1); // [1, d_model]
        let input = (ir_p + act_e).unsqueeze_dim(1); // [1, 1, d_model]

        let gru_out = self.gru.forward(input, Some(h_prev)); // [1, 1, d_hidden]
        let h_new = gru_out.squeeze_dim(1); // [1, d_hidden]

        let logits = self.policy_head.forward(h_new.clone()); // [1, A]
        let value = self.value_head.forward(h_new.clone()).flatten::<1>(0, 1); // [1]

        let logits_vec: Vec<f32> = logits.flatten::<1>(0, 1).into_data().to_vec().unwrap();
        let value_scalar: f32 = value.into_scalar();
        let h_new_vec: Vec<f32> = h_new.into_data().to_vec().unwrap();

        (logits_vec, value_scalar, Some(h_new_vec))
    }

    // ── Fallback single-step inference (replays from start — O(K²)) ──────────

    fn infer_step(
        &self,
        ir_features_so_far: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Vec<f32>, f32) {
        let t = ir_features_so_far.len() - 1;
        let mut replay_actions = taken_actions.to_vec();
        replay_actions.push(0); // dummy for the current step

        let (logits_all, values_all) =
            self.replay_episode(ir_features_so_far, &replay_actions, device);

        let logits_t: Vec<f32> = logits_all
            .narrow(0, t, 1)
            .flatten::<1>(0, 1)
            .into_data()
            .to_vec()
            .unwrap();
        let value_t: f32 = values_all.narrow(0, t, 1).into_scalar();
        (logits_t, value_t)
    }

    // ── Single-episode GRU forward ────────────────────────────────────────────

    fn replay_episode(
        &self,
        ir_features_per_step: &[Vec<f32>],
        taken_actions: &[usize],
        device: &B::Device,
    ) -> (Tensor<B, 2>, Tensor<B, 1>) {
        let ep_len = taken_actions.len();
        assert_eq!(ir_features_per_step.len(), ep_len);
        let ir_dim = ir_features_per_step[0].len();

        let h0 = {
            let ir0 = Tensor::<B, 2>::from_data(
                TensorData::new(ir_features_per_step[0].clone(), [1, ir_dim]),
                device,
            );
            self.ir_init.forward(ir0).tanh() // [1, d_hidden]
        };

        let mut ir_flat: Vec<f32> = Vec::with_capacity(ep_len * ir_dim);
        for feat in ir_features_per_step {
            ir_flat.extend_from_slice(feat);
        }
        let ir_all =
            Tensor::<B, 2>::from_data(TensorData::new(ir_flat, [ep_len, ir_dim]), device);
        let ir_proj = self.ir_proj.forward(ir_all); // [ep_len, d_model]

        let mut prev_ids: Vec<i64> = Vec::with_capacity(ep_len);
        prev_ids.push(GRU_START_IDX as i64);
        for &a in &taken_actions[..ep_len - 1] {
            prev_ids.push(a as i64);
        }
        let act_emb = self
            .action_embed
            .forward(
                Tensor::<B, 1, Int>::from_data(TensorData::new(prev_ids, [ep_len]), device)
                    .unsqueeze_dim(0),
            )
            .squeeze_dim(0); // [ep_len, d_model]

        let gru_out = self.gru.forward((ir_proj + act_emb).unsqueeze_dim(0), Some(h0));
        // [1, ep_len, d_hidden]
        let out = gru_out.squeeze_dim(0); // [ep_len, d_hidden]

        let logits = self.policy_head.forward(out.clone()); // [ep_len, A]
        let values = self.value_head.forward(out).flatten::<1>(0, 1); // [ep_len]
        (logits, values)
    }

    // ── Batched PPO replay: one GRU call for the whole mini-batch ────────────

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
        let ir_dim = batch_ir_features[0][0].len();

        // ── h_0: [n, d_hidden] ────────────────────────────────────────────────
        let mut h0_ir: Vec<f32> = Vec::with_capacity(n * ir_dim);
        for i in 0..n {
            h0_ir.extend_from_slice(&batch_ir_features[i][0]);
        }
        let h0 = self
            .ir_init
            .forward(Tensor::<B, 2>::from_data(
                TensorData::new(h0_ir, [n, ir_dim]),
                device,
            ))
            .tanh(); // [n, d_hidden]

        // ── Padded IR features: [n*max_ep_len, ir_dim] ───────────────────────
        let mut ir_flat: Vec<f32> = vec![0.0; n * max_ep_len * ir_dim];
        for i in 0..n {
            for t in 0..ep_lens[i] {
                let dst = (i * max_ep_len + t) * ir_dim;
                ir_flat[dst..dst + ir_dim].copy_from_slice(&batch_ir_features[i][t]);
            }
        }
        let ir_proj =
            self.ir_proj
                .forward(Tensor::<B, 2>::from_data(
                    TensorData::new(ir_flat, [n * max_ep_len, ir_dim]),
                    device,
                )); // [n*max_ep_len, d_model]
        let d_model = ir_proj.dims()[1];

        // ── Padded prev-action ids: [n, max_ep_len] ───────────────────────────
        let mut prev_ids: Vec<i64> = vec![0; n * max_ep_len];
        for i in 0..n {
            for t in 0..ep_lens[i] {
                prev_ids[i * max_ep_len + t] = if t == 0 {
                    GRU_START_IDX as i64
                } else {
                    batch_taken_actions[i][t - 1] as i64
                };
            }
        }
        let act_emb = self
            .action_embed
            .forward(Tensor::<B, 2, Int>::from_data(
                TensorData::new(prev_ids, [n, max_ep_len]),
                device,
            ))
            .reshape([n * max_ep_len, d_model]); // [n*max_ep_len, d_model]

        // ── Single batched GRU call ───────────────────────────────────────────
        let gru_input = (ir_proj + act_emb).reshape([n, max_ep_len, d_model]);
        let gru_out = self.gru.forward(gru_input, Some(h0)); // [n, max_ep_len, d_hidden]
        let d_hidden = gru_out.dims()[2];

        let out_flat = gru_out.reshape([n * max_ep_len, d_hidden]);
        let logits_all = self.policy_head.forward(out_flat.clone()); // [n*max_ep_len, A]
        let values_all = self.value_head.forward(out_flat).flatten::<1>(0, 1); // [n*max_ep_len]
        let num_a = logits_all.dims()[1];

        let logits_3d = logits_all.reshape([n, max_ep_len, num_a]); // [n, max_ep_len, A]
        let values_2d = values_all.reshape([n, max_ep_len]); // [n, max_ep_len]

        // ── Slice valid steps → flat episode-contiguous output ────────────────
        let mut logits_list: Vec<Tensor<B, 2>> = Vec::with_capacity(n);
        let mut values_list: Vec<Tensor<B, 1>> = Vec::with_capacity(n);
        for i in 0..n {
            let ep_len = ep_lens[i];
            logits_list.push(logits_3d.clone().narrow(0, i, 1).narrow(1, 0, ep_len).squeeze_dim(0));
            values_list.push(values_2d.clone().narrow(0, i, 1).narrow(1, 0, ep_len).flatten::<1>(0, 1));
        }

        (Tensor::cat(logits_list, 0), Tensor::cat(values_list, 0))
    }
}
