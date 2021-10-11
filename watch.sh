#/usr/bin/sh


echo "Installing cargo watch if not already installed..."

if ! cargo install -q cargo-watch; then
    echo "Installed successfully"
fi

cargo watch -x check -x run