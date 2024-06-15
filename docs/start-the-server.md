# Start the server

turn-server has only one command line parameter `--config`, which is used to run the server by specifying a configuration file. The detailed information of the configuration file can be found in [configure](./configure.md).

```bash
turn-server --config ./turn-server.toml
```

Starting the service is that simple.


### Linux service

If you need to run turn-rs as a systemd service, first, create a service description file:

```bash
vim /etc/systemd/system/turn-server.service
```

Enter the following content in the service description file:

```ini
[Unit]
Description=A pure rust-implemented turn server.
After=network.target

[Service]
Type=simple
Restart=always
ExecStart=/usr/local/bin/turn-server --config=/etc/turn-server/config.toml

[Install]
WantedBy=multi-user.target
```

`ExecStart` can be adjusted according to your actual situation, but it is recommended to place the corresponding file in the location of the above example.

Next, set the service to start automatically by default and start the service:

```bash
systemctl daemon-reload
systemctl enable turn-server
systemctl start turn-server
```

You can use `systemctl status turn-server` to view the startup status of the service.
