#!/usr/bin/env bash

sudo apt update
# Test on Ubuntu 24.04
sudo apt install -y \
  gcc g++ clang libfontconfig-dev libwayland-dev \
  libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libssl-dev libzstd-dev libasound2-dev \
  vulkan-validationlayers libvulkan1
