use erc6551crunch::Config;
use std::env;
use std::process;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ERC6551 Vanity Address Cruncher                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("Usage: erc6551crunch [OPTIONS]");
        println!();
        println!("Required Arguments:");
        println!("  -i, --impl <address>           Implementation contract address");
        println!("  -c, --chain <id>               Chain ID (decimal)");
        println!("  -n, --nft <address>            NFT contract address");
        println!("  -t, --token <id>               Token ID (decimal)");
        println!("  -p, --prefix <pattern>         Search for addresses STARTING with pattern");
        println!("      OR --contains <pattern>    Search for addresses CONTAINING pattern");
        println!();
        println!("Optional Arguments:");
        println!(
            "  -r, --registry <address>       ERC6551 Registry (default: 0x000000006551c19487814612e58FE06813775758)"
        );
        println!("  -w, --workers <num>            Number of CPU threads (default: all cores)");
        println!("  -g, --gpu                      Use GPU acceleration (OpenCL)");
        println!("      --list-gpus                List available GPU devices");
        println!("  -h, --help                     Show this help message");
        println!();
        println!("Examples:");
        println!("  # Find addresses starting with '00000' (uses default registry):");
        println!("  erc6551crunch -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \\");
        println!("                -c 1 -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D -t 1 \\");
        println!("                -p 00000");
        println!();
        println!("  # GPU mode - Much faster:");
        println!("  erc6551crunch -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \\");
        println!("                -c 1 -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D -t 1 \\");
        println!("                -p 00000000 --gpu");
        println!();
        println!("  # List available GPUs:");
        println!("  erc6551crunch --list-gpus");
        process::exit(0);
    }

    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Error: {err}");
        eprintln!("Use --help for usage information.");
        process::exit(1);
    });

    if config.use_gpu {
        println!("🚀 GPU Mode enabled");
        if let Err(e) = erc6551crunch::gpu(config) {
            eprintln!("GPU application error: {e}");
            eprintln!("Tip: Make sure you have OpenCL drivers installed.");
            eprintln!("     For NVIDIA: Install CUDA Toolkit");
            eprintln!("     For AMD: Install AMD APP SDK or ROCm");
            eprintln!("     For Intel: Install Intel OpenCL Runtime");
            process::exit(1);
        }
    } else {
        if let Err(e) = erc6551crunch::cpu(config) {
            eprintln!("CPU application error: {e}");
            process::exit(1);
        }
    }
}
