use alloy_primitives::{hex, FixedBytes};
use ocl::enums::DeviceInfo;
use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue};
use std::error::Error;
use std::io::prelude::*;
use std::io::stdout;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{
    output_file, PatternMode, CONTROL_CHARACTER, ERC6551_CONSTRUCTOR_HEADER, ERC6551_FOOTER,
};

// Keccak-256 OpenCL kernel
const KECCAK_KERNEL: &str = r#"
// Keccak-256 constants
__constant ulong RC[24] = {
    0x0000000000000001UL, 0x0000000000008082UL, 0x800000000000808aUL,
    0x8000000080008000UL, 0x000000000000808bUL, 0x0000000080000001UL,
    0x8000000080008081UL, 0x8000000000008009UL, 0x000000000000008aUL,
    0x0000000000000088UL, 0x0000000080008009UL, 0x000000008000000aUL,
    0x000000008000808bUL, 0x800000000000008bUL, 0x8000000000008089UL,
    0x8000000000008003UL, 0x8000000000008002UL, 0x8000000000000080UL,
    0x000000000000800aUL, 0x800000008000000aUL, 0x8000000080008081UL,
    0x8000000000008080UL, 0x0000000080000001UL, 0x8000000080008008UL
};

__constant int ROTC[24] = {
    1,  3,  6,  10, 15, 21, 28, 36, 45, 55, 2,  14,
    27, 41, 56, 8,  25, 43, 62, 18, 39, 61, 20, 44
};

__constant int PILN[24] = {
    10, 7,  11, 17, 18, 3, 5,  16, 8,  21, 24, 4,
    15, 23, 19, 13, 12, 2, 20, 14, 22, 9,  6,  1
};

inline ulong rotl64(ulong x, int n) {
    return (x << n) | (x >> (64 - n));
}

