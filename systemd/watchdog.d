[Unit]
Description=Watchdog container manager

[Service]
Type=notify
ExecStart=/usr/bin/watchdog
Restart=on-failure

[Install]
WantedBy=multi-user.target