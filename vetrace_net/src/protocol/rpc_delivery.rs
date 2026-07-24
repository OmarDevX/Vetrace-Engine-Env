/// Pending reliable RPC call awaiting acknowledgement.
#[derive(Clone, Debug)]
pub struct PendingReliableRpc<Payload> {
    pub call: RpcCall<Payload>,
    pub elapsed_seconds: f32,
    pub attempts: u32,
}

/// Generic RPC delivery state: outgoing queue, reliable resend tracking,
/// duplicate reliable suppression, and unreliable-ordered stale dropping.
#[derive(Clone, Debug)]
pub struct RpcDeliveryState<Payload> {
    sequence: NetSequence,
    outgoing: Vec<RpcCall<Payload>>,
    pending_reliable: Vec<PendingReliableRpc<Payload>>,
    received_reliable: HashSet<(SocketAddr, u8, u64)>,
    last_ordered: HashMap<(SocketAddr, Option<u64>, u8, String), u64>,
}

impl<Payload> RpcDeliveryState<Payload> {
    pub fn new(first_rpc_id: u64) -> Self {
        Self {
            sequence: NetSequence::new(first_rpc_id),
            outgoing: Vec::new(),
            pending_reliable: Vec::new(),
            received_reliable: HashSet::new(),
            last_ordered: HashMap::new(),
        }
    }

    pub fn next_id(&mut self) -> u64 {
        self.sequence.next()
    }

    pub fn peek_next_id(&self) -> u64 {
        self.sequence.peek()
    }

    pub fn pending_reliable_len(&self) -> usize {
        self.pending_reliable.len()
    }

    pub fn clear(&mut self) {
        self.outgoing.clear();
        self.pending_reliable.clear();
        self.received_reliable.clear();
        self.last_ordered.clear();
    }

    pub fn acknowledge(&mut self, ack: RpcAck) -> bool {
        let before = self.pending_reliable.len();
        self.pending_reliable.retain(|pending| !(pending.call.id == ack.id && pending.call.channel == ack.channel));
        before != self.pending_reliable.len()
    }

    pub fn receive_result(
        &mut self,
        source_addr: SocketAddr,
        call: &RpcCall<Payload>,
        registry: &RpcRegistry,
        authority_peer_id: Option<u64>,
    ) -> RpcReceiveResult {
        if !registry.contains(&call.name) {
            return RpcReceiveResult::rejected(RpcRejectReason::UnknownRpc(call.name.clone()), reliable_ack_for(call));
        }

        if call.mode == RpcMode::Authority && call.from_client_id != authority_peer_id {
            return RpcReceiveResult::rejected(
                RpcRejectReason::NotAuthority { expected_authority: authority_peer_id, caller: call.from_client_id },
                reliable_ack_for(call),
            );
        }

        match call.transfer_mode {
            TransferMode::Reliable => {
                let key = (source_addr, call.channel, call.id);
                if !self.received_reliable.insert(key) {
                    return RpcReceiveResult::rejected(RpcRejectReason::DuplicateReliable, Some(RpcAck::new(call.id, call.channel)));
                }
                RpcReceiveResult::accepted(Some(RpcAck::new(call.id, call.channel)))
            }
            TransferMode::Unreliable => RpcReceiveResult::accepted(None),
            TransferMode::UnreliableOrdered => {
                let key = (source_addr, call.from_client_id, call.channel, call.name.clone());
                let last_seen = self.last_ordered.get(&key).copied().unwrap_or(0);
                if call.id <= last_seen {
                    RpcReceiveResult::rejected(
                        RpcRejectReason::StaleUnreliableOrdered { last_seen_id: last_seen, incoming_id: call.id },
                        None,
                    )
                } else {
                    self.last_ordered.insert(key, call.id);
                    RpcReceiveResult::accepted(None)
                }
            }
        }
    }
}

impl<Payload> RpcDeliveryState<Payload>
where
    Payload: Clone,
{
    pub fn queue_call(&mut self, call: RpcCall<Payload>) -> u64 {
        let id = call.id;
        if call.transfer_mode == TransferMode::Reliable {
            self.pending_reliable.push(PendingReliableRpc { call: call.clone(), elapsed_seconds: 0.0, attempts: 0 });
        }
        self.outgoing.push(call);
        id
    }

    pub fn call_named(
        &mut self,
        name: impl Into<String>,
        from_client_id: Option<u64>,
        target: RpcTarget,
        config: RpcConfig,
        payload: Payload,
    ) -> RpcCall<Payload> {
        RpcCall::named(self.next_id(), name, from_client_id, target, config, payload)
    }

    pub fn drain_outgoing(&mut self) -> impl Iterator<Item = RpcCall<Payload>> + '_ {
        self.outgoing.drain(..)
    }

    pub fn reliable_resends_due(&mut self, dt: f32, resend_interval_seconds: f32) -> Vec<RpcCall<Payload>> {
        let mut due = Vec::new();
        let interval = resend_interval_seconds.max(0.001);
        for pending in &mut self.pending_reliable {
            pending.elapsed_seconds += dt.max(0.0);
            if pending.elapsed_seconds >= interval {
                pending.elapsed_seconds = 0.0;
                pending.attempts = pending.attempts.saturating_add(1);
                due.push(pending.call.clone());
            }
        }
        due
    }
}

impl<Payload> Default for RpcDeliveryState<Payload> {
    fn default() -> Self {
        Self::new(1)
    }
}

fn reliable_ack_for<Payload>(call: &RpcCall<Payload>) -> Option<RpcAck> {
    (call.transfer_mode == TransferMode::Reliable).then(|| RpcAck::new(call.id, call.channel))
}
