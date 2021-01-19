#[derive(Debug, PartialEq, Clone)]
pub enum SamplingPriority {
    UserDrop,
    SamplerDrop,
    SamplerKeep,
    UserKeep,
}
