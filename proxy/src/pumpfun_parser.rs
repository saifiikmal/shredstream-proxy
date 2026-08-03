use log::{debug, info, warn};
use solana_entry::entry::Entry;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, LazyLock, RwLock};

const PUMP_BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
const PUMP_SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
const PUMP_EXACT_IN_DISCRIMINATOR: [u8; 8] = [56, 252, 116, 8, 158, 223, 205, 95];
const PUMP_CREATE_V2_DISCRIMINATOR: [u8; 8] = [214, 144, 76, 236, 95, 139, 49, 180];
const PUMP_CREATE_DISCRIMINATOR: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
const PUMP_MIGRATE_DISCRIMINATOR: [u8; 8] = [155, 234, 231, 146, 236, 158, 162, 30];
const PUMP_MIGRATE_V2_DISCRIMINATOR: [u8; 8] = [187, 203, 18, 31, 206, 237, 254, 41];
const PUMP_SWAP_BUY_EXACT_QUOTE_IN_DISCRIMINATOR: [u8; 8] = [198, 46, 21, 82, 180, 217, 232, 112];
const PUMP_SWAP_DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
const PUMP_SWAP_WITHDRAW_DISCRIMINATOR: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
const PUMP_EXACT_QUOTE_IN_DISCRIMINATOR: [u8; 8] = [194, 171, 28, 70, 104, 77, 91, 47];
const PUMP_BUY_V2_DISCRIMINATOR: [u8; 8] = [184, 23, 238, 97, 103, 197, 211, 61];
const PUMP_SELL_V2_DISCRIMINATOR: [u8; 8] = [93, 246, 130, 60, 231, 233, 64, 178];
const TERMINAL_BUY_DISCRIMINATOR: [u8; 8] =
    [160, 187, 227, 151, 210, 5, 34, 85];

const TERMINAL_SELL_DISCRIMINATOR: [u8; 8] =
    [154, 231, 130, 238, 7, 46, 19, 115];

const PUMPFUN_PROGRAM_ID_STR: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const PUMP_SWAP_PROGRAM_ID_STR: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
const AXIOM_PROGRAM_ID_STR: &str = "FLASHX8DrLbgeR8FcfNV1F5krxYcYMUdBkrP1EPBtxB9";
const AXIOM_ALT_STR: &str = "7RKtfATWCe98ChuwecNq8XCzAzfoK3DtZTprFsPMGtio";
const TOKEN_PROGRAM_ID_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM_ID_STR: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const GMGN_PROGRAM_ID_STR: &str = "GMgnVFR8Jb39LoXsEVzb3DvBy3ywCmdmJquHUy1Lrkqb";
const TERMINAL_PROGRAM_ID_STR: &str = "term9YPb9mzAsABaqN71A4xdbxHmpBNZavpBiQKZzN3";

pub static PUMPFUN_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| PUMPFUN_PROGRAM_ID_STR.parse().unwrap());
pub static PUMP_SWAP_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| PUMP_SWAP_PROGRAM_ID_STR.parse().unwrap());
pub static AXIOM_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| AXIOM_PROGRAM_ID_STR.parse().unwrap());
pub static TOKEN_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| TOKEN_PROGRAM_ID_STR.parse().unwrap());
pub static TOKEN_2022_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| TOKEN_2022_PROGRAM_ID_STR.parse().unwrap());
static AXIOM_ALT: LazyLock<Pubkey> = LazyLock::new(|| AXIOM_ALT_STR.parse().unwrap());
pub static GMGN_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| GMGN_PROGRAM_ID_STR.parse().unwrap());
pub static TERMINAL_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| TERMINAL_PROGRAM_ID_STR.parse().unwrap());

// Cache for known lookup tables (ALT address -> list of resolved addresses)
static KNOWN_ALT_CACHE: LazyLock<Arc<RwLock<HashMap<Pubkey, Vec<Pubkey>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

pub fn init_lookup_tables(rpc_url: &str) {
    info!("Initializing lookup tables with RPC: {}", rpc_url);

    let mut cache = KNOWN_ALT_CACHE.write().unwrap();

    // Fetch known ALT from RPC
    let alt_addresses = vec![*AXIOM_ALT];

    for alt in alt_addresses {
        match fetch_address_lookup_table(rpc_url, alt) {
            Ok(addresses) => {
                info!("Fetched ALT {} with {} addresses", alt, addresses.len());
                cache.insert(alt, addresses);
            }
            Err(e) => {
                warn!("Failed to fetch ALT {}: {}", alt, e);
            }
        }
    }

    info!(
        "Lookup table initialization complete, {} ALTs loaded",
        cache.len()
    );
}

