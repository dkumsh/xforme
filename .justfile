# print options
default:
    @just --list --unsorted

# install cargo tools
init:
    cargo upgrade --incompatible
    cargo update

# check code
check:
    cargo check
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features

# automatically fix clippy warnings
fix:
    cargo fmt --all
    cargo clippy --allow-dirty --allow-staged --fix

# build project
build:
   cargo build --all-targets

# execute tests
test:
   cargo test

# run the Portfolio Statement showcase example -> target/portfolio_statement.xlsx
portfolio:
   cargo run --example portfolio_statement

# run the Sales Receipt example -> target/sales_receipt.xlsx
receipt:
   cargo run --example sales_receipt

# run all bundled examples
examples: portfolio receipt
