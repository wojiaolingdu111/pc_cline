SERVER_HOST ?= root@117.78.4.78
SERVER_PASSWORD ?= 581100Li.
BINARY := target/release/ai-toreder-server

.PHONY: build deploy restart clean

build:
	cargo build --release

deploy: build
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "mkdir -p /opt/ai-toreder-server/data"
	sshpass -p "$(SERVER_PASSWORD)" scp -o StrictHostKeyChecking=no $(BINARY) $(SERVER_HOST):/opt/ai-toreder-server/ai-toreder-server
	sshpass -p "$(SERVER_PASSWORD)" scp -o StrictHostKeyChecking=no ai-toreder-server.service $(SERVER_HOST):/etc/systemd/system/ai-toreder-server.service
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "systemctl daemon-reload && systemctl enable ai-toreder-server && systemctl restart ai-toreder-server"
	@echo "=== Done ==="
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "systemctl status ai-toreder-server --no-pager"

restart:
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "systemctl restart ai-toreder-server"
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "systemctl status ai-toreder-server --no-pager"

logs:
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "journalctl -u ai-toreder-server -n 50 --no-pager"

status:
	sshpass -p "$(SERVER_PASSWORD)" ssh -o StrictHostKeyChecking=no $(SERVER_HOST) "systemctl status ai-toreder-server --no-pager"

clean:
	rm -f $(BINARY)
