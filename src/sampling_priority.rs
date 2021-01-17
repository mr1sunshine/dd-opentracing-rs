#[derive(Debug, PartialEq)]
pub enum SamplingPriority {
    UserDrop,
    SamplerDrop,
    SamplerKeep,
    UserKeep,
}
