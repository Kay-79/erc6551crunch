use alloy_primitives::{Address, FixedBytes, hex};
use rayon::prelude::*;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::stdout;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tiny_keccak::{Hasher, Keccak};
pub mod gpu;
pub use gpu::{gpu, list_gpus};
const CONTROL_CHARACTER: u8 = 0xff;
const MAX_INCREMENTER: u64 = 0xffffffffffff;

/// ERC6551 Registry address (same on all EVM chains)
/// https://eips.ethereum.org/EIPS/eip-6551
pub const ERC6551_REGISTRY: [u8; 20] = [
    0x00, 0x00, 0x00, 0x00, 0x65, 0x51, 0xc1, 0x94, 0x87, 0x81,
    0x46, 0x12, 0xe5, 0x8F, 0xE0, 0x68, 0x13, 0x77, 0x57, 0x58,
];

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
    pub pattern: String,
    pub pattern_mode: PatternMode,
    pub num_threads: usize,
    pub use_gpu: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PatternMode {
    Prefix,   // Pattern must be at start of address
    Contains, // Pattern can be anywhere in address
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Self, &'static str> {
        args.next(); // skip program name

        let mut resistry_address_string: Option<String> = None;
        let mut implement_address_string: Option<String> = None;
        let mut chain_id_string: Option<String> = None;
        let mut nft_address_string: Option<String> = None;
        let mut token_id_string: Option<String> = None;
        let mut pattern = String::new();
        let mut pattern_mode = PatternMode::Prefix;
        let mut num_threads: usize = 0; // 0 = auto (use all cores)
        let mut use_gpu = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--gpu" | "-g" => {
                    use_gpu = true;
                }
                "--list-gpus" => {
                    let _ = crate::list_gpus();
                    std::process::exit(0);
                }
                "--workers" | "-w" => {
                    let threads_str = args.next().ok_or("--workers requires a number")?;
                    num_threads = threads_str
                        .parse()
                        .map_err(|_| "--workers must be a positive number")?;
                }
                "--registry" | "-r" => {
                    resistry_address_string =
                        Some(args.next().ok_or("--registry requires an address")?);
                }
                "--implementation" | "--impl" | "-i" => {
                    implement_address_string =
                        Some(args.next().ok_or("--implementation requires an address")?);
                }
                "--chain" | "-c" => {
                    chain_id_string = Some(args.next().ok_or("--chain requires a chain ID")?);
                }
                "--nft" | "-n" => {
                    nft_address_string = Some(args.next().ok_or("--nft requires an address")?);
                }
                "--token" | "-t" => {
                    token_id_string = Some(args.next().ok_or("--token requires a token ID")?);
                }
                "--prefix" | "-p" => {
                    pattern = args.next().ok_or("--prefix requires a pattern")?;
                    pattern_mode = PatternMode::Prefix;
                }
                "--contains" => {
                    pattern = args.next().ok_or("--contains requires a pattern")?;
                    pattern_mode = PatternMode::Contains;
                }
                _ => {
                    return Err("Unknown argument. Use --help for usage.");
                }
            }
        }

        let resistry_address_string = resistry_address_string;
        let implement_address_string =
            implement_address_string.ok_or("Missing --implementation argument")?;
        let chain_id_string = chain_id_string.ok_or("Missing --chain argument")?;
        let nft_address_string = nft_address_string.ok_or("Missing --nft argument")?;
        let token_id_string = token_id_string.ok_or("Missing --token argument")?;

        if pattern.is_empty() {
            return Err("Missing pattern. Use --prefix or --contains to specify a pattern.");
        }

        // Use default ERC6551 Registry if not specified
        let resistry_address: [u8; 20] = if let Some(addr_str) = resistry_address_string {
            let Ok(vec) = hex::decode(addr_str) else {
                return Err("could not decode registry address argument");
            };
            vec.try_into().map_err(|_| "invalid length for registry address")?  
        } else {
            ERC6551_REGISTRY
        };
        
        let Ok(implement_address_vec) = hex::decode(implement_address_string) else {
            return Err("could not decode implement address argument");
        };
        let Ok(nft_address_vec) = hex::decode(nft_address_string) else {
            return Err("could not decode nft address argument");
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
            pattern,
            pattern_mode,
            num_threads,
            use_gpu,
        })
    }
}

