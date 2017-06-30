[Unit]
Description=Watchdog container manager

[Service]
Type=simple
ExecStart=/usr/bin/watchdog
Restart=on-failure

[Install]
WantedBy=multi-user.target