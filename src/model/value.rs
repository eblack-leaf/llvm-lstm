// TODO: Implement value network using Burn
//
// Architecture:
// - Input: same as policy network (IR features + pass history)
// - Can share LSTM backbone with policy (actor-critic) or be separate
// - Output: single scalar value estimate V(s)
//
// Key design decisions for the human:
// - Shared vs separate backbone with policy
// - Value head architecture (MLP on top of LSTM hidden state)

/// Value network for estimating state values in PPO.
#[derive(Debug)]
pub struct ValueNetwork {
    // TODO: Define layers
    // - Possibly shared LSTM backbone with policy
    // - value_head: Linear layers mapping hidden state to scalar value
    _phantom: std::marker::PhantomData<()>,
}

impl ValueNetwork {
    // TODO: Implement forward pass
    // pub fn forward(&self, features: Tensor<B, 2>, hidden: LstmState) -> (Tensor<B, 1>, LstmState)
}
