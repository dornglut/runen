use std::collections::BTreeSet;

use crate::realization::RealizationAgentId;

/// Verification-only identity token for one physical allocation fixture.
///
/// The private numeric representation carries equality only. It is not a Buffer
/// identity, numeric address, pointer provenance, source handle, memory-space
/// identifier, execution-agent identity, or ordering token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationId(u32);

impl AllocationId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Invalid lifetime/accessibility transition for the focused physical-allocation fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationError {
    Ended,
    AgentNotAccessible(RealizationAgentId),
    ActiveMappings,
    DuplicateMappingIdentity,
    UnknownMapping,
}

/// Verification-only finite evidence for one physical allocation extent.
///
/// This fixture models only whether the allocation extent still exists, which
/// represented physical execution agents may service typed mappings through it,
/// and whether typed Buffer mappings are still nested inside it. It is not an
/// allocator, memory-space taxonomy, capability database, byte store, address
/// range, relocation model, scheduler, device model, or runtime allocation object.
/// Its private accessibility and mapping-token bookkeeping intentionally expose no
/// public enumeration, debug, or fixture-equality surface.
pub struct AllocationFixture {
    id: AllocationId,
    live: bool,
    accessible_agents: BTreeSet<RealizationAgentId>,
    used_mapping_tokens: BTreeSet<u32>,
    active_mapping_tokens: BTreeSet<u32>,
}

impl AllocationFixture {
    #[must_use]
    pub fn new(
        id: AllocationId,
        accessible_agents: impl IntoIterator<Item = RealizationAgentId>,
    ) -> Self {
        Self {
            id,
            live: true,
            accessible_agents: accessible_agents.into_iter().collect(),
            used_mapping_tokens: BTreeSet::new(),
            active_mapping_tokens: BTreeSet::new(),
        }
    }

    #[must_use]
    pub const fn id(&self) -> AllocationId {
        self.id
    }

    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.live
    }

    /// Ends the represented allocation extent only when no mapping still depends on
    /// it. This is verification evidence for lifetime nesting, not an allocator API.
    pub fn end_extent(&mut self) -> Result<(), AllocationError> {
        if !self.live {
            return Err(AllocationError::Ended);
        }
        if !self.active_mapping_tokens.is_empty() {
            return Err(AllocationError::ActiveMappings);
        }

        self.live = false;
        Ok(())
    }

    pub(crate) fn begin_mapping(
        &mut self,
        token: u32,
        agent: RealizationAgentId,
    ) -> Result<(), AllocationError> {
        if !self.live {
            return Err(AllocationError::Ended);
        }
        if !self.accessible_agents.contains(&agent) {
            return Err(AllocationError::AgentNotAccessible(agent));
        }
        if !self.used_mapping_tokens.insert(token) {
            return Err(AllocationError::DuplicateMappingIdentity);
        }

        let inserted = self.active_mapping_tokens.insert(token);
        debug_assert!(inserted, "new mapping occurrence is not already active");
        Ok(())
    }

    pub(crate) fn end_mapping(&mut self, token: u32) -> Result<(), AllocationError> {
        if !self.live {
            return Err(AllocationError::Ended);
        }
        if !self.active_mapping_tokens.remove(&token) {
            return Err(AllocationError::UnknownMapping);
        }

        Ok(())
    }
}
