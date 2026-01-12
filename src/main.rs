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
        println!("  -r, --registry <address>       ERC6551 Registry contract address");
        println!("  -i, --impl <address>           Implementation contract address");
        println!("  -c, --chain <id>               Chain ID (decimal)");
        println!("  -n, --nft <address>            NFT contract address");
        println!("  -t, --token <id>               Token ID (decimal)");
        println!();
        println!("Optional Arguments:");
        println!("  -p, --prefix <pattern>         Search for addresses STARTING with pattern");
        println!("      --contains <pattern>       Search for addresses CONTAINING pattern");
        println!("  -w, --workers <num>            Number of threads (default: all CPU cores)");
        println!("  -h, --help                     Show this help message");
        println!();
        println!("Examples:");
        println!("  # Search for addresses with reward (0x11 pattern):");
        println!("  erc6551crunch -r 0x000000006551c19487814612e58FE06813775758 \\");
        println!("                -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \\");
        println!("                -c 1 -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D -t 1");
        println!();
        println!("  # Search for addresses STARTING with 'dead':");
        println!("  erc6551crunch -r 0x000000006551c19487814612e58FE06813775758 \\");
        println!("                -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \\");
        println!("                -c 1 -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D -t 1 \\");
        println!("                -p dead");
        println!();
        println!("  # Search for addresses CONTAINING 'beef':");
        println!("  erc6551crunch -r 0x000000006551c19487814612e58FE06813775758 \\");
        println!("                -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \\");
        println!("                -c 1 -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D -t 1 \\");
        println!("                --contains beef");
        process::exit(0);
    }

    let config = Config::new(env::args()).unwrap_or_else(|err| {
        eprintln!("Error: {err}");
        eprintln!("Use --help for usage information.");
        process::exit(1);
    });

    if let Err(e) = erc6551crunch::cpu(config) {
        eprintln!("CPU application error: {e}");
        process::exit(1);
    }
}