#[allow(dead_code)]
fn fetch_address_lookup_table(rpc_url: &str, alt: Pubkey) -> Result<Vec<Pubkey>, String> {
    let client = reqwest::blocking::Client::new();

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [
            alt.to_string(),
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json: serde_json::Value = response.json().map_err(|e| e.to_string())?;

    if let Some(error) = json.get("error") {
        return Err(format!("RPC error: {}", error));
    }

    // Parse jsonParsed format
    let addresses = json["result"]["value"]["data"]["parsed"]["info"]["addresses"]
        .as_array()
        .ok_or("No addresses in response")?;

    let mut pubkeys = Vec::with_capacity(addresses.len());
    for addr in addresses {
        let addr_str = addr.as_str().ok_or("Invalid address")?;
        let pubkey = addr_str.parse::<Pubkey>().map_err(|e| e.to_string())?;
        pubkeys.push(pubkey);
    }

    Ok(pubkeys)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeType {
    Unknown = 0,
    PumpfunBuy = 1,
    PumpfunSell = 2,
    PumpfunBuyExactIn = 3,
    AxiomBuy = 4,
    AxiomSell = 5,
    PumpfunCreate = 6,
    PumpSwapBuy = 9,
    PumpSwapSell = 10,
    PumpSwapCreatePool = 11,
    PumpSwapBuyExactIn = 12,
    PumpSwapCreate = 13,
}

impl From<TradeType> for i32 {
    fn from(t: TradeType) -> Self {
        t as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Unspecified = 0,
    Pumpfun = 1,
    Axiom = 2,
    PumpSwap = 3,
}

impl From<Origin> for i32 {
    fn from(o: Origin) -> Self {
        o as i32
    }
}

#[derive(Debug, Clone)]
pub struct ParsedTransaction {
    pub slot: u64,
    pub signature: String,
    pub mint: String,
    pub signer: String,
    pub trade_type: TradeType,
    pub origin: Origin,
    pub token_amount: u64,
    pub sol_amount: u64,
    pub timestamp: u64,
    pub pool: Option<String>,
}

pub struct PumpFunParser;

impl PumpFunParser {
    pub fn parse_entries(entries_bytes: &[u8], filter: &str) -> Vec<ParsedTransaction> {
        let entries: Vec<Entry> = match bincode::deserialize(entries_bytes) {
            Ok(e) => e,
            Err(e) => {
                debug!("Failed to deserialize entries: {}", e);
                return vec![];
            }
        };

        let mut results = vec![];

        for entry in entries {
            for tx in entry.transactions {
                if let Some(parsed) = Self::parse_transaction(&tx, filter) {
                    results.push(parsed);
                }
            }
        }

        results
    }

    fn parse_transaction(
        transaction: &VersionedTransaction,
        filter: &str,
    ) -> Option<ParsedTransaction> {
        // Start with static account keys
        let mut account_keys: Vec<Pubkey> = transaction.message.static_account_keys().to_vec();

        // If there are address table lookups and they're in cache, resolve them
        if let Some(lookups) = transaction.message.address_table_lookups() {
            if !lookups.is_empty() {
                let cache = KNOWN_ALT_CACHE.read().unwrap();
                for lookup in lookups {
                    if let Some(alt_addresses) = cache.get(&lookup.account_key) {
                        // Append ALT addresses in loaded-message order:
                        // writable indexes first, then readonly indexes
                        for idx in &lookup.writable_indexes {
                            if let Some(addr) = alt_addresses.get(*idx as usize) {
                                account_keys.push(*addr);
                            }
                        }
                        for idx in &lookup.readonly_indexes {
                            if let Some(addr) = alt_addresses.get(*idx as usize) {
                                account_keys.push(*addr);
                            }
                        }
                    }
                    // If ALT not in cache, we just use static keys (may cause out of bounds later)
                }
            }
        }

        let signature = transaction.signatures.get(0)?.to_string();

        for instruction in transaction.message.instructions() {
            if instruction.data.len() < 8 {
                continue;
            }

            let program_id = match account_keys.get(instruction.program_id_index as usize) {
                Some(p) => *p,
                None => return None,
            };
            let data = &instruction.data;

            if program_id == *PUMPFUN_PROGRAM_ID {
                let discriminator: [u8; 8] = match data[..8].try_into() {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                if discriminator == PUMP_MIGRATE_DISCRIMINATOR
                    || discriminator == PUMP_MIGRATE_V2_DISCRIMINATOR
                {
                    if filter == "axiom" || filter == "pumpfun" || filter == "pumpfun_new" {
                        continue;
                    }

                    let signer = account_keys.get(0)?.to_string();
                    let mint = Self::resolve_mint(&account_keys, instruction, 2)?;
                    // migrate (v1): pool at account index 9; migrate_v2: pool at account index 10
                    let pool_index = if discriminator == PUMP_MIGRATE_DISCRIMINATOR {
                        9
                    } else {
                        10
                    };
                    let pool = Self::resolve_mint(&account_keys, instruction, pool_index);

                    return Some(ParsedTransaction {
                        slot: 0,
                        signature,
                        mint,
                        signer,
                        trade_type: TradeType::PumpSwapCreatePool,
                        origin: Origin::PumpSwap,
                        token_amount: 0,
                        sol_amount: 0,
                        timestamp: 0,
                        pool,
                    });
                }

                if filter == "axiom" || filter == "pumpswap" || filter == "pumpswap_new" {
                    continue;
                }

                if let Some((trade_type, token_amount, sol_amount, idx_mint)) =
                    Self::parse_pumpfun_args(discriminator, data)
                {
                    if trade_type == TradeType::PumpfunCreate {
                        // For create: mint at account[0], signer at account[0]
                        let signer = account_keys.get(0)?.to_string();
                        let mint = Self::resolve_mint(&account_keys, instruction, idx_mint)?;

                        return Some(ParsedTransaction {
                            slot: 0,
                            signature,
                            mint,
                            signer,
                            trade_type,
                            origin: Origin::Pumpfun,
                            token_amount,
                            sol_amount,
                            timestamp: 0,
                            pool: None,
                        });
                    } else {
                        if filter == "pumpfun_new" {
                            continue;
                        }

                        let signer = account_keys.get(0)?.to_string();
                        let mint = Self::resolve_mint(&account_keys, instruction, idx_mint)?;

                        return Some(ParsedTransaction {
                            slot: 0,
                            signature,
                            mint,
                            signer,
                            trade_type,
                            origin: Origin::Pumpfun,
                            token_amount,
                            sol_amount,
                            timestamp: 0,
                            pool: None,
                        });
                    }
                    
                }
            }

            if program_id == *GMGN_PROGRAM_ID {
                if filter == "axiom" || filter == "pumpswap" || filter == "pumpswap_new" {
                    continue;
                }

                let discriminator: [u8; 8] = data[..8].try_into().unwrap();
                if let Some((trade_type, token_amount, sol_amount, idx_mint)) =
                    Self::parse_gmgn_args(discriminator, data)
                {
                    if trade_type == TradeType::PumpfunCreate {
                        // For create: mint at account[0], signer at account[0]
                        let signer = account_keys.get(0)?.to_string();
                        let mint = Self::resolve_mint(&account_keys, instruction, idx_mint)?;

                        return Some(ParsedTransaction {
                            slot: 0,
                            signature,
                            mint,
                            signer,
                            trade_type,
                            origin: Origin::Pumpfun,
                            token_amount,
                            sol_amount,
                            timestamp: 0,
                            pool: None,
                        });
                    } else {
                        if filter == "pumpfun_new" {
                            continue;
                        }

                        let signer = account_keys.get(0)?.to_string();
                        let mint = Self::resolve_mint(&account_keys, instruction, idx_mint)?;

                        return Some(ParsedTransaction {
                            slot: 0,
                            signature,
                            mint,
                            signer,
                            trade_type,
                            origin: Origin::Pumpfun,
                            token_amount,
                            sol_amount,
                            timestamp: 0,
                            pool: None,
                        });
                    }
                    
                }
            }

            if program_id == *TERMINAL_PROGRAM_ID {
                if filter == "axiom" {
                    continue;
                }
                if data.len() < 24 {
                    continue;
                }

                let discriminator: [u8; 8] =
                    data[..8].try_into().unwrap();

                let trade_type = if discriminator
                    == TERMINAL_BUY_DISCRIMINATOR
                {
                    TradeType::PumpfunBuy
                } else if discriminator
                    == TERMINAL_SELL_DISCRIMINATOR
                {
                    TradeType::PumpfunSell
                } else {
                    continue;
                };

                let value_1 =
                    u64::from_le_bytes(data[8..16].try_into().unwrap());

                let value_2 =
                    u64::from_le_bytes(data[16..24].try_into().unwrap());

                let (token_amount, sol_amount) =
                    if value_1 > value_2 {
                        (value_1, value_2)
                    } else {
                        (value_2, value_1)
                    };

                let signer = account_keys.get(0)?.to_string();

                let mint =
                    Self::resolve_mint(&account_keys, instruction, 2)?;

                return Some(ParsedTransaction {
                    slot: 0,
                    signature,
                    mint,
                    signer,
                    trade_type,
                    origin: Origin::Pumpfun,
                    token_amount,
                    sol_amount,
                    timestamp: 0,
                            pool: None,
                });
            }

            if program_id == *PUMP_SWAP_PROGRAM_ID {
                if filter == "axiom" || filter == "pumpfun" || filter == "pumpfun_new" {
                    continue;
                }

                let discriminator: [u8; 8] = match data[..8].try_into() {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                if let Some((trade_type, token_amount, sol_amount)) =
                    Self::parse_pump_swap_args(discriminator, data)
                {
                    if filter == "pumpswap_new" {
                        continue;
                    }

                    let signer = account_keys.get(0)?.to_string();
                    let mint = Self::resolve_pump_swap_mint(&account_keys, instruction, 3, 4)?;
                    let pool = Self::resolve_mint(&account_keys, instruction, 0)?;

                    return Some(ParsedTransaction {
                        slot: 0,
                        signature,
                        mint,
                        signer,
                        trade_type,
                        origin: Origin::PumpSwap,
                        token_amount,
                        sol_amount,
                        timestamp: 0,
                        pool: Some(pool),
                    });
                }
            }

            if program_id == *AXIOM_PROGRAM_ID {
                if filter == "pumpfun" || filter == "pumpfun_new" || filter == "pumpswap" || filter == "pumpswap_new" {
                    continue;
                }

                if data.len() == 22 
                    && data[0] == 0x00 
                    && (data[17] == 0x00 || data[17] == 0x01)
                {
                    let is_sell = data[17] == 0x01;
                    let amount_in = u64::from_le_bytes(data[1..9].try_into().unwrap());
                    let min_amount_out = u64::from_le_bytes(data[9..17].try_into().unwrap());

                    let signer = account_keys.get(0)?.to_string();
                    let mint = Self::resolve_mint(&account_keys, instruction, 10)?;

                    let trade_type = if is_sell {
                        TradeType::AxiomSell
                    } else {
                        TradeType::AxiomBuy
                    };

                    // let (token_amount, sol_amount) = if is_sell {
                    //     (amount_in, min_amount_out)
                    // } else {
                    //     (min_amount_out, amount_in)
                    // };
                    let (token_amount, sol_amount) =
                        if Self::looks_like_sol(amount_in)
                            && !Self::looks_like_sol(min_amount_out)
                        {
                            (min_amount_out, amount_in)
                        } else if Self::looks_like_sol(min_amount_out)
                            && !Self::looks_like_sol(amount_in)
                        {
                            (amount_in, min_amount_out)
                        } else if amount_in > min_amount_out {
                            (amount_in, min_amount_out)
                        } else {
                            (min_amount_out, amount_in)
                        };

                    return Some(ParsedTransaction {
                        slot: 0,
                        signature,
                        mint,
                        signer,
                        trade_type,
                        origin: Origin::Axiom,
                        token_amount,
                        sol_amount,
                        timestamp: 0,
                        pool: None,
                    });
                }
            }
        }

        None
    }

    fn parse_pumpfun_args(discriminator: [u8; 8], data: &[u8]) -> Option<(TradeType, u64, u64, usize)> {
        if data.len() < 24 {
            return None;
        }

        if discriminator == PUMP_BUY_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunBuy, token_amount, sol_amount, 2));
        }

        if discriminator == PUMP_BUY_V2_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunBuy, token_amount, sol_amount, 1));
        }

        if discriminator == PUMP_SELL_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunSell, token_amount, sol_amount, 2));
        }

        if discriminator == PUMP_SELL_V2_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunSell, token_amount, sol_amount, 1));
        }

        if discriminator == PUMP_EXACT_IN_DISCRIMINATOR {
            let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunBuyExactIn, token_amount, sol_amount, 2));
        }

        if discriminator == PUMP_EXACT_QUOTE_IN_DISCRIMINATOR {
            let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunBuyExactIn, token_amount, sol_amount, 1));
        }

        // if discriminator == PUMP_CREATE_DISCRIMINATOR || discriminator == PUMP_CREATE_V2_DISCRIMINATOR {
        //     return Some((TradeType::PumpfunCreate, 0, 0));
        // }

        None
    }

    fn parse_pump_swap_args(discriminator: [u8; 8], data: &[u8]) -> Option<(TradeType, u64, u64)> {
        if data.len() < 24 {
            return None;
        }

        if discriminator == PUMP_BUY_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpSwapBuy, token_amount, sol_amount));
        }

        if discriminator == PUMP_BUY_V2_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpSwapBuy, token_amount, sol_amount));
        }

        if discriminator == PUMP_SELL_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpSwapSell, token_amount, sol_amount));
        }

        if discriminator == PUMP_SELL_V2_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpSwapSell, token_amount, sol_amount));
        }

        if discriminator == PUMP_EXACT_IN_DISCRIMINATOR {
            let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpSwapBuyExactIn, token_amount, sol_amount));
        }

        if discriminator == PUMP_EXACT_QUOTE_IN_DISCRIMINATOR {
            let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpSwapBuyExactIn, token_amount, sol_amount));
        }

        None
    }

    fn parse_gmgn_args(discriminator: [u8; 8], data: &[u8]) -> Option<(TradeType, u64, u64, usize)> {
        if data.len() < 24 {
            return None;
        }

        if discriminator == PUMP_BUY_DISCRIMINATOR {
            let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunBuy, token_amount, sol_amount, 2));
        }

        if discriminator == PUMP_BUY_V2_DISCRIMINATOR {
            let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunBuy, token_amount, sol_amount, 1));
        }

        if discriminator == PUMP_SELL_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunSell, token_amount, sol_amount, 2));
        }

        if discriminator == PUMP_SELL_V2_DISCRIMINATOR {
            let token_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let sol_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
            return Some((TradeType::PumpfunSell, token_amount, sol_amount, 1));
        }

        // if discriminator == PUMP_EXACT_IN_DISCRIMINATOR {
        //     let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
        //     let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
        //     return Some((TradeType::PumpfunBuyExactIn, token_amount, sol_amount));
        // }

        // if discriminator == PUMP_EXACT_QUOTE_IN_DISCRIMINATOR {
        //     let sol_amount = u64::from_le_bytes(data[8..16].try_into().unwrap());
        //     let token_amount = u64::from_le_bytes(data[16..24].try_into().unwrap());
        //     return Some((TradeType::PumpfunBuyExactIn, token_amount, sol_amount));
        // }

        // if discriminator == PUMP_CREATE_DISCRIMINATOR || discriminator == PUMP_CREATE_V2_DISCRIMINATOR {
        //     return Some((TradeType::PumpfunCreate, 0, 0));
        // }

        None
    }

    fn resolve_mint(
        account_keys: &[Pubkey],
        instruction: &solana_sdk::instruction::CompiledInstruction,
        mint_index: usize,
    ) -> Option<String> {
        if instruction.accounts.len() > mint_index {
            let key_idx = instruction.accounts[mint_index] as usize;
            if key_idx < account_keys.len() {
                return Some(account_keys[key_idx].to_string());
            }
        }
        None
    }

    fn resolve_pump_swap_mint(
        account_keys: &[Pubkey],
        instruction: &solana_sdk::instruction::CompiledInstruction,
        index_a: usize,
        index_b: usize,
    ) -> Option<String> {
        let mint_a = Self::resolve_mint(account_keys, instruction, index_a);
        let mint_b = Self::resolve_mint(account_keys, instruction, index_b);

        let wsol = Pubkey::from_str("So11111111111111111111111111111111111111112").ok()?;

        match (mint_a, mint_b) {
            (Some(a), Some(b)) => {
                let a_is_wsol = a == wsol.to_string();
                let b_is_wsol = b == wsol.to_string();
                if a_is_wsol && !b_is_wsol {
                    Some(b)
                } else if !a_is_wsol && b_is_wsol {
                    Some(a)
                } else {
                    Some(a)
                }
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    fn looks_like_sol(v: u64) -> bool {
        v < 100_000_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real mainnet migrate_v2 tx:
    // 3a3K3AB5ESQrs71E5VtmVLREoHLedsAHAQqv3EoHmzLj5tziNh9boq32PNrJvzSt9XyUMSeNGs4agDHKpLtToiwJ
    const SAMPLE_MIGRATE_V2_TX: &[u8] = &[
        0x01, 0x80, 0x86, 0xbb, 0xe5, 0xc0, 0xb7, 0x73, 0x06, 0x48, 0x8d, 0x71,
        0xa2, 0x46, 0xb2, 0x03, 0xed, 0x16, 0xf3, 0x8a, 0xf5, 0x0f, 0x87, 0xd9,
        0x65, 0x63, 0x1b, 0x1b, 0x4c, 0xde, 0x02, 0x51, 0x18, 0x9c, 0x8d, 0x13,
        0xec, 0x1f, 0xce, 0x31, 0x02, 0x18, 0xe1, 0x79, 0x7a, 0x00, 0xfa, 0xea,
        0x03, 0x46, 0x32, 0xef, 0xdd, 0xc9, 0xf9, 0x0b, 0xc3, 0x0f, 0x31, 0x8a,
        0xf3, 0x19, 0x65, 0x2b, 0x01, 0x80, 0x01, 0x00, 0x05, 0x12, 0x08, 0xaa,
        0x1d, 0xec, 0x0b, 0x59, 0x1c, 0x19, 0xd2, 0x31, 0x68, 0xb1, 0x81, 0x84,
        0xb2, 0x6b, 0x6f, 0x8c, 0xac, 0xe5, 0x4f, 0xb1, 0xde, 0xc3, 0xbc, 0x78,
        0xb6, 0xeb, 0x1b, 0x42, 0xb5, 0xf2, 0x0a, 0x33, 0x72, 0xad, 0xae, 0xff,
        0xdd, 0xd8, 0xcc, 0xd4, 0xf0, 0xf4, 0x47, 0xe6, 0xe6, 0xdd, 0xce, 0x44,
        0xae, 0xdb, 0x86, 0x8a, 0xb3, 0x83, 0x0f, 0xa5, 0x06, 0x92, 0x0f, 0xed,
        0xd3, 0x93, 0x13, 0x40, 0x47, 0x15, 0xe7, 0xc8, 0x40, 0x59, 0x32, 0x62,
        0x42, 0xef, 0xea, 0x44, 0x57, 0xee, 0xd2, 0x9e, 0x4a, 0x25, 0x75, 0xae,
        0x44, 0xee, 0xba, 0x5b, 0xbe, 0x0e, 0xd8, 0xd4, 0x76, 0x22, 0x20, 0xdc,
        0xed, 0x2b, 0x7a, 0x3d, 0x66, 0x56, 0xf0, 0x13, 0xd8, 0x9a, 0xc5, 0x99,
        0xe6, 0x73, 0x27, 0x6c, 0xe1, 0xad, 0x3d, 0xdd, 0x79, 0x10, 0xe6, 0x9f,
        0xf9, 0x2f, 0x84, 0x72, 0x71, 0xf9, 0x4e, 0x6f, 0xe9, 0x77, 0x0f, 0x5b,
        0xab, 0x84, 0xad, 0xed, 0xe4, 0x0c, 0xf4, 0xa1, 0xc8, 0x9b, 0xd3, 0x3d,
        0x8d, 0x81, 0x99, 0xb4, 0x8e, 0xb4, 0xa5, 0xda, 0x49, 0x77, 0x53, 0xbc,
        0xb4, 0x02, 0x5a, 0x8b, 0x62, 0x30, 0x11, 0x7a, 0x89, 0xa7, 0xd5, 0xae,
        0xf7, 0xf6, 0x44, 0xae, 0x62, 0x7a, 0xe5, 0x4f, 0x29, 0x3e, 0xea, 0x2f,
        0xf8, 0x32, 0x51, 0x8f, 0x43, 0x19, 0xfd, 0xed, 0xdd, 0xc1, 0x83, 0x26,
        0x16, 0x58, 0xcb, 0xdd, 0xaf, 0x81, 0xc7, 0x95, 0x16, 0x1a, 0xee, 0xb5,
        0xf8, 0x40, 0x82, 0xb3, 0x9f, 0x40, 0xae, 0xc0, 0xff, 0xbb, 0x2a, 0x73,
        0xb7, 0x7f, 0x00, 0x1a, 0xf9, 0xef, 0x83, 0x56, 0x77, 0x08, 0x54, 0x30,
        0x23, 0xef, 0x4d, 0x43, 0x55, 0xdb, 0x53, 0xbf, 0xa9, 0xec, 0xdc, 0x8d,
        0x8c, 0xcf, 0x69, 0x9f, 0xc0, 0xa0, 0xa8, 0x7c, 0x8c, 0xaf, 0xe5, 0x04,
        0x2d, 0xa6, 0x83, 0xf8, 0x63, 0x07, 0x50, 0x66, 0x45, 0xb0, 0xd3, 0x97,
        0xc8, 0x2d, 0x9a, 0x04, 0xc0, 0xea, 0xe7, 0x3f, 0x35, 0xfa, 0xdf, 0x2a,
        0x3b, 0x74, 0xc4, 0x61, 0xa4, 0x7d, 0xec, 0x83, 0xfd, 0xb8, 0x8b, 0x7d,
        0x5f, 0x30, 0x38, 0xa8, 0xdd, 0xd4, 0xf4, 0x6d, 0xaa, 0x80, 0x4b, 0x42,
        0xc4, 0x6c, 0x0d, 0xad, 0xbd, 0x4d, 0xf6, 0xa0, 0x7c, 0x5e, 0x24, 0x15,
        0x00, 0x33, 0x74, 0x12, 0x72, 0x35, 0xa2, 0x95, 0xbe, 0xb9, 0x5f, 0x38,
        0xf2, 0xd0, 0x32, 0x8a, 0x79, 0x7e, 0x2e, 0x9e, 0x2b, 0x60, 0x85, 0xa8,
        0x4e, 0x57, 0x39, 0x2d, 0xe2, 0xa6, 0x9f, 0x64, 0x44, 0x55, 0xaa, 0x31,
        0x38, 0x77, 0xae, 0xc4, 0xd1, 0xc2, 0x52, 0x77, 0xad, 0x50, 0xb3, 0x93,
        0xec, 0xbd, 0x74, 0x3a, 0x23, 0xdb, 0xe5, 0xc4, 0xba, 0xea, 0x18, 0x5e,
        0xdb, 0xbc, 0x3e, 0xa2, 0x2a, 0x79, 0x30, 0x82, 0xed, 0x99, 0xc8, 0xdd,
        0xd9, 0xdb, 0xcf, 0xf4, 0x22, 0xd5, 0xe4, 0x30, 0x81, 0x05, 0x5d, 0xaf,
        0xbe, 0x85, 0x05, 0x06, 0x17, 0x5f, 0xf5, 0x0b, 0x0c, 0xec, 0xfc, 0x5b,
        0x90, 0xd1, 0x36, 0x24, 0xcd, 0xe4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x56, 0xe0, 0xf6, 0x93, 0x66, 0x5a, 0xcf, 0x44, 0xdb,
        0x15, 0x68, 0xbf, 0x17, 0x5b, 0xaa, 0x51, 0x89, 0xcb, 0x97, 0xf5, 0xd2,
        0xff, 0x3b, 0x65, 0x5d, 0x2b, 0xb6, 0xfd, 0x6d, 0x18, 0xb0, 0x03, 0x06,
        0x46, 0x6f, 0xe5, 0x21, 0x17, 0x32, 0xff, 0xec, 0xad, 0xba, 0x72, 0xc3,
        0x9b, 0xe7, 0xbc, 0x8c, 0xe5, 0xbb, 0xc5, 0xf7, 0x12, 0x6b, 0x2c, 0x43,
        0x9b, 0x3a, 0x40, 0x00, 0x00, 0x00, 0x2c, 0x4b, 0xa9, 0x55, 0x32, 0x3b,
        0xcc, 0x12, 0x9c, 0x9a, 0x9f, 0x3b, 0xbc, 0xbd, 0xf5, 0x89, 0x2d, 0xcf,
        0x5a, 0xce, 0xfe, 0xea, 0x96, 0x9f, 0x7f, 0xb4, 0xf8, 0x4b, 0x36, 0x29,
        0xe6, 0x6f, 0xcc, 0x4c, 0x22, 0x60, 0x40, 0x86, 0xf1, 0xf2, 0x8a, 0xa0,
        0x93, 0xe5, 0x00, 0x9a, 0x7f, 0x46, 0x22, 0x45, 0x70, 0x32, 0x91, 0xf3,
        0xdb, 0xfc, 0x9d, 0x6c, 0xdd, 0x37, 0x3b, 0x05, 0xb7, 0x85, 0x7f, 0xd9,
        0xf6, 0x82, 0x00, 0x14, 0xa2, 0x11, 0x15, 0x32, 0xe0, 0xb1, 0x6e, 0x94,
        0x19, 0x87, 0x88, 0x09, 0xe3, 0x2c, 0xa6, 0xc2, 0xc5, 0xcc, 0xbf, 0x81,
        0xd4, 0xc9, 0x26, 0xa6, 0x56, 0x90, 0x04, 0x0f, 0x00, 0x05, 0x02, 0x30,
        0x57, 0x05, 0x00, 0x0f, 0x00, 0x09, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x0e, 0x1d, 0x19, 0x13, 0x10, 0x14, 0x05, 0x02, 0x07,
        0x00, 0x0d, 0x18, 0x0a, 0x06, 0x09, 0x08, 0x1a, 0x04, 0x0c, 0x0b, 0x03,
        0x17, 0x16, 0x17, 0x1b, 0x1d, 0x15, 0x1c, 0x0e, 0x11, 0x01, 0x08, 0xbb,
        0xcb, 0x12, 0x1f, 0xce, 0xed, 0xfe, 0x29, 0x0d, 0x02, 0x00, 0x12, 0x0c,
        0x02, 0x00, 0x00, 0x00, 0x10, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x60, 0x19, 0x3a, 0xfb, 0x97, 0xd1, 0xa2, 0x7a, 0xdf, 0xb0, 0xf0,
        0x95, 0x1d, 0x74, 0xb7, 0x58, 0x8f, 0xdc, 0xf7, 0x28, 0x54, 0x36, 0x00,
        0x7f, 0x9d, 0x4d, 0xa1, 0x1d, 0x3f, 0x0e, 0xbc, 0x7c, 0x02, 0x00, 0x1b,
        0x0a, 0x0b, 0x04, 0x07, 0x08, 0x20, 0x16, 0x23, 0x05, 0x17, 0x21,
    ];

    #[test]
    fn parse_migrate_v2_sample_tx() {
        let tx: VersionedTransaction = bincode::deserialize(SAMPLE_MIGRATE_V2_TX)
            .expect("failed to deserialize sample tx");
        let parsed = PumpFunParser::parse_transaction(&tx, "pumpswap").expect("expected a create pool event");
        assert_eq!(
            parsed.mint,
            "3yurY7fQPvhgyAkcjsFUurECMYuBv7kGsGGrHGZFpump"
        );
        assert_eq!(
            parsed.pool.as_deref(),
            Some("BwfUyRJPCGT3MC8wQw6Noay32adRL4gF4ASCgzWuxQC6")
        );
        assert_eq!(parsed.signer, "apiQH2gJBFg7WnpKfwkcbLntv8fW99RjyLeYfCMdD7f");
        assert_eq!(parsed.trade_type, TradeType::PumpSwapCreatePool);
        assert_eq!(parsed.origin, Origin::PumpSwap);
    }

    // Real mainnet PumpSwap sell tx (creator-fee layout, no address table lookups):
    // 4RzufL81oGPn7EYP5ec8k51JZZXt5chLLYPSdC3Xvp1v9UhSDtM5N2gTZiUBK8Qedc6RdXWEaLmJA3VuVd5DStpM
    const SAMPLE_PUMP_SWAP_SELL_TX: &[u8] = &[
        0x01, 0xab, 0x9b, 0xd2, 0x49, 0xb0, 0xf7, 0xb6, 0xa8, 0x6f, 0x4d, 0xd1,
        0xf9, 0xfb, 0x0f, 0x91, 0xc7, 0xef, 0x3c, 0xae, 0x27, 0xc1, 0x56, 0x33,
        0x2d, 0x1a, 0x9e, 0x48, 0x4f, 0x99, 0x64, 0x72, 0xdd, 0xcf, 0xb9, 0xd8,
        0x8b, 0x4d, 0x5d, 0xdd, 0x17, 0x98, 0xb0, 0x89, 0x4b, 0x87, 0x7f, 0xbc,
        0x35, 0x60, 0x2b, 0xed, 0x14, 0x99, 0x60, 0x71, 0x43, 0x0f, 0x7a, 0x91,
        0x47, 0xcd, 0x6b, 0x9d, 0x0e, 0x80, 0x01, 0x00, 0x0e, 0x17, 0x3c, 0xf3,
        0x01, 0x60, 0x5d, 0x52, 0xfa, 0xa2, 0x1f, 0xe9, 0x0e, 0xda, 0x3b, 0x93,
        0xe6, 0xc3, 0xbf, 0xde, 0xe8, 0x0e, 0xde, 0x49, 0x95, 0x32, 0xbe, 0xa0,
        0xf8, 0x61, 0x6f, 0x8e, 0x47, 0xda, 0xd9, 0xc2, 0xcf, 0x22, 0xe4, 0x40,
        0xab, 0x3d, 0x65, 0xfb, 0xf8, 0x71, 0xa8, 0xbf, 0x1d, 0xef, 0xd7, 0x69,
        0xe1, 0x04, 0x5c, 0x7d, 0x98, 0x0c, 0x55, 0xe6, 0x94, 0x51, 0x91, 0x20,
        0x6e, 0xd6, 0xe5, 0x38, 0x6b, 0xf7, 0x59, 0x06, 0xee, 0x1b, 0x20, 0xae,
        0x87, 0x38, 0xaf, 0xbc, 0x1e, 0xb1, 0x12, 0x74, 0x4a, 0xbd, 0xbb, 0xf5,
        0xea, 0xd6, 0xc4, 0xa6, 0xb3, 0x5e, 0x40, 0xd4, 0x61, 0xb0, 0xfc, 0x16,
        0x08, 0xc9, 0x48, 0xcd, 0xf8, 0xb9, 0xd1, 0x01, 0xe4, 0x29, 0x06, 0xa6,
        0x00, 0x87, 0xc7, 0xc1, 0x49, 0x02, 0xca, 0x52, 0x13, 0x82, 0x27, 0x72,
        0x6a, 0x5b, 0xbf, 0x24, 0xcb, 0x91, 0xe7, 0xc4, 0x5e, 0x2e, 0x57, 0xdd,
        0x2c, 0x24, 0x36, 0xe8, 0x75, 0x23, 0xa7, 0x64, 0x00, 0x6f, 0xa9, 0x8a,
        0x26, 0x63, 0xe1, 0x5a, 0x8e, 0x6d, 0xb9, 0x10, 0x1d, 0xe3, 0xa4, 0xa2,
        0xc8, 0x78, 0xbb, 0x6a, 0x4e, 0xfc, 0x1b, 0xf6, 0x8a, 0xdc, 0xaf, 0xab,
        0xa0, 0xe6, 0x33, 0xf4, 0x24, 0xf4, 0xed, 0x4a, 0x16, 0x7b, 0xff, 0x66,
        0xa3, 0x73, 0x28, 0xc2, 0x0d, 0xcd, 0x4b, 0x64, 0xe4, 0x36, 0xc6, 0xdd,
        0xab, 0x15, 0x0e, 0x8e, 0xce, 0xe0, 0x82, 0x54, 0x7e, 0x23, 0x41, 0x64,
        0x40, 0x91, 0xe8, 0x70, 0xf4, 0xfa, 0xba, 0xce, 0xeb, 0xc8, 0xb3, 0xe5,
        0x0f, 0xcc, 0x09, 0xe2, 0x88, 0x13, 0x0f, 0xc7, 0x94, 0xf1, 0x72, 0x3a,
        0xa8, 0xc2, 0xba, 0x1a, 0xf2, 0xea, 0x7f, 0xf5, 0x8a, 0xec, 0xf0, 0x7c,
        0x1f, 0xa0, 0x74, 0x82, 0x49, 0x5b, 0x0d, 0xcc, 0x48, 0x23, 0xc7, 0x0e,
        0x9f, 0x4a, 0x4a, 0xd2, 0x27, 0x3b, 0x43, 0x8f, 0x81, 0x06, 0x84, 0xbe,
        0xb6, 0x5d, 0xa0, 0x1a, 0x36, 0x5e, 0x10, 0xfe, 0x44, 0x16, 0x6f, 0x78,
        0x60, 0xa6, 0x45, 0xf7, 0x9b, 0xc1, 0xd4, 0x2e, 0xb3, 0xf1, 0x03, 0x06,
        0x46, 0x6f, 0xe5, 0x21, 0x17, 0x32, 0xff, 0xec, 0xad, 0xba, 0x72, 0xc3,
        0x9b, 0xe7, 0xbc, 0x8c, 0xe5, 0xbb, 0xc5, 0xf7, 0x12, 0x6b, 0x2c, 0x43,
        0x9b, 0x3a, 0x40, 0x00, 0x00, 0x00, 0x8c, 0x97, 0x25, 0x8f, 0x4e, 0x24,
        0x89, 0xf1, 0xbb, 0x3d, 0x10, 0x29, 0x14, 0x8e, 0x0d, 0x83, 0x0b, 0x5a,
        0x13, 0x99, 0xda, 0xff, 0x10, 0x84, 0x04, 0x8e, 0x7b, 0xd8, 0xdb, 0xe9,
        0xf8, 0x59, 0x06, 0x9b, 0x88, 0x57, 0xfe, 0xab, 0x81, 0x84, 0xfb, 0x68,
        0x7f, 0x63, 0x46, 0x18, 0xc0, 0x35, 0xda, 0xc4, 0x39, 0xdc, 0x1a, 0xeb,
        0x3b, 0x55, 0x98, 0xa0, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0xdd, 0xf6, 0xe1, 0xd7, 0x65,
        0xa1, 0x93, 0xd9, 0xcb, 0xe1, 0x46, 0xce, 0xeb, 0x79, 0xac, 0x1c, 0xb4,
        0x85, 0xed, 0x5f, 0x5b, 0x37, 0x91, 0x3a, 0x8c, 0xf5, 0x85, 0x7e, 0xff,
        0x00, 0xa9, 0x0c, 0x14, 0xde, 0xfc, 0x82, 0x5e, 0xc6, 0x76, 0x94, 0x25,
        0x08, 0x18, 0xbb, 0x65, 0x40, 0x65, 0xf4, 0x29, 0x8d, 0x31, 0x56, 0xd5,
        0x71, 0xb4, 0xd4, 0xf8, 0x09, 0x0c, 0x18, 0xe9, 0xa8, 0x63, 0x89, 0x0b,
        0xa6, 0x44, 0xfe, 0x1f, 0x55, 0xaa, 0x19, 0xf1, 0x1c, 0xd2, 0xd2, 0xec,
        0x14, 0xd3, 0x23, 0x3b, 0x6e, 0x0a, 0x4b, 0xea, 0xee, 0xf7, 0x2b, 0x69,
        0x85, 0x8e, 0x21, 0xe1, 0x70, 0xd6, 0xf6, 0x4b, 0x55, 0x19, 0x5a, 0xe4,
        0xa4, 0x70, 0x44, 0x21, 0x7e, 0xd4, 0xe0, 0x9d, 0xef, 0x0a, 0x02, 0x23,
        0x8d, 0xe4, 0x0b, 0x7e, 0x44, 0x08, 0x64, 0xb9, 0x6f, 0x8e, 0x0c, 0x53,
        0x72, 0x24, 0x4a, 0xc2, 0xf8, 0xd0, 0xdd, 0x5c, 0xbc, 0x97, 0xe3, 0x28,
        0x9c, 0x19, 0x7c, 0xb5, 0x06, 0x2a, 0x54, 0xf3, 0xd9, 0x56, 0xb9, 0xce,
        0x6e, 0x51, 0x15, 0xf9, 0x65, 0x67, 0xaa, 0x5c, 0xb3, 0xe6, 0xe5, 0x4a,
        0x70, 0x95, 0x28, 0x83, 0x9f, 0x61, 0xc0, 0xb9, 0xb8, 0x60, 0x79, 0x89,
        0x1c, 0x13, 0x92, 0x16, 0xe4, 0x7a, 0x71, 0xb6, 0x2f, 0xb7, 0x3b, 0xec,
        0x72, 0x16, 0x94, 0x58, 0x74, 0x5e, 0x6d, 0x65, 0x90, 0x43, 0xba, 0xa5,
        0x2a, 0x18, 0x1b, 0x58, 0x44, 0x23, 0x9e, 0x8d, 0x54, 0x2d, 0x1f, 0xde,
        0xdc, 0xd6, 0x81, 0xce, 0x7e, 0x71, 0xd0, 0x75, 0x29, 0xda, 0xc9, 0x6e,
        0x26, 0xad, 0x41, 0x24, 0x6e, 0xcc, 0x7d, 0x78, 0xfe, 0x81, 0xe4, 0x17,
        0x73, 0xa4, 0x69, 0x65, 0x41, 0x99, 0x37, 0x92, 0x3a, 0x07, 0x64, 0x47,
        0x97, 0xdf, 0x6f, 0x3e, 0xb5, 0x14, 0x42, 0x60, 0x10, 0xcb, 0x0c, 0x35,
        0xff, 0xa9, 0x05, 0x5a, 0x8e, 0x56, 0x8d, 0xa8, 0xf7, 0xbc, 0x07, 0x56,
        0x15, 0x27, 0x4c, 0xf1, 0xc9, 0x2c, 0xa4, 0x1f, 0x40, 0x00, 0x9c, 0x51,
        0x6a, 0xa4, 0x14, 0xc2, 0x7c, 0x70, 0x87, 0x70, 0x15, 0x7e, 0xeb, 0xeb,
        0x67, 0x8a, 0x65, 0x5d, 0xb9, 0x9b, 0x37, 0xf6, 0xb1, 0x32, 0x6c, 0x76,
        0x57, 0xdb, 0x90, 0xcf, 0xb8, 0xa8, 0x7a, 0xbe, 0xf8, 0xc7, 0xb6, 0xf2,
        0xc8, 0x69, 0x03, 0xc5, 0x8d, 0x25, 0x2b, 0x75, 0x3e, 0xbc, 0xe0, 0x7f,
        0x9d, 0x7d, 0x0c, 0x64, 0x33, 0xff, 0xcc, 0x35, 0x12, 0x94, 0x3e, 0xc3,
        0x09, 0xe2, 0x94, 0x6c, 0xcb, 0x7f, 0xfe, 0x54, 0x11, 0x97, 0x06, 0x09,
        0x00, 0x05, 0x02, 0xe0, 0x93, 0x04, 0x00, 0x0a, 0x06, 0x00, 0x01, 0x00,
        0x0b, 0x0c, 0x0d, 0x01, 0x01, 0x0c, 0x02, 0x00, 0x01, 0x0c, 0x02, 0x00,
        0x00, 0x00, 0x2e, 0x11, 0xa8, 0x6b, 0x00, 0x00, 0x00, 0x00, 0x0d, 0x01,
        0x01, 0x01, 0x11, 0x0e, 0x17, 0x02, 0x00, 0x0f, 0x0b, 0x10, 0x01, 0x03,
        0x04, 0x05, 0x11, 0x06, 0x0d, 0x0d, 0x0c, 0x0a, 0x12, 0x0e, 0x07, 0x13,
        0x14, 0x15, 0x16, 0x08, 0x18, 0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83,
        0xad, 0x2e, 0x11, 0xa8, 0x6b, 0x00, 0x00, 0x00, 0x00, 0x79, 0x6c, 0xb9,
        0x9a, 0x31, 0x01, 0x00, 0x00, 0x0d, 0x03, 0x01, 0x00, 0x00, 0x01, 0x09,
        0x00,
    ];

    #[test]
    fn parse_pump_swap_sell_pool() {
        let tx: VersionedTransaction = bincode::deserialize(SAMPLE_PUMP_SWAP_SELL_TX)
            .expect("failed to deserialize sample tx");
        let parsed = PumpFunParser::parse_transaction(&tx, "pumpswap")
            .expect("expected a sell event");
        assert_eq!(parsed.origin, Origin::PumpSwap);
        assert_eq!(parsed.trade_type, TradeType::PumpSwapSell);
        assert_eq!(
            parsed.mint,
            "HaRvFE4XzR6Vvsyx6ZUkjzfwsEtqG9BcgW2ZoTaEBkew"
        );
        assert_eq!(
            parsed.pool.as_deref(),
            Some("GRnGDvefECgeiYzHWq7RqB8QKdELrbx6FQB9tGLXvExX")
        );
    }
}

