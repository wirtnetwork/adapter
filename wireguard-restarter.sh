#! /bin/sh
while inotifywait -e close_write /etc/wireguard/server.conf; do systemctl restart wg-quick@server; done
