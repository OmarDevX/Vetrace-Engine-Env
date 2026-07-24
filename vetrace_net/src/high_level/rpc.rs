use std::net::SocketAddr;

use crate::protocol::{
    RpcAck, RpcCall, RpcDeliveryState, RpcInbox, RpcOutbox, RpcReceiveResult,
    RpcRegistration, RpcRegistry, RpcTarget, TransferMode,
};

use super::PeerId;

/// High-level configurable RPC bus for games that want Godot-style
/// `register_rpc(...).any_peer().call_remote().unreliable_ordered()` code
/// without binding `vetrace_net` to any game-specific payload enum.
pub struct MultiplayerRpc<Payload> {
    pub registry: RpcRegistry,
    pub inbox: RpcInbox<Payload>,
    pub outbox: RpcOutbox<Payload>,
    delivery: RpcDeliveryState<Payload>,
    pub default_transfer_mode: TransferMode,
}

impl<Payload> MultiplayerRpc<Payload> {
    pub fn new(first_rpc_id: u64) -> Self {
        Self {
            registry: RpcRegistry::new(),
            inbox: RpcInbox::new(),
            outbox: RpcOutbox::new(first_rpc_id),
            delivery: RpcDeliveryState::new(first_rpc_id),
            default_transfer_mode: TransferMode::Reliable,
        }
    }

    /// Register a named RPC using the same readable style as Godot's `@rpc` options.
    pub fn register_rpc<T>(&mut self, name: impl Into<String>) -> RpcRegistration<'_, T> {
        self.registry.register_rpc::<T>(name)
    }

    /// Queue a configured named RPC.
    ///
    /// `CallLocal` is handled here: the call is immediately pushed into the
    /// local inbox as well as queued for remote peers.
    pub fn rpc_named(
        &mut self,
        name: impl Into<String>,
        from_client_id: Option<PeerId>,
        target: RpcTarget,
        payload: Payload,
    ) -> u64
    where
        Payload: Clone,
    {
        let name = name.into();
        let mut config = self.registry.config_or_default(&name);
        if !self.registry.contains(&name) {
            config.transfer_mode = self.default_transfer_mode;
        }
        let call = self.delivery.call_named(name, from_client_id, target, config, payload);
        let id = call.id;
        if call.sync == crate::protocol::RpcSync::CallLocal {
            self.inbox.push(call.clone());
        }
        self.delivery.queue_call(call);
        id
    }

    /// Backward-compatible unnamed RPC helper. Prefer `rpc_named` for new code.
    pub fn rpc(&mut self, from_client_id: Option<PeerId>, target: RpcTarget, payload: Payload) -> u64
    where
        Payload: Clone,
    {
        self.rpc_named("rpc", from_client_id, target, payload)
    }

    /// Backward-compatible reliable helper. Prefer registering the RPC as reliable.
    pub fn rpc_reliable(&mut self, from_client_id: Option<PeerId>, target: RpcTarget, payload: Payload) -> u64
    where
        Payload: Clone,
    {
        let call = crate::protocol::RpcCall::named(
            self.delivery.next_id(),
            "rpc",
            from_client_id,
            target,
            crate::protocol::RpcConfig::default().reliable(),
            payload,
        );
        let id = call.id;
        self.delivery.queue_call(call);
        id
    }

    /// Accept or reject an incoming call. Accepted calls are pushed into the inbox.
    ///
    /// `authority_peer_id` is supplied by the game/high-level entity layer because
    /// only the game knows which peer owns a particular object/action. Use `None`
    /// for server-authoritative RPCs.
    pub fn receive_rpc_call(
        &mut self,
        source_addr: SocketAddr,
        call: RpcCall<Payload>,
        authority_peer_id: Option<PeerId>,
    ) -> RpcReceiveResult
    where
        Payload: Clone,
    {
        let result = self.delivery.receive_result(source_addr, &call, &self.registry, authority_peer_id);
        if result.accepted {
            self.inbox.push(call);
        }
        result
    }

    pub fn push_incoming(&mut self, call: RpcCall<Payload>) {
        self.inbox.push(call);
    }

    pub fn drain_incoming(&mut self) -> impl Iterator<Item = RpcCall<Payload>> + '_ {
        self.inbox.drain()
    }

    /// Outgoing newly queued RPC calls. Games wrap these into their packet enum.
    pub fn drain_outgoing(&mut self) -> impl Iterator<Item = RpcCall<Payload>> + '_
    where
        Payload: Clone,
    {
        self.delivery.drain_outgoing()
    }

    /// Reliable calls that are due for resend. Games should send these again.
    pub fn reliable_resends_due(&mut self, dt: f32, resend_interval_seconds: f32) -> Vec<RpcCall<Payload>>
    where
        Payload: Clone,
    {
        self.delivery.reliable_resends_due(dt, resend_interval_seconds)
    }

    pub fn acknowledge(&mut self, ack: RpcAck) -> bool {
        self.delivery.acknowledge(ack)
    }

    pub fn pending_reliable_len(&self) -> usize {
        self.delivery.pending_reliable_len()
    }
}

impl<Payload> Default for MultiplayerRpc<Payload> {
    fn default() -> Self {
        Self::new(1)
    }
}
