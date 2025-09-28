if ! [ "$1" = "" ]; then
    TASM_DIR="$1" cargo build --release
    echo "Installed with TASM_DIR="$1""
else 
    cargo build --release
fi
cp target/release/tasm-runner $HOME/.local/bin/