# erc6551crunch

Rust tool for generating vanity ERC6551 tokenbound account addresses.
Supports both **CPU** (multi-threaded) and **GPU** (OpenCL) acceleration.

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
  -r, --registry <addr>    Registry address
  -i, --impl <addr>        Implementation address
  -c, --chain <id>         Chain ID
  -n, --nft <addr>         NFT contract address
  -t, --token <id>         Token ID

Optional:
  -p, --prefix <pattern>   Find addresses starting with pattern
      --contains <pattern> Find addresses containing pattern
  -w, --workers <num>      CPU threads (default: all cores)
  -g, --gpu                Use GPU acceleration (OpenCL)
      --list-gpus          List available GPU devices
  -h, --help               Show help
```

## Examples

```shell
# CPU mode - Find addresses starting with "00000"
.\target\release\erc6551crunch.exe \
  -r 0x000000006551c19487814612e58FE06813775758 \
  -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \
  -c 1 \
  -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D \
  -t 1 \
  -p "00000"

# GPU mode - Much faster! Find addresses starting with "00000000"
.\target\release\erc6551crunch.exe \
  -r 0x000000006551c19487814612e58FE06813775758 \
  -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \
  -c 1 \
  -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D \
  -t 1 \
  -p 00000000 --gpu

# List available GPUs
.\target\release\erc6551crunch.exe --list-gpus
```

## Performance

| Mode | Speed (approx) |
|------|----------------|
| CPU (8 cores) | ~5-10M/s |
| GPU (RTX 3050 Ti) | ~50-100M/s |

## Output

Results saved to `result.txt`:

```
salt: 0x... => init_code_hash: 0x... => address: 0xdead... => pattern: dead
```

## Acknowledgements

- [tokenbound](https://github.com/tokenbound)
- [create2crunch](https://github.com/0age/create2crunch)