pub fn cpu(config: Config) -> Result<(), Box<dyn Error>> {
    // Set thread pool size
    let num_threads = if config.num_threads > 0 {
        config.num_threads
    } else {
        rayon::current_num_threads()
    };

    if config.num_threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .ok(); // Ignore error if already initialized
    }

    println!("🧵 Using {} threads", num_threads);

    let file = Arc::new(Mutex::new(output_file()));
    let mut header_bytes_code_header = [0; 55];
    header_bytes_code_header[0..20].copy_from_slice(&ERC6551_CONSTRUCTOR_HEADER);
    header_bytes_code_header[20..40].copy_from_slice(&config.implement_address);
    header_bytes_code_header[40..].copy_from_slice(&ERC6551_FOOTER);
    let mut header_bytes_code_body = [0; 6];
    let mut header_bytes_code_footer = [0; 96];
    header_bytes_code_footer[0..32].copy_from_slice(&config.chain_id);
    header_bytes_code_footer[44..64].copy_from_slice(&config.nft_address);
    header_bytes_code_footer[64..].copy_from_slice(&config.token_id);

    // Parse pattern - convert to lowercase hex bytes for matching
    let pattern = config.pattern.to_lowercase();
    let pattern = pattern.strip_prefix("0x").unwrap_or(&pattern);
    let pattern_mode = config.pattern_mode;

    match pattern_mode {
        PatternMode::Prefix => println!("🔍 Searching for addresses starting with: 0x{}", pattern),
        PatternMode::Contains => println!("🔍 Searching for addresses containing: 0x{}", pattern),
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Speed tracking with Arc for thread safety
    let total_checked = Arc::new(AtomicU64::new(0));
    let found_count = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    // Spawn a thread to print speed stats
    let total_checked_clone = Arc::clone(&total_checked);
    let found_count_clone = Arc::clone(&found_count);
    std::thread::spawn(move || {
        use std::io::Write;
        let mut last_count = 0u64;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let current = total_checked_clone.load(Ordering::Relaxed);
            let found = found_count_clone.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = current as f64 / elapsed;
            let instant_speed = (current - last_count) as f64 / 2.0;
            last_count = current;

            eprint!(
                "\r⚡ Speed: {:.2}M/s | Avg: {:.2}M/s | Checked: {}M | Found: {} | Time: {:.0}s    ",
                instant_speed / 1_000_000.0,
                speed / 1_000_000.0,
                current / 1_000_000,
                found,
                elapsed
            );
            let _ = std::io::stderr().flush();
        }
    });

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

            total_checked.fetch_add(1, Ordering::Relaxed);

            // Check pattern match based on mode
            match pattern_mode {
                PatternMode::Prefix => {
                    let addr_hex = hex::encode(address); // already lowercase
                    if !addr_hex.starts_with(pattern) {
                        return;
                    }
                    // Pattern matched at prefix!
                    found_count.fetch_add(1, Ordering::Relaxed);
                    let header_hex_string = hex::encode(header);
                    let body_hex_string = hex::encode(salt_incremented_segment);
                    let full_salt = format!("0x{}{}", &header_hex_string[42..], &body_hex_string);
                    let output = format!("\n{} => 0x{}\n", full_salt, addr_hex);
                    print!("{output}");
                    let _ = stdout().flush();
                    {
                        let mut f = file.lock().unwrap();
                        writeln!(f, "{} => 0x{}", full_salt, addr_hex)
                            .expect("Couldn't write to `result.txt` file.");
                    }
                    return;
                }
                PatternMode::Contains => {
                    let addr_hex = hex::encode(address); // already lowercase
                    if !addr_hex.contains(pattern) {
                        return;
                    }
                    // Pattern found in address!
                    found_count.fetch_add(1, Ordering::Relaxed);
                    let header_hex_string = hex::encode(header);
                    let body_hex_string = hex::encode(salt_incremented_segment);
                    let full_salt = format!("0x{}{}", &header_hex_string[42..], &body_hex_string);
                    let output = format!("\n{} => 0x{}\n", full_salt, addr_hex);
                    print!("{output}");
                    let _ = stdout().flush();
                    {
                        let mut f = file.lock().unwrap();
                        writeln!(f, "{} => 0x{}", full_salt, addr_hex)
                            .expect("Couldn't write to `result.txt` file.");
                    }
                }
            }
        });
    }
}

#[track_caller]
fn output_file() -> File {
    // Always save to executable's parent directory (project root when running from target/release)
    let result_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .and_then(|dir| {
            // If running from target/release, go up 2 levels to project root
            if dir.ends_with("release") || dir.ends_with("debug") {
                dir.parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            } else {
                Some(dir)
            }
        })
        .map(|dir| dir.join("result.txt"))
        .unwrap_or_else(|| std::path::PathBuf::from("result.txt"));

    println!("📁 Saving results to: {}", result_path.display());

    OpenOptions::new()
        .append(true)
        .create(true)
        .read(true)
        .open(&result_path)
        .expect("Could not create or open `result.txt` file.")
}
