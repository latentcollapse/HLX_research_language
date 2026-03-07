//! Network Transport for DD Protocol — Phase 18
//!
//! Provides TCP and UDP sockets for distributed agent consensus.
//! All packets carry a logical clock timestamp for deterministic ordering.
//! UDP is used for heartbeats and discovery; TCP for state transfer and consensus.
//!
//! Design constraints (per HLX-S Axiom 1):
//! - No background threads — all I/O is non-blocking poll-based
//! - Logical clock ordering ensures determinism across nodes
//! - Network effect requires `Effect::Network` conscience verification

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::time::Duration;

/// Wire format header: [magic:4][version:1][msg_type:1][clock:8][payload_len:4]
const WIRE_MAGIC: &[u8; 4] = b"HLX\x01";
const WIRE_VERSION: u8 = 1;
const HEADER_SIZE: usize = 18; // 4 + 1 + 1 + 8 + 4

/// Message types on the wire
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WireMessageType {
    /// UDP heartbeat — "I'm alive at this clock tick"
    Heartbeat = 0,
    /// UDP discovery — "What nodes exist?"
    Discovery = 1,
    /// TCP consensus proposal — "I propose this state change"
    ConsensusPropose = 2,
    /// TCP consensus vote — "I agree/disagree"
    ConsensusVote = 3,
    /// TCP state snapshot transfer
    StateTransfer = 4,
    /// TCP DD protocol operation
    DdOperation = 5,
    /// TCP agent migration
    AgentMigrate = 6,
}

impl WireMessageType {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Heartbeat),
            1 => Some(Self::Discovery),
            2 => Some(Self::ConsensusPropose),
            3 => Some(Self::ConsensusVote),
            4 => Some(Self::StateTransfer),
            5 => Some(Self::DdOperation),
            6 => Some(Self::AgentMigrate),
            _ => None,
        }
    }
}

/// A network packet with logical clock ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub msg_type: WireMessageType,
    pub logical_clock: u64,
    pub sender: NodeId,
    pub payload: Vec<u8>,
}

/// Unique node identifier (blake3 hash of bind address + startup nonce)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

/// Peer tracking
#[derive(Debug, Clone)]
pub struct Peer {
    pub node_id: NodeId,
    pub addr: SocketAddr,
    pub last_clock: u64,
    pub last_seen_ms: u64,
}

/// Network transport for HLX distributed consensus
pub struct NetworkTransport {
    /// Our node identity
    node_id: NodeId,
    /// TCP listener for incoming connections (non-blocking)
    tcp_listener: Option<TcpListener>,
    /// UDP socket for heartbeats/discovery (non-blocking)
    udp_socket: Option<UdpSocket>,
    /// Known peers
    peers: BTreeMap<String, Peer>,
    /// Inbound message queue (ordered by logical clock)
    inbound: VecDeque<WireMessage>,
    /// Outbound message queue (reserved for batched sends)
    #[allow(dead_code)]
    outbound: VecDeque<(SocketAddr, WireMessage)>,
    /// Our logical clock (mirrors VM logical clock)
    logical_clock: u64,
    /// Bind address
    bind_addr: SocketAddr,
    /// Max inbound queue size
    max_inbound: usize,
}

impl NetworkTransport {
    /// Create a new transport bound to the given address.
    /// Does NOT start listening until `bind()` is called.
    pub fn new(bind_addr: SocketAddr) -> Self {
        let nonce = rand::random::<u64>();
        let node_hash = blake3::hash(format!("{}{}", bind_addr, nonce).as_bytes());
        let node_id = NodeId(node_hash.to_hex()[..16].to_string());

        NetworkTransport {
            node_id,
            tcp_listener: None,
            udp_socket: None,
            peers: BTreeMap::new(),
            inbound: VecDeque::new(),
            outbound: VecDeque::new(),
            logical_clock: 0,
            bind_addr,
            max_inbound: 1024,
        }
    }

    /// Bind TCP and UDP sockets (non-blocking)
    pub fn bind(&mut self) -> io::Result<()> {
        let tcp = TcpListener::bind(self.bind_addr)?;
        tcp.set_nonblocking(true)?;
        self.tcp_listener = Some(tcp);

        let udp = UdpSocket::bind(self.bind_addr)?;
        udp.set_nonblocking(true)?;
        self.udp_socket = Some(udp);

        Ok(())
    }

    /// Update the logical clock (called by VM at each cycle)
    pub fn set_clock(&mut self, clock: u64) {
        self.logical_clock = clock;
    }

