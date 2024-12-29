#!/bin/bash

if [ -f "/etc/systemd/system/turn-server.service" ]; then 
    systemctl stop turn-server
fi

cargo build --release
cp ./target/release/turn-server /usr/local/bin/turn-server

if [ ! -d "/etc/turn-server" ]; then 
    mkdir /etc/turn-server 
fi

if [ ! -f "/etc/turn-server/config.toml" ]; then 
    cp ./turn-server.toml /etc/turn-server/config.toml
fi

if [ ! -f "/etc/systemd/system/turn-server.service" ]; then 
    cp ./turn-server.service /etc/systemd/system/turn-server.service
    systemctl daemon-reload
    systemctl enable turn-server
    systemctl start turn-server
else
    systemctl start turn-server
fi
