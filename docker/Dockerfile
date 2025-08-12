FROM ubuntu:noble

RUN export DEBIAN_FRONTEND=noninteractive && apt update && apt upgrade -y && apt install -y \
    libgtk-4-* build-essential pkg-config libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev rustup && rustup -y install