    /// Our node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Number of known peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Poll for incoming messages (non-blocking).
    /// Returns number of messages received.
    pub fn poll(&mut self) -> usize {
        let mut count = 0;
        count += self.poll_udp();
        count += self.poll_tcp();
        count
    }

    /// Send a heartbeat to all known peers via UDP
    pub fn send_heartbeat(&mut self) -> io::Result<usize> {
        let msg = WireMessage {
            msg_type: WireMessageType::Heartbeat,
            logical_clock: self.logical_clock,
            sender: self.node_id.clone(),
            payload: Vec::new(),
        };

        let wire = Self::encode_wire(&msg)?;
        let mut sent = 0;

        if let Some(ref udp) = self.udp_socket {
            for peer in self.peers.values() {
                match udp.send_to(&wire, peer.addr) {
                    Ok(_) => sent += 1,
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(sent)
    }

    /// Send a discovery broadcast to a target address
    pub fn send_discovery(&mut self, target: SocketAddr) -> io::Result<()> {
        let msg = WireMessage {
            msg_type: WireMessageType::Discovery,
            logical_clock: self.logical_clock,
            sender: self.node_id.clone(),
            payload: Vec::new(),
        };

        let wire = Self::encode_wire(&msg)?;
        if let Some(ref udp) = self.udp_socket {
            udp.send_to(&wire, target)?;
        }
        Ok(())
    }

    /// Send a consensus proposal via TCP
    pub fn send_consensus_propose(
        &mut self,
        target: SocketAddr,
        proposal: &[u8],
    ) -> io::Result<()> {
        let msg = WireMessage {
            msg_type: WireMessageType::ConsensusPropose,
            logical_clock: self.logical_clock,
            sender: self.node_id.clone(),
            payload: proposal.to_vec(),
        };
        Self::tcp_send(target, &msg)
    }

    /// Send a consensus vote via TCP
    pub fn send_consensus_vote(
        &mut self,
        target: SocketAddr,
        vote: &[u8],
    ) -> io::Result<()> {
        let msg = WireMessage {
            msg_type: WireMessageType::ConsensusVote,
            logical_clock: self.logical_clock,
            sender: self.node_id.clone(),
            payload: vote.to_vec(),
        };
        Self::tcp_send(target, &msg)
    }

    /// Send a state transfer (snapshot) via TCP
    pub fn send_state_transfer(
        &mut self,
        target: SocketAddr,
        snapshot: &[u8],
    ) -> io::Result<()> {
        let msg = WireMessage {
            msg_type: WireMessageType::StateTransfer,
            logical_clock: self.logical_clock,
            sender: self.node_id.clone(),
            payload: snapshot.to_vec(),
        };
        Self::tcp_send(target, &msg)
    }

    /// Drain the inbound queue (ordered by logical clock)
    pub fn drain_inbound(&mut self) -> Vec<WireMessage> {
        let mut msgs: Vec<WireMessage> = self.inbound.drain(..).collect();
        msgs.sort_by_key(|m| m.logical_clock);
        msgs
    }

    /// Peek at inbound queue size
    pub fn inbound_count(&self) -> usize {
        self.inbound.len()
    }

    /// Add a known peer
    pub fn add_peer(&mut self, node_id: NodeId, addr: SocketAddr) {
        self.peers.insert(
            node_id.0.clone(),
            Peer {
                node_id,
                addr,
                last_clock: 0,
                last_seen_ms: 0,
            },
        );
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, node_id: &str) {
        self.peers.remove(node_id);
    }

    /// List known peers
    pub fn peers(&self) -> Vec<&Peer> {
        self.peers.values().collect()
    }

    // ---- Internal ----

    fn poll_udp(&mut self) -> usize {
        let mut count = 0;
        let mut buf = [0u8; 65536];

        if let Some(ref udp) = self.udp_socket {
            loop {
                match udp.recv_from(&mut buf) {
                    Ok((len, src)) => {
                        if let Some(msg) = Self::decode_wire(&buf[..len]) {
                            // Update peer tracking
                            if let Some(peer) = self.peers.get_mut(&msg.sender.0) {
                                peer.last_clock = msg.logical_clock;
                            } else {
                                // Auto-discover new peers
                                self.peers.insert(
                                    msg.sender.0.clone(),
                                    Peer {
                                        node_id: msg.sender.clone(),
                                        addr: src,
                                        last_clock: msg.logical_clock,
                                        last_seen_ms: 0,
                                    },
                                );
                            }

                            if self.inbound.len() < self.max_inbound {
                                self.inbound.push_back(msg);
                                count += 1;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }
        count
    }

    fn poll_tcp(&mut self) -> usize {
        let mut count = 0;

        // Accept new connections
        if let Some(ref listener) = self.tcp_listener {
            loop {
                match listener.accept() {
                    Ok((mut stream, _src)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_millis(100)))
                            .ok();
                        if let Some(msg) = Self::read_tcp_message(&mut stream) {
                            if self.inbound.len() < self.max_inbound {
                                self.inbound.push_back(msg);
                                count += 1;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }

        count
    }

    fn tcp_send(target: SocketAddr, msg: &WireMessage) -> io::Result<()> {
        let wire = Self::encode_wire(msg)?;
        let mut stream = TcpStream::connect_timeout(&target, Duration::from_secs(5))?;
        stream.write_all(&wire)?;
        stream.flush()?;
        Ok(())
    }

    fn encode_wire(msg: &WireMessage) -> io::Result<Vec<u8>> {
        let payload_json = serde_json::to_vec(msg).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e.to_string())
        })?;

        let mut buf = Vec::with_capacity(HEADER_SIZE + payload_json.len());
        buf.extend_from_slice(WIRE_MAGIC);
        buf.push(WIRE_VERSION);
        buf.push(msg.msg_type as u8);
        buf.extend_from_slice(&msg.logical_clock.to_le_bytes());
        buf.extend_from_slice(&(payload_json.len() as u32).to_le_bytes());
        buf.extend_from_slice(&payload_json);
        Ok(buf)
    }

    fn decode_wire(data: &[u8]) -> Option<WireMessage> {
        if data.len() < HEADER_SIZE {
            return None;
        }

        // Verify magic
        if &data[0..4] != WIRE_MAGIC {
            return None;
        }

        // Check version
        if data[4] != WIRE_VERSION {
            return None;
        }

        // Parse header
        let _msg_type_raw = data[5];
        let _clock = u64::from_le_bytes(data[6..14].try_into().ok()?);
        let payload_len = u32::from_le_bytes(data[14..18].try_into().ok()?) as usize;

        if data.len() < HEADER_SIZE + payload_len {
            return None;
        }

        let payload = &data[HEADER_SIZE..HEADER_SIZE + payload_len];
        serde_json::from_slice(payload).ok()
    }

    fn read_tcp_message(stream: &mut TcpStream) -> Option<WireMessage> {
        let mut header = [0u8; HEADER_SIZE];
        stream.read_exact(&mut header).ok()?;

        if &header[0..4] != WIRE_MAGIC || header[4] != WIRE_VERSION {
            return None;
        }

        let payload_len = u32::from_le_bytes(header[14..18].try_into().ok()?) as usize;

        // Sanity limit: 16MB max payload
        if payload_len > 16 * 1024 * 1024 {
            return None;
        }

        let mut payload = vec![0u8; payload_len];
        stream.read_exact(&mut payload).ok()?;

        serde_json::from_slice(&payload).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_encode_decode_roundtrip() {
        let msg = WireMessage {
            msg_type: WireMessageType::Heartbeat,
            logical_clock: 42,
            sender: NodeId("test_node".to_string()),
            payload: vec![1, 2, 3],
        };

        let encoded = NetworkTransport::encode_wire(&msg).unwrap();
        let decoded = NetworkTransport::decode_wire(&encoded).unwrap();

        assert_eq!(decoded.msg_type, WireMessageType::Heartbeat);
        assert_eq!(decoded.logical_clock, 42);
        assert_eq!(decoded.sender.0, "test_node");
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_wire_magic_validation() {
        let bad_data = b"BAD\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert!(NetworkTransport::decode_wire(bad_data).is_none());
    }

    #[test]
    fn test_wire_too_short() {
        assert!(NetworkTransport::decode_wire(&[0; 5]).is_none());
    }

    #[test]
    fn test_node_id_generation() {
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let t1 = NetworkTransport::new(addr);
        let t2 = NetworkTransport::new(addr);
        // Different nonces should produce different IDs
        assert_ne!(t1.node_id().0, t2.node_id().0);
    }

    #[test]
    fn test_peer_management() {
        let addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let mut transport = NetworkTransport::new(addr);

        assert_eq!(transport.peer_count(), 0);

        let peer_addr: SocketAddr = "127.0.0.1:9002".parse().unwrap();
        transport.add_peer(NodeId("peer1".to_string()), peer_addr);
        assert_eq!(transport.peer_count(), 1);

        transport.remove_peer("peer1");
        assert_eq!(transport.peer_count(), 0);
    }

    #[test]
    fn test_message_types() {
        for i in 0..=6 {
            assert!(WireMessageType::from_u8(i).is_some());
        }
        assert!(WireMessageType::from_u8(7).is_none());
        assert!(WireMessageType::from_u8(255).is_none());
    }
}
