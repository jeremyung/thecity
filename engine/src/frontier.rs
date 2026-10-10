/// Read-only result of the bounded frontier search. The caller commits the
/// returned cursor and RNG state exactly once, after choosing whether to admit
/// this proposal or try another already-enumerated planner.
#[derive(Clone, Debug)]
pub(crate) struct FrontierScan {
    pub proposal: Option<FrontierProposal>,
    pub next_cursor: usize,
    pub next_rng: u64,
    /// Mirrors the existing planner: an empty origin set returns without
    /// trying interior access, while an exhausted nonempty search permits it.
    pub allow_access_fallback: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct FrontierProposal {
    pub start_id: u32,
    pub target_id: Option<u32>,
    pub end_x: f32,
    pub end_z: f32,
    pub bridgehead: bool,
}
