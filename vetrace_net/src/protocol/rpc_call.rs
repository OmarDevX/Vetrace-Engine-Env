/// Generic RPC target. Payload meaning remains game-owned.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcTarget {
    Server,
    Client(u64),
    AllClients,
    AllExcept(u64),
    EntityOwner(u64),
    EntityObservers(u64),
}

impl Default for RpcTarget {
    fn default() -> Self { Self::Server }
}

/// Generic RPC call envelope.
///
/// Games define the `Payload` enum, for example `InventoryRpc` or `ChatRpc`.
/// `vetrace_net` only handles addressing, identity, sequence, transfer mode,
/// channel, and permission metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcCall<Payload> {
    pub id: u64,
    pub name: String,
    pub from_client_id: Option<u64>,
    pub target: RpcTarget,
    pub mode: RpcMode,
    pub sync: RpcSync,
    pub transfer_mode: TransferMode,
    pub channel: u8,
    pub payload: Payload,
}

impl<Payload> RpcCall<Payload> {
    pub fn new(id: u64, from_client_id: Option<u64>, target: RpcTarget, payload: Payload) -> Self {
        Self::named(id, "rpc", from_client_id, target, RpcConfig::default(), payload)
    }

    pub fn named(
        id: u64,
        name: impl Into<String>,
        from_client_id: Option<u64>,
        target: RpcTarget,
        config: RpcConfig,
        payload: Payload,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            from_client_id,
            target,
            mode: config.mode,
            sync: config.sync,
            transfer_mode: config.transfer_mode,
            channel: config.channel,
            payload,
        }
    }

    pub fn with_delivery(mut self, delivery: NetDelivery) -> Self {
        self.transfer_mode = delivery;
        self
    }

    pub fn with_transfer_mode(mut self, transfer_mode: TransferMode) -> Self {
        self.transfer_mode = transfer_mode;
        self
    }

    pub fn with_channel(mut self, channel: u8) -> Self {
        self.channel = channel;
        self
    }

    pub fn map<Q>(self, map: impl FnOnce(Payload) -> Q) -> RpcCall<Q> {
        RpcCall {
            id: self.id,
            name: self.name,
            from_client_id: self.from_client_id,
            target: self.target,
            mode: self.mode,
            sync: self.sync,
            transfer_mode: self.transfer_mode,
            channel: self.channel,
            payload: map(self.payload),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RpcAck {
    pub id: u64,
    pub channel: u8,
}

impl RpcAck {
    pub fn new(id: u64, channel: u8) -> Self { Self { id, channel } }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RpcRejectReason {
    UnknownRpc(String),
    NotAuthority { expected_authority: Option<u64>, caller: Option<u64> },
    DuplicateReliable,
    StaleUnreliableOrdered { last_seen_id: u64, incoming_id: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcReceiveResult {
    pub accepted: bool,
    pub ack: Option<RpcAck>,
    pub reason: Option<RpcRejectReason>,
}

impl RpcReceiveResult {
    pub fn accepted(ack: Option<RpcAck>) -> Self {
        Self { accepted: true, ack, reason: None }
    }

    pub fn rejected(reason: RpcRejectReason, ack: Option<RpcAck>) -> Self {
        Self { accepted: false, ack, reason: Some(reason) }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RpcInbox<Payload> {
    calls: VecDeque<RpcCall<Payload>>,
}

impl<Payload> RpcInbox<Payload> {
    pub fn new() -> Self { Self { calls: VecDeque::new() } }

    pub fn push(&mut self, call: RpcCall<Payload>) {
        self.calls.push_back(call);
    }

    pub fn pop_front(&mut self) -> Option<RpcCall<Payload>> {
        self.calls.pop_front()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = RpcCall<Payload>> + '_ {
        self.calls.drain(..)
    }

    pub fn len(&self) -> usize { self.calls.len() }
    pub fn is_empty(&self) -> bool { self.calls.is_empty() }
}

#[derive(Clone, Debug)]
pub struct RpcOutbox<Payload> {
    sequence: NetSequence,
    calls: Vec<RpcCall<Payload>>,
}

impl<Payload> RpcOutbox<Payload> {
    pub fn new(first_id: u64) -> Self {
        Self { sequence: NetSequence::new(first_id), calls: Vec::new() }
    }

    pub fn call(&mut self, from_client_id: Option<u64>, target: RpcTarget, payload: Payload) -> u64 {
        let id = self.sequence.next();
        self.calls.push(RpcCall::new(id, from_client_id, target, payload));
        id
    }

    pub fn call_with_delivery(
        &mut self,
        from_client_id: Option<u64>,
        target: RpcTarget,
        delivery: NetDelivery,
        payload: Payload,
    ) -> u64 {
        let id = self.sequence.next();
        self.calls.push(RpcCall::new(id, from_client_id, target, payload).with_delivery(delivery));
        id
    }

    pub fn call_named(
        &mut self,
        name: impl Into<String>,
        from_client_id: Option<u64>,
        target: RpcTarget,
        config: RpcConfig,
        payload: Payload,
    ) -> u64 {
        let id = self.sequence.next();
        self.calls.push(RpcCall::named(id, name, from_client_id, target, config, payload));
        id
    }

    pub fn drain(&mut self) -> impl Iterator<Item = RpcCall<Payload>> + '_ {
        self.calls.drain(..)
    }

    pub fn is_empty(&self) -> bool { self.calls.is_empty() }
}
