use alloy_primitives::{hex, Address, FixedBytes};
use fs4::FileExt;
use rayon::prelude::*;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use tiny_keccak::{Hasher, Keccak};
mod reward;
pub use reward::Reward;
const CONTROL_CHARACTER: u8 = 0xff;
const MAX_INCREMENTER: u64 = 0xffffffffffff;
const ERC6551_CONSTRUCTOR_HEADER: [u8; 20] = [
    61, 96, 173, 128, 96, 10, 61, 57, 129, 243, 54, 61, 61, 55, 61, 61, 61, 54, 61, 115,
];
const ERC6551_FOOTER: [u8; 15] = [
    90, 244, 61, 130, 128, 62, 144, 61, 145, 96, 43, 87, 253, 91, 243,
];

pub struct Config {
    pub resistry_address: [u8; 20],
    pub implement_address: [u8; 20],
    pub chain_id: [u8; 32],
    pub nft_address: [u8; 20],
    pub token_id: [u8; 32],
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Self, &'static str> {
        args.next();

        let Some(resistry_address_string) = args.next() else {
            return Err("didn't get a resistry_address argument");
        };
        let Some(implement_address_string) = args.next() else {
            return Err("didn't get a implement_address argument");
        };
        let Some(chain_id_string) = args.next() else {
            return Err("didn't get a chain_id argument");
        };
        let Some(nft_address_string) = args.next() else {
            return Err("didn't get a nft_address argument");
        };
        let Some(token_id_string) = args.next() else {
            return Err("didn't get a token_id argument");
        };

        let Ok(resistry_address_vec) = hex::decode(resistry_address_string) else {
            return Err("could not decode resistry address argument");
        };
        let Ok(implement_address_vec) = hex::decode(implement_address_string) else {
            return Err("could not decode implement address argument");
        };
        let Ok(nft_address_vec) = hex::decode(nft_address_string) else {
            return Err("could not decode nft address argument");
        };
        let Ok(resistry_address) = resistry_address_vec.try_into() else {
            return Err("invalid length for resistry address argument");
        };
        let Ok(implement_address) = implement_address_vec.try_into() else {
            return Err("invalid length for implement address argument");
        };
        let Ok(chain_id) = chain_id_string.parse::<u128>() else {
            return Err("could not parse chain id as decimal integer");
        };
        let chain_id_bytes: [u8; 16] = chain_id.to_be_bytes();
        let mut chain_id_vec = [0u8; 32];
        chain_id_vec[16..].copy_from_slice(&chain_id_bytes);
        let Ok(nft_address) = nft_address_vec.try_into() else {
            return Err("invalid length for nft address argument");
        };
        let Ok(token_id) = token_id_string.parse::<u128>() else {
            return Err("could not parse token id as decimal integer");
        };
        let token_id_bytes: [u8; 16] = token_id.to_be_bytes();
        let mut token_id_vec = [0u8; 32];
        token_id_vec[16..].copy_from_slice(&token_id_bytes);

        Ok(Self {
            resistry_address,
            implement_address,
            chain_id: chain_id_vec,
            nft_address,
            token_id: token_id_vec,
        })
    }
}

pub fn cpu(config: Config) -> Result<(), Box<dyn Error>> {
    let file = output_file();
    let rewards = Reward::new();
    let mut header_bytes_code_header = [0; 55];
    header_bytes_code_header[0..20].copy_from_slice(&ERC6551_CONSTRUCTOR_HEADER);
    header_bytes_code_header[20..40].copy_from_slice(&config.implement_address);
    header_bytes_code_header[40..].copy_from_slice(&ERC6551_FOOTER);
    let mut header_bytes_code_body = [0; 6];
    let mut header_bytes_code_footer = [0; 96];
    header_bytes_code_footer[0..32].copy_from_slice(&config.chain_id);
    header_bytes_code_footer[44..64].copy_from_slice(&config.nft_address);
    header_bytes_code_footer[64..].copy_from_slice(&config.token_id);
    loop {
        let mut header = [0; 47];
        header[0] = CONTROL_CHARACTER;
        header[1..21].copy_from_slice(&config.resistry_address);
        header[21..41].copy_from_slice(&config.implement_address);
        header[41..].copy_from_slice(&FixedBytes::<6>::random()[..]);
        header_bytes_code_body.copy_from_slice(&header[41..]);
        let mut hash_header = Keccak::v256();
        hash_header.update(&header);
        (0..MAX_INCREMENTER).into_par_iter().for_each(|salt| {
            let salt = salt.to_le_bytes();
            let salt_incremented_segment = &salt[..6];
            let mut hash = hash_header.clone();
            let mut hash_bytescode = Keccak::v256();
            hash_bytescode.update(&header_bytes_code_header);
            hash_bytescode.update(&config.implement_address);
            hash_bytescode.update(&header_bytes_code_body);
            hash_bytescode.update(&salt_incremented_segment);
            hash_bytescode.update(&header_bytes_code_footer);
            let mut keccak_bytescode: [u8; 32] = [0; 32];
            hash_bytescode.finalize(&mut keccak_bytescode);
            hash.update(salt_incremented_segment);
            hash.update(&keccak_bytescode);
            let mut keccak_create2: [u8; 32] = [0; 32];
            hash.finalize(&mut keccak_create2);
            let address = <&Address>::try_from(&keccak_create2[12..]).unwrap();
            let mut total = 0;
            let mut leading = 21;
            let target_char = 17;
            for (i, &b) in address.iter().enumerate() {
                if b == target_char {
                    total += 1;
                } else if leading == 21 {
                    leading = i;
                }
            }
            if total < 3 {
                return;
            }
            let key = leading * 20 + total;
            let reward_amount = rewards.get(&key);
            if reward_amount.is_none() {
                return;
            }
            let header_hex_string = hex::encode(header);
            let body_hex_string = hex::encode(salt_incremented_segment);
            let full_salt = format!("0x{}{}", &header_hex_string[42..], &body_hex_string);
            let output = format!(
                "salt: {full_salt} => init_code_hash: 0x{} => address: {address} => reward: {}",
                hex::encode(keccak_bytescode),
                reward_amount.unwrap_or("0")
            );
            println!("{output}");
            file.lock_exclusive().expect("Couldn't lock file.");
            writeln!(&file, "{output}").expect("Couldn't write to `result.txt` file.");
            file.unlock().expect("Couldn't unlock file.")
        });
    }
}

#[track_caller]
fn output_file() -> File {
    OpenOptions::new()
        .append(true)
        .create(true)
        .read(true)
        .open("result.txt")
        .expect("Could not create or open `result.txt` file.")
}
