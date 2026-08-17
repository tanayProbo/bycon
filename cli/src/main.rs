//! BYCON CLI — wallet generation and node status.
use std::process;
use secp256k1::Secp256k1;
use tiny_keccak::{Hasher, Keccak};

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut k = Keccak::v256(); let mut o = [0u8; 32]; k.update(data); k.finalize(&mut o); o
}

fn help() {
    println!("BYCON CLI v{}", env!("CARGO_PKG_VERSION"));
    println!("USAGE: bycon-cli <COMMAND>");
    println!("  wallet new              Generate a new key pair");
    println!("  wallet address <key>    Derive address from hex secret key");
    println!("  node status             Print node status");
    println!("  version                 Print version");
}

fn cmd_wallet_new() {
    let secp = Secp256k1::new();
    let (sk, pk) = secp.generate_keypair(&mut rand::thread_rng());
    let raw = pk.serialize_uncompressed();
    let hash = keccak256(&raw[1..]);
    println!("New BYCON wallet generated");
    println!("  Address : 0x{}", hex::encode(&hash[12..]));
    println!("  Secret  : 0x{}", hex::encode(sk.secret_bytes()));
    println!("Keep your secret key safe!");
}

fn cmd_wallet_address(key: &str) {
    let hex = key.trim_start_matches("0x");
    let bytes = hex::decode(hex).unwrap_or_else(|e| { eprintln!("error: {}", e); process::exit(1); });
    if bytes.len() != 32 { eprintln!("error: key must be 32 bytes"); process::exit(1); }
    let arr: [u8; 32] = bytes.try_into().unwrap();
    use secp256k1::{SecretKey, PublicKey};
    let sk = SecretKey::from_slice(&arr).unwrap_or_else(|e| { eprintln!("error: {}", e); process::exit(1); });
    let secp = Secp256k1::new();
    let pk = PublicKey::from_secret_key(&secp, &sk);
    let hash = keccak256(&pk.serialize_uncompressed()[1..]);
    println!("  Address : 0x{}", hex::encode(&hash[12..]));
}

fn cmd_node_status() {
    println!("BYCON Node Status");
    println!("  Chain : BYCON mainnet (chain-id 1)");
    println!("  Sync  : idle");
    println!("  Peers : 0");
    println!("Run `bycon-node` to start the full node.");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("version") => println!("bycon-cli {}", env!("CARGO_PKG_VERSION")),
        Some("help") | Some("--help") | Some("-h") | None => help(),
        Some("node") => match args.get(2).map(String::as_str) {
            Some("status") => cmd_node_status(),
            _ => { eprintln!("usage: bycon-cli node status"); process::exit(1); }
        },
        Some("wallet") => match args.get(2).map(String::as_str) {
            Some("new") => cmd_wallet_new(),
            Some("address") => {
                let k = args.get(3).unwrap_or_else(|| { eprintln!("usage: wallet address <key>"); process::exit(1); });
                cmd_wallet_address(k);
            }
            _ => { eprintln!("usage: bycon-cli wallet <new|address>"); process::exit(1); }
        },
        Some(u) => { eprintln!("unknown command: {}", u); process::exit(1); }
    }
}
