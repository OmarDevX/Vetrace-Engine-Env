/// Godot-like RPC permission mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcMode {
    /// Only the authority for the relevant object/action may issue the call.
    Authority,
    /// Any connected peer may issue the call.
    AnyPeer,
}

impl Default for RpcMode {
    fn default() -> Self { Self::Authority }
}

/// Godot-like local execution policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcSync {
    /// Execute only on receiving peers.
    CallRemote,
    /// Execute on receivers and also immediately on the sender.
    CallLocal,
}

impl Default for RpcSync {
    fn default() -> Self { Self::CallRemote }
}

/// Godot-like transfer mode for RPC/event envelopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferMode {
    /// Resend until acknowledged by the receiver.
    Reliable,
    /// Fire-and-forget. Good for high-rate state and cosmetic effects.
    Unreliable,
    /// Fire-and-forget, but old late packets on the same stream are discarded.
    UnreliableOrdered,
}

impl Default for TransferMode {
    fn default() -> Self { Self::Reliable }
}

/// Backward-compatible name from the earlier API.
pub type NetDelivery = TransferMode;

/// Runtime RPC configuration, deliberately independent from game payload type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RpcConfig {
    pub mode: RpcMode,
    pub sync: RpcSync,
    pub transfer_mode: TransferMode,
    pub channel: u8,
}

impl RpcConfig {
    pub fn new() -> Self { Self::default() }

    pub fn authority(mut self) -> Self {
        self.mode = RpcMode::Authority;
        self
    }

    pub fn any_peer(mut self) -> Self {
        self.mode = RpcMode::AnyPeer;
        self
    }

    pub fn call_remote(mut self) -> Self {
        self.sync = RpcSync::CallRemote;
        self
    }

    pub fn call_local(mut self) -> Self {
        self.sync = RpcSync::CallLocal;
        self
    }

    pub fn reliable(mut self) -> Self {
        self.transfer_mode = TransferMode::Reliable;
        self
    }

    pub fn unreliable(mut self) -> Self {
        self.transfer_mode = TransferMode::Unreliable;
        self
    }

    pub fn unreliable_ordered(mut self) -> Self {
        self.transfer_mode = TransferMode::UnreliableOrdered;
        self
    }

    pub fn channel(mut self, channel: u8) -> Self {
        self.channel = channel;
        self
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            mode: RpcMode::Authority,
            sync: RpcSync::CallRemote,
            transfer_mode: TransferMode::Reliable,
            channel: 0,
        }
    }
}

/// Registry for named RPCs and their Godot-like configuration.
#[derive(Clone, Debug, Default)]
pub struct RpcRegistry {
    configs: HashMap<String, RpcConfig>,
}

impl RpcRegistry {
    pub fn new() -> Self { Self::default() }

    /// Register a named RPC and return a chainable config builder.
    ///
    /// The type parameter is intentionally only documentation/ergonomics. It
    /// lets game code read naturally as:
    ///
    /// `rpc.register_rpc::<ShooterRpc>("fire_weapon").any_peer().unreliable_ordered();`
    pub fn register_rpc<Payload>(&mut self, name: impl Into<String>) -> RpcRegistration<'_, Payload> {
        let name = name.into();
        self.configs.entry(name.clone()).or_insert_with(RpcConfig::default);
        RpcRegistration { registry: self, name, _marker: PhantomData }
    }

    pub fn set_config(&mut self, name: impl Into<String>, config: RpcConfig) {
        self.configs.insert(name.into(), config);
    }

    pub fn config(&self, name: &str) -> Option<RpcConfig> {
        self.configs.get(name).copied()
    }

    pub fn config_or_default(&self, name: &str) -> RpcConfig {
        self.config(name).unwrap_or_default()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.configs.contains_key(name)
    }

    pub fn clear(&mut self) {
        self.configs.clear();
    }
}

/// Chainable RPC registration/configuration builder.
pub struct RpcRegistration<'a, Payload> {
    registry: &'a mut RpcRegistry,
    name: String,
    _marker: PhantomData<fn() -> Payload>,
}

impl<'a, Payload> RpcRegistration<'a, Payload> {
    fn update(self, edit: impl FnOnce(&mut RpcConfig)) -> Self {
        if let Some(config) = self.registry.configs.get_mut(&self.name) {
            edit(config);
        }
        self
    }

    pub fn authority(self) -> Self { self.update(|config| config.mode = RpcMode::Authority) }
    pub fn any_peer(self) -> Self { self.update(|config| config.mode = RpcMode::AnyPeer) }
    pub fn call_remote(self) -> Self { self.update(|config| config.sync = RpcSync::CallRemote) }
    pub fn call_local(self) -> Self { self.update(|config| config.sync = RpcSync::CallLocal) }
    pub fn reliable(self) -> Self { self.update(|config| config.transfer_mode = TransferMode::Reliable) }
    pub fn unreliable(self) -> Self { self.update(|config| config.transfer_mode = TransferMode::Unreliable) }
    pub fn unreliable_ordered(self) -> Self { self.update(|config| config.transfer_mode = TransferMode::UnreliableOrdered) }
    pub fn channel(self, channel: u8) -> Self { self.update(|config| config.channel = channel) }

    pub fn config(&self) -> RpcConfig {
        self.registry.config_or_default(&self.name)
    }
}

impl<'a, Payload> fmt::Debug for RpcRegistration<'a, Payload> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RpcRegistration").field("name", &self.name).field("config", &self.config()).finish()
    }
}
