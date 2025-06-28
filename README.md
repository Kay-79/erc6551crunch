# erc6551crunch

`erc6551crunch` is a [Rust](https://www.rust-lang.org) implementation of the profanity tokenbound account (ERC6551).

## Installation

1. Install Rust

-   ```shell
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2. Install `erc6551crunch`

-   ```shell
    git clone https://github.com/Kay-79/erc6551crunch.git
    ```
-   ```shell
    cd erc6551crunch
    ```

3. Build

-   ```shell
    cargo build --release
    ```
-   Now you can crunch your profanity tokenbound account

## Usage

-   ```Shell
    cargo run --release <registry_address> <implement_address> <chain_id> <nft_address> <token_id>
    ```

-   Example: if you use implementUpgradeable for Tokenbound Account, you can use the following command to crunch the account.

    -   `registry_address`: `0x000000006551c19487814612e58FE06813775758`
    -   `implement_address`: `0x41C8f39463A868d3A88af00cd0fe7102F30E44eC`
    -   `chain_id`: `1` (the chain ID where the NFT contract is deployed)
    -   `nft_address`: `0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D` (Bored Ape Yacht Club)
    -   `token_id`: `1` (BAYC #1)

    ```shell
    cargo run --release 0x000000006551c19487814612e58FE06813775758 0x41C8f39463A868d3A88af00cd0fe7102F30E44eC 1 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D 1
    ```

-   Example: if you use implementProxy for Tokenbound Account, you can use the following command to crunch the account.

    -   `registry_address`: `0x000000006551c19487814612e58FE06813775758`
    -   `implement_proxy_address`: `0x55266d75D1a14E4572138116aF39863Ed6596E7F`
    -   `chain_id`: `1` (the chain ID where the NFT contract is deployed)
    -   `nft_address`: `0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D` (Bored Ape Yacht Club)
    -   `token_id`: `1` (BAYC #1)

    ```shell
    cargo run --release 0x000000006551c19487814612e58FE06813775758 0x55266d75D1a14E4572138116aF39863Ed6596E7F 1 0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D 1
    ```

## Result

-   Check the result in the `result.txt` file

## Contributions

PRs welcome!

## Acknowledgements

-   [`tokenbound`](https://github.com/tokenbound)
-   [`create2crunch`](https://github.com/0age/create2crunch)
