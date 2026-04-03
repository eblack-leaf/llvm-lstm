use crate::config::{BurnAutoDiff, BurnDevice, Cfg};
use crate::ppo::episode::Results;
use crate::ppo::metrics::PpoLosses;
use crate::ppo::model::{ACTIONS, Actor, Input};
use indicatif::ProgressBar;
use burn::module::AutodiffModule;
use burn::optim::{GradientsParams, Optimizer};
use burn::prelude::Int;
use burn::tensor::activation::log_softmax;
use burn::tensor::{Tensor, TensorData};

pub(crate) mod advantages;
pub(crate) mod checkpoint;
pub(crate) mod episode;
pub(crate) mod logging;
pub(crate) mod metrics;
pub(crate) mod model;
pub(crate) mod returns;
pub(crate) mod step;
pub(crate) mod tokens;

/// Per-step data for the PPO update: the exact inputs seen during rollout, the action
/// taken, and the return/advantage computed afterwards.
pub(crate) struct BatchStep {
    /// 68-dim input: [base_features (34) | delta_features (34)] at state s_t.
    pub(crate) features: Vec<f32>,
    /// Action sequence fed to the model at step t: [Start, a_0, ..., a_{t-1}].
    pub(crate) action_seq: Vec<i64>,
    /// Index of the taken action in ACTIONS (used to slice log-probs).
    pub(crate) taken_action_idx: usize,
    pub(crate) old_log_prob: f32,
    pub(crate) ret: f32,
    pub(crate) advantage: f32,
}

/// Flat collection of all steps across all episodes in one epoch.
pub(crate) struct Batch {
    pub(crate) steps: Vec<BatchStep>,
}

pub(crate) struct Ppo {
    clip_epsilon: f32,
    value_coef: f32,
    entropy_coef: f32,
    ppo_epochs: usize,
}

impl Ppo {
    pub(crate) fn new(cfg: &Cfg) -> Self {
        Self {
            clip_epsilon: cfg.clip_epsilon,
            value_coef: cfg.value_coef,
            entropy_coef: cfg.entropy_coef,
            ppo_epochs: cfg.ppo_epochs,
        }
    }

    /// Flatten all episode results + pre-computed returns/advantages into a Batch.
    /// For step t, the model input is actions[0..=t] (the growing prefix) and
    /// features = [base | delta_at_t]. No padding — each step keeps its exact sequence.
    pub(crate) fn batch(
        results: &[Results],
        returns: &[Vec<f32>],
        advantages: &[Vec<f32>],
    ) -> Batch {
        let mut steps = Vec::new();
        for ((ep, ep_rets), ep_advs) in results.iter().zip(returns).zip(advantages) {
            for t in 0..ep.log_probs.len() {
                // features = base || delta at state s_t
                let features: Vec<f32> = ep
                    .base_features
                    .iter()
                    .chain(&ep.steps[t].delta_features)
                    .copied()
                    .collect();
                // actions[0] = Start prefix; actions[0..=t] is the sequence seen at step t
                let action_seq: Vec<i64> = ep.actions[0..=t].iter().map(|p| *p as i64).collect();
                // actions[t+1] is the action taken at step t
                let taken = ep.actions[t + 1];
                let taken_action_idx = ACTIONS
                    .iter()
                    .position(|&p| p == taken)
                    .expect("taken action not in ACTIONS");
                steps.push(BatchStep {
                    features,
                    action_seq,
                    taken_action_idx,
                    old_log_prob: ep.log_probs[t],
                    ret: ep_rets[t],
                    advantage: ep_advs[t],
                });
            }
        }
        Batch { steps }
    }

    /// Clipped surrogate policy loss for a single step (scalar tensor).
    /// Returns a positive value to minimise (negated PPO objective).
    fn policy_loss(
        &self,
        new_log_prob: Tensor<BurnAutoDiff, 1>,
        old_log_prob: Tensor<BurnAutoDiff, 1>,
        advantage: Tensor<BurnAutoDiff, 1>,
    ) -> Tensor<BurnAutoDiff, 1> {
        let ratio = (new_log_prob - old_log_prob).exp();
        let a = ratio.clone() * advantage.clone();
        let b = ratio.clamp(1.0 - self.clip_epsilon, 1.0 + self.clip_epsilon) * advantage;
        // -min(a, b) via the identity min(a,b) = (a+b-|a-b|)/2
        let diff = (a.clone() - b.clone()).abs();
        -((a + b - diff) / 2.0)
    }

