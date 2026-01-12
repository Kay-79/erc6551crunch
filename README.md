# erc6551crunch

Rust tool for generating vanity ERC6551 tokenbound account addresses.

## Installation

```shell
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/Kay-79/erc6551crunch.git
cd erc6551crunch
cargo build --release
```

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
  -w, --workers <num>      Number of threads (default: all cores)
  -h, --help               Show help
```

## Examples

```shell
# Find addresses starting with "dead"
.\target\release\erc6551crunch.exe \
  -r 0x000000006551c19487814612e58FE06813775758 \
  -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \
  -c 1 \
  -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D \
  -t 1 \
  -p dead

# Find addresses containing "beef" with 4 threads
.\target\release\erc6551crunch.exe \
  -r 0x000000006551c19487814612e58FE06813775758 \
  -i 0x55266d75D1a14E4572138116aF39863Ed6596E7F \
  -c 1 \
  -n 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D \
  -t 1 \
  --contains beef \
  -w 4
```

## Output

Results saved to `result.txt`:

```
salt: 0x... => init_code_hash: 0x... => address: 0xdead... => pattern: dead
```

## Acknowledgements

- [tokenbound](https://github.com/tokenbound)
- [create2crunch](https://github.com/0age/create2crunch)
