//! Protocol-agnostic multiplayer message building blocks.
//!
//! Games still own their packet enum and payload structs. This module only
//! provides reusable envelopes that show up in almost every multiplayer game:
//! sequenced client input, server snapshot frames, configurable RPC calls,
//! acknowledgement/resend state, and ordered-unreliable stale packet dropping.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::marker::PhantomData;
use std::net::SocketAddr;

use serde::{Deserialize, Serialize};


include!("protocol/input_snapshot.rs");
include!("protocol/rpc_config.rs");
include!("protocol/rpc_call.rs");
include!("protocol/rpc_delivery.rs");
include!("protocol/sequence.rs");
