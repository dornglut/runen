/// Verification-only opaque identity for one physical execution-agent fixture.
///
/// The private numeric representation carries equality and deterministic-collection
/// ordering only. It is not a Runen value, task or iteration identity, worker/lane
/// index, queue, device class, memory space, scheduling order, concurrency relation,
/// or progress token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RealizationAgentId(u32);

impl RealizationAgentId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}