void keccak_f1600(__private ulong *st) {
    ulong t, bc[5];
    
    for (int r = 0; r < 24; r++) {
        // Theta
        for (int i = 0; i < 5; i++)
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        
        for (int i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ rotl64(bc[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5)
                st[j + i] ^= t;
        }
        
        // Rho Pi
        t = st[1];
        for (int i = 0; i < 24; i++) {
            int j = PILN[i];
            bc[0] = st[j];
            st[j] = rotl64(t, ROTC[i]);
            t = bc[0];
        }
        
        // Chi
        for (int j = 0; j < 25; j += 5) {
            for (int i = 0; i < 5; i++)
                bc[i] = st[j + i];
            for (int i = 0; i < 5; i++)
                st[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
        }
        
        // Iota
        st[0] ^= RC[r];
    }
}

void keccak256(__private uchar *input, int len, __private uchar *output) {
    ulong st[25];
    for (int i = 0; i < 25; i++) st[i] = 0;
    
    int rate = 136; // rate for keccak-256
    int offset = 0;
    
    // Absorb
    while (len >= rate) {
        for (int i = 0; i < rate / 8; i++) {
            ulong val = 0;
            for (int j = 0; j < 8; j++) {
                val |= ((ulong)input[offset + i * 8 + j]) << (j * 8);
            }
            st[i] ^= val;
        }
        keccak_f1600(st);
        offset += rate;
        len -= rate;
    }
    
    // Final block with padding
    uchar temp[136];
    for (int i = 0; i < 136; i++) temp[i] = 0;
    for (int i = 0; i < len; i++) temp[i] = input[offset + i];
    temp[len] = 0x01;
    temp[rate - 1] |= 0x80;
    
    for (int i = 0; i < rate / 8; i++) {
        ulong val = 0;
        for (int j = 0; j < 8; j++) {
            val |= ((ulong)temp[i * 8 + j]) << (j * 8);
        }
        st[i] ^= val;
    }
    keccak_f1600(st);
    
    // Squeeze
    for (int i = 0; i < 4; i++) {
        ulong val = st[i];
        for (int j = 0; j < 8; j++) {
            output[i * 8 + j] = (uchar)(val >> (j * 8));
        }
    }
}

// Check if hex string starts with pattern
int check_prefix(__private uchar *addr, __global uchar *pattern, int pattern_len) {
    for (int i = 0; i < pattern_len; i++) {
        int byte_idx = i / 2;
        int nibble;
        if (i % 2 == 0) {
            nibble = (addr[byte_idx] >> 4) & 0x0F;
        } else {
            nibble = addr[byte_idx] & 0x0F;
        }
        uchar expected = pattern[i];
        uchar hex_char;
        if (nibble < 10) {
            hex_char = '0' + nibble;
        } else {
            hex_char = 'a' + nibble - 10;
        }
        if (hex_char != expected) return 0;
    }
    return 1;
}

// Check if hex string contains pattern
int check_contains(__private uchar *addr, __global uchar *pattern, int pattern_len) {
    // For each possible starting position in the 40-char hex string
    for (int start = 0; start <= 40 - pattern_len; start++) {
        int match = 1;
        for (int i = 0; i < pattern_len && match; i++) {
            int pos = start + i;
            int byte_idx = pos / 2;
            int nibble;
            if (pos % 2 == 0) {
                nibble = (addr[byte_idx] >> 4) & 0x0F;
            } else {
                nibble = addr[byte_idx] & 0x0F;
            }
            uchar hex_char;
            if (nibble < 10) {
                hex_char = '0' + nibble;
            } else {
                hex_char = 'a' + nibble - 10;
            }
            if (hex_char != pattern[i]) match = 0;
        }
        if (match) return 1;
    }
    return 0;
}

__kernel void erc6551_crunch(
    __global uchar *header_base,        // 47 bytes: control + registry + impl + random(6)
    __global uchar *bytecode_header,    // 55 bytes
    __global uchar *impl_addr,          // 20 bytes
    __global uchar *bytecode_body,      // 6 bytes (random part)
    __global uchar *bytecode_footer,    // 96 bytes
    __global uchar *pattern,            // Pattern to match (hex chars)
    int pattern_len,                    // Length of pattern
    int pattern_mode,                   // 0=prefix, 1=contains
    ulong salt_offset,                  // Starting salt offset
    __global ulong *results_salt,       // Output: found salts
    __global uchar *results_addr,       // Output: found addresses (20 bytes each)
    __global uchar *results_hash,       // Output: init code hashes (32 bytes each)
    __global int *results_count,        // Output: number of results found
    int max_results                     // Maximum results to store
) {
    ulong gid = get_global_id(0);
    ulong salt = salt_offset + gid;
    
    // Prepare salt bytes (6 bytes, little endian)
    uchar salt_bytes[8];
    for (int i = 0; i < 8; i++) {
        salt_bytes[i] = (salt >> (i * 8)) & 0xFF;
    }
    
    // Build bytecode for init_code_hash
    // bytecode = header(55) + impl(20) + body(6) + salt(6) + footer(96) = 183 bytes
    uchar bytecode[183];
    for (int i = 0; i < 55; i++) bytecode[i] = bytecode_header[i];
    for (int i = 0; i < 20; i++) bytecode[55 + i] = impl_addr[i];
    for (int i = 0; i < 6; i++) bytecode[75 + i] = bytecode_body[i];
    for (int i = 0; i < 6; i++) bytecode[81 + i] = salt_bytes[i];
    for (int i = 0; i < 96; i++) bytecode[87 + i] = bytecode_footer[i];
    
    // Compute init_code_hash = keccak256(bytecode)
    uchar init_code_hash[32];
    keccak256(bytecode, 183, init_code_hash);
    
    // Build CREATE2 input
    // create2_input = header(47) + salt(6) + init_code_hash(32) = 85 bytes
    uchar create2_input[85];
    for (int i = 0; i < 47; i++) create2_input[i] = header_base[i];
    for (int i = 0; i < 6; i++) create2_input[47 + i] = salt_bytes[i];
    for (int i = 0; i < 32; i++) create2_input[53 + i] = init_code_hash[i];
    
    // Compute address = keccak256(create2_input)[12:32]
    uchar create2_hash[32];
    keccak256(create2_input, 85, create2_hash);
    
    // Extract address (last 20 bytes)
    uchar address[20];
    for (int i = 0; i < 20; i++) {
        address[i] = create2_hash[12 + i];
    }
    
    // Check pattern based on mode
    int matched = 0;
    if (pattern_mode == 0) {
        matched = check_prefix(address, pattern, pattern_len);
    } else if (pattern_mode == 1) {
        matched = check_contains(address, pattern, pattern_len);
    }
    
    if (matched) {
        int idx = atomic_add(results_count, 1);
        if (idx < max_results) {
            results_salt[idx] = salt;
            for (int i = 0; i < 20; i++) {
                results_addr[idx * 20 + i] = address[i];
            }
            for (int i = 0; i < 32; i++) {
                results_hash[idx * 32 + i] = init_code_hash[i];
            }
        }
    }
}
"#;

const GPU_BATCH_SIZE: usize = 1 << 22; // 4M addresses per batch
const MAX_RESULTS_PER_BATCH: usize = 1024;

pub fn gpu(config: crate::Config) -> Result<(), Box<dyn Error>> {
    // Initialize OpenCL
    println!("🔍 Detecting GPU devices...");

    let platform = Platform::default();
    let device = Device::first(platform)?;

    println!("🎮 GPU: {} ({})", device.name()?, device.vendor()?);
    println!(
        "   Max compute units: {}",
        device.info(DeviceInfo::MaxComputeUnits)?
    );
    println!("   Max work group size: {}", device.max_wg_size()?);

    let context = Context::builder()
        .platform(platform)
        .devices(device)
        .build()?;
    let queue = Queue::new(&context, device, None)?;

    let program = Program::builder()
        .src(KECCAK_KERNEL)
        .devices(device)
        .build(&context)?;

    // Prepare data
    let file = Arc::new(Mutex::new(output_file()));

    // Prepare bytecode components
    let mut bytecode_header = [0u8; 55];
    bytecode_header[0..20].copy_from_slice(&ERC6551_CONSTRUCTOR_HEADER);
    bytecode_header[20..40].copy_from_slice(&config.implement_address);
    bytecode_header[40..].copy_from_slice(&ERC6551_FOOTER);

    let mut bytecode_footer = [0u8; 96];
    bytecode_footer[0..32].copy_from_slice(&config.chain_id);
    bytecode_footer[44..64].copy_from_slice(&config.nft_address);
    bytecode_footer[64..].copy_from_slice(&config.token_id);

    // Parse pattern
    let pattern = config.pattern.to_lowercase();
    let pattern = pattern.strip_prefix("0x").unwrap_or(&pattern);
    let pattern_bytes: Vec<u8> = pattern.as_bytes().to_vec();
    let pattern_len = pattern_bytes.len();

    let pattern_mode_int: i32 = match config.pattern_mode {
        PatternMode::Prefix => 0,
        PatternMode::Contains => 1,
    };

    match config.pattern_mode {
        PatternMode::Prefix => println!("🔍 Searching for addresses starting with: 0x{}", pattern),
        PatternMode::Contains => println!("🔍 Searching for addresses containing: 0x{}", pattern),
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Create buffers
    let buf_bytecode_header = Buffer::<u8>::builder()
        .queue(queue.clone())
        .len(55)
        .copy_host_slice(&bytecode_header)
        .build()?;

    let buf_impl_addr = Buffer::<u8>::builder()
        .queue(queue.clone())
        .len(20)
        .copy_host_slice(&config.implement_address)
        .build()?;

    let buf_bytecode_footer = Buffer::<u8>::builder()
        .queue(queue.clone())
        .len(96)
        .copy_host_slice(&bytecode_footer)
        .build()?;

    let buf_pattern = Buffer::<u8>::builder()
        .queue(queue.clone())
        .len(pattern_bytes.len().max(1))
        .copy_host_slice(if pattern_bytes.is_empty() {
            &[0u8]
        } else {
            &pattern_bytes
        })
        .build()?;

    // Output buffers
    let buf_results_salt = Buffer::<u64>::builder()
        .queue(queue.clone())
        .len(MAX_RESULTS_PER_BATCH)
        .build()?;

    let buf_results_addr = Buffer::<u8>::builder()
        .queue(queue.clone())
        .len(MAX_RESULTS_PER_BATCH * 20)
        .build()?;

    let buf_results_hash = Buffer::<u8>::builder()
        .queue(queue.clone())
        .len(MAX_RESULTS_PER_BATCH * 32)
        .build()?;

    let buf_results_count = Buffer::<i32>::builder()
        .queue(queue.clone())
        .len(1)
        .build()?;

    // Speed tracking
    let total_checked = Arc::new(AtomicU64::new(0));
    let found_count = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    // Spawn a thread to print speed stats
    let total_checked_clone = Arc::clone(&total_checked);
    let found_count_clone = Arc::clone(&found_count);
    std::thread::spawn(move || {
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
                "\r🚀 GPU Speed: {:.2}M/s | Avg: {:.2}M/s | Checked: {}M | Found: {} | Time: {:.0}s    ",
                instant_speed / 1_000_000.0,
                speed / 1_000_000.0,
                current / 1_000_000,
                found,
                elapsed
            );
            let _ = std::io::stderr().flush();
        }
    });

    // Main loop
    let mut global_salt_offset: u64 = 0;

    loop {
        // Generate random header for this batch
        let mut header = [0u8; 47];
        header[0] = CONTROL_CHARACTER;
        header[1..21].copy_from_slice(&config.resistry_address);
        header[21..41].copy_from_slice(&config.implement_address);
        let random_bytes: [u8; 6] = FixedBytes::<6>::random().0;
        header[41..].copy_from_slice(&random_bytes);

        let bytecode_body = random_bytes;

        // Create per-batch buffers
        let buf_header = Buffer::<u8>::builder()
            .queue(queue.clone())
            .len(47)
            .copy_host_slice(&header)
            .build()?;

        let buf_bytecode_body = Buffer::<u8>::builder()
            .queue(queue.clone())
            .len(6)
            .copy_host_slice(&bytecode_body)
            .build()?;

        // Reset results count
        let zero = [0i32].to_vec();
        buf_results_count.write(&zero).enq()?;

        // Build and execute kernel
        let kernel = Kernel::builder()
            .program(&program)
            .name("erc6551_crunch")
            .queue(queue.clone())
            .global_work_size(GPU_BATCH_SIZE)
            .arg(&buf_header)
            .arg(&buf_bytecode_header)
            .arg(&buf_impl_addr)
            .arg(&buf_bytecode_body)
            .arg(&buf_bytecode_footer)
            .arg(&buf_pattern)
            .arg(pattern_len as i32)
            .arg(pattern_mode_int)
            .arg(global_salt_offset)
            .arg(&buf_results_salt)
            .arg(&buf_results_addr)
            .arg(&buf_results_hash)
            .arg(&buf_results_count)
            .arg(MAX_RESULTS_PER_BATCH as i32)
            .build()?;

        unsafe {
            kernel.enq()?;
        }
        queue.finish()?;

        // Update counters
        total_checked.fetch_add(GPU_BATCH_SIZE as u64, Ordering::Relaxed);
        global_salt_offset += GPU_BATCH_SIZE as u64;

        // Check for results
        let mut result_count = [0i32].to_vec();
        buf_results_count.read(&mut result_count).enq()?;

        if result_count[0] > 0 {
            let count = (result_count[0] as usize).min(MAX_RESULTS_PER_BATCH);

            let mut results_salt = vec![0u64; count];
            let mut results_addr = vec![0u8; count * 20];
            let mut results_hash = vec![0u8; count * 32];

            buf_results_salt.read(&mut results_salt).enq()?;
            buf_results_addr.read(&mut results_addr).enq()?;
            buf_results_hash.read(&mut results_hash).enq()?;

            for i in 0..count {
                let salt = results_salt[i];
                let addr = &results_addr[i * 20..(i + 1) * 20];

                found_count.fetch_add(1, Ordering::Relaxed);

                let header_hex = hex::encode(&header);
                let salt_bytes = salt.to_le_bytes();
                let salt_hex = hex::encode(&salt_bytes[..6]);
                let full_salt = format!("0x{}{}", &header_hex[42..], salt_hex);
                let addr_hex = hex::encode(addr);

                let output = format!("\n{} => 0x{}\n", full_salt, addr_hex);
                print!("{output}");
                let _ = stdout().flush();

                {
                    let mut f = file.lock().unwrap();
                    writeln!(f, "{} => 0x{}", full_salt, addr_hex)
                        .expect("Couldn't write to result.txt file.");
                }
            }
        }
    }
}

/// List available GPU devices
pub fn list_gpus() -> Result<(), Box<dyn Error>> {
    println!("Available OpenCL devices:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for platform in Platform::list() {
        println!("Platform: {}", platform.name()?);
        for device in Device::list_all(platform)? {
            println!("  📊 {} ({})", device.name()?, device.vendor()?);
            println!("     Type: {}", device.info(DeviceInfo::Type)?);
            println!(
                "     Compute Units: {}",
                device.info(DeviceInfo::MaxComputeUnits)?
            );
            println!("     Max Work Group Size: {}", device.max_wg_size()?);
            if let Ok(mem) = device.info(DeviceInfo::GlobalMemSize) {
                println!("     Global Memory: {}", mem);
            }
        }
    }

    Ok(())
}