    /// MSE value loss for a single step (scalar tensor).
    fn value_loss(
        &self,
        pred: Tensor<BurnAutoDiff, 1>,
        target: Tensor<BurnAutoDiff, 1>,
    ) -> Tensor<BurnAutoDiff, 1> {
        let d = pred - target;
        d.clone() * d
    }

    /// Entropy of the policy distribution (scalar tensor, positive = more entropy).
    fn entropy_loss(&self, logits: Tensor<BurnAutoDiff, 1>) -> Tensor<BurnAutoDiff, 1> {
        let log_p = log_softmax(logits.clone(), 0);
        let p = log_p.clone().exp();
        -(p * log_p).sum()
    }

    /// Run `ppo_epochs` gradient steps over the batch.
    /// Each epoch iterates all steps sequentially, accumulates the combined loss,
    /// then does a single backward + optimizer step.
    /// Returns the updated model, optimizer, and average losses across all ppo_epochs.
    pub(crate) fn update<A, O>(
        &self,
        mut model: A,
        mut optimizer: O,
        batch: &Batch,
        lr: f64,
        cfg: &Cfg,
        device: &BurnDevice,
        ppo_bar: &ProgressBar,
    ) -> (A, O, PpoLosses)
    where
        A: Actor<BurnAutoDiff> + AutodiffModule<BurnAutoDiff>,
        O: Optimizer<A, BurnAutoDiff>,
    {
        let mut sum_policy = 0.0_f32;
        let mut sum_value = 0.0_f32;
        let mut sum_entropy = 0.0_f32;
        let mut step_count = 0usize;

        for ep in 0..self.ppo_epochs {
            let mut total: Option<Tensor<BurnAutoDiff, 1>> = None;
            let mut epoch_loss_sum = 0.0f32;
            let mut epoch_steps = 0usize;

            for step in &batch.steps {
                let n = step.features.len();
                let s = step.action_seq.len();
                let features = Tensor::<BurnAutoDiff, 2>::from_data(
                    TensorData::new(step.features.clone(), [1, n]),
                    device,
                );
                let actions = Tensor::<BurnAutoDiff, 2, Int>::from_data(
                    TensorData::new(step.action_seq.clone(), [1, s]),
                    device,
                );
                let output = model.forward(cfg, Input { features, actions });

                // [num_actions] logits; slice the taken action's log-prob
                let logits = output.policy.flatten::<1>(0, 2);
                let new_lp = log_softmax(logits.clone(), 0).narrow(0, step.taken_action_idx, 1);
                let pred_v = output.value.flatten::<1>(0, 1);

                let old_lp = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(vec![step.old_log_prob], [1]),
                    device,
                );
                let adv = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(vec![step.advantage], [1]),
                    device,
                );
                let ret = Tensor::<BurnAutoDiff, 1>::from_data(
                    TensorData::new(vec![step.ret], [1]),
                    device,
                );

                let p = self.policy_loss(new_lp, old_lp, adv);
                let v = self.value_loss(pred_v, ret) * self.value_coef;
                let e = self.entropy_loss(logits) * self.entropy_coef;

                let p_val = p.clone().into_scalar();
                let v_val = v.clone().into_scalar();
                let e_val = e.clone().into_scalar();
                sum_policy += p_val;
                sum_value += v_val;
                sum_entropy += e_val;
                epoch_loss_sum += p_val + v_val - e_val;
                step_count += 1;
                epoch_steps += 1;

                let step_loss = p + v - e;
                total = Some(match total.take() {
                    None => step_loss,
                    Some(acc) => acc + step_loss,
                });

                ppo_bar.set_message(format!(
                    "epoch {}/{} loss={:.4}",
                    ep + 1,
                    self.ppo_epochs,
                    epoch_loss_sum / epoch_steps as f32,
                ));
                ppo_bar.inc(1);
            }

            if let Some(loss) = total {
                let grads = (loss / batch.steps.len() as f32).backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optimizer.step(lr, model, grads);
            }
        }

        let n = step_count.max(1) as f32;
        let losses = PpoLosses {
            policy_loss: sum_policy / n,
            value_loss: sum_value / n,
            entropy: sum_entropy / n,
        };
        (model, optimizer, losses)
    }
}
