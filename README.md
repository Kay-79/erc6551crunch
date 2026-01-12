# erc6551crunch

Rust tool for generating vanity ERC6551 tokenbound account addresses.
Supports both **CPU** (multi-threaded) and **GPU** (OpenCL) acceleration.

## What is ERC6551?

[ERC-6551](https://eips.ethereum.org/EIPS/eip-6551) allows every NFT to have its own smart contract wallet (Token Bound Account - TBA). This tool helps you find a "vanity" salt that creates a TBA with a memorable address pattern (e.g., `0x00000000...`, `0xdead...`).

**Registry Address**: `0x000000006551c19487814612e58FE06813775758` (same on all EVM chains)

## Installation

```shell
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/Kay-79/erc6551crunch.git
cd erc6551crunch
cargo build --release
```

### GPU Requirements (Optional)
- **NVIDIA**: Install CUDA Toolkit
- **AMD**: Install AMD APP SDK or ROCm
- **Intel**: Install Intel OpenCL Runtime

## Usage

```
erc6551crunch [OPTIONS]

Required:
  -i, --impl <addr>        Implementation contract address
  -c, --chain <id>         Chain ID (1=Ethereum, 137=Polygon, 8453=Base, ...)
  -n, --nft <addr>         NFT contract address
  -t, --token <id>         Token ID
  -p, --prefix <pattern>   Find addresses starting with pattern
    OR --contains <pattern>

Optional:
  -r, --registry <addr>    Registry address (default: 0x000000006551c19487814612e58FE06813775758)
  -w, --workers <num>      CPU threads (default: all cores)
  -g, --gpu                Use GPU acceleration (OpenCL)
      --list-gpus          List available GPU devices
  -h, --help               Show help
```

## Examples

```shell
# CPU mode
.\target\release\erc6551crunch.exe \
  -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \
  -c 1 \
  -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D \
  -t 1 \
  -p 00000 \
  -w 8

# GPU mode
.\target\release\erc6551crunch.exe \
  -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \
  -c 1 \
  -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D \
  -t 1 \
  -p 00000000 \
  --gpu

# List available GPUs
erc6551crunch --list-gpus
```

## Verify Result

Before creating the account, verify your salt produces the expected address:

1. Go to [ERC6551 Registry - Read Contract](https://etherscan.io/address/0x000000006551c19487814612e58FE06813775758#readContract)
2. Call `account` with:
   - `implementation`: Your implementation address
   - `salt`: The salt from this tool's output
   - `chainId`: Chain ID
   - `tokenContract`: NFT contract address
   - `tokenId`: Token ID
3. Confirm the returned address matches your vanity pattern

## Creating the Account On-Chain

After verifying, create the Token Bound Account by calling the registry:

### Using Etherscan

1. Go to [ERC6551 Registry - Write Contract](https://etherscan.io/address/0x000000006551c19487814612e58FE06813775758#writeContract)
2. Connect your wallet
3. Call `createAccount` with:
   - `implementation`: Your implementation address (e.g., `0x55266d75D1a14E4572138116aF39863Ed6596E7F`)
   - `salt`: The salt from this tool's output
   - `chainId`: Chain ID (e.g., `1` for Ethereum)
   - `tokenContract`: NFT contract address
   - `tokenId`: Token ID

### Using Solidity

```solidity
IERC6551Registry registry = IERC6551Registry(0x000000006551c19487814612e58FE06813775758);

address account = registry.createAccount(
    implementation,  // e.g., 0x55266d75D1a14E4572138116aF39863Ed6596E7F
    salt,            // from this tool
    chainId,         // e.g., 1
    nftContract,     // e.g., 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D
    tokenId          // e.g., 1
);
```

## Performance

| Mode | Speed (approx) |
|------|----------------|
| CPU (8 threads - Intel i7-12700H) | ~5.5M/s |
| GPU (RTX 3050 Ti) | ~160M/s |

## Output

Results saved to `result.txt`:

```
0x... => 0x00000...
```

Format: `salt => address`

## Acknowledgements

- [ERC-6551](https://eips.ethereum.org/EIPS/eip-6551)
- [tokenbound](https://github.com/tokenbound)
- [create2crunch](https://github.com/0age/create2crunch)
