// TODO: Implement LSTM policy network using Burn
//
// Architecture (from briefing):
// - Input: IR feature vector (18 features) + pass history embedding
// - LSTM layer(s) processing the sequence of (features, action) pairs
// - Output: probability distribution over 16 actions (15 passes + STOP)
//
// Key design decisions for the human:
// - Number of LSTM layers and hidden size
// - How to embed pass history (one-hot, learned embedding, etc.)
// - Whether to use attention over the sequence
// - Temperature/entropy bonus for exploration

/// LSTM-based policy network for selecting optimization passes.
#[derive(Debug)]
pub struct LstmPolicy {
    // TODO: Define layers
    // - input_projection: Linear layer mapping features to hidden size
    // - lstm: LSTM layer(s)
    // - action_head: Linear layer mapping hidden state to action logits
    _phantom: std::marker::PhantomData<()>,
}

impl LstmPolicy {
    // TODO: Implement forward pass
    // pub fn forward(&self, features: Tensor<B, 2>, hidden: LstmState) -> (Tensor<B, 2>, LstmState)

    // TODO: Implement action sampling
    // pub fn sample_action(&self, logits: Tensor<B, 2>) -> (usize, f32)
}
