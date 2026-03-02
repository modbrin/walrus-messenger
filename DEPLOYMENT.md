# Walrus VPS Deployment

This guide sets up a single-VPS production deployment with:
- Docker Compose (`walrus-server` + `postgres`)
- Nginx on host (TLS termination)
- Security baseline (`ufw`, `fail2ban`, `unattended-upgrades`)
- GitHub Actions CD over SSH using `deploy.sh`

## 0. Important Before Production

Current backend startup applies migrations and ensures origin admin exists.  
On first bootstrap only, set `WALRUS_ORIGIN_PASSWORD`; startup fails if origin user is missing and this env var is not provided.

## 1. Create User and SSH Key Access

On local machine:
```bash
ssh-keygen -t ed25519 -a 100 -C "walrus-prod"
ssh-copy-id root@<VPS_IP>
```

On VPS as `root`:
```bash
adduser walrus
usermod -aG sudo walrus
install -d -m 700 -o walrus -g walrus /home/walrus/.ssh
cp /root/.ssh/authorized_keys /home/walrus/.ssh/authorized_keys
chown walrus:walrus /home/walrus/.ssh/authorized_keys
chmod 600 /home/walrus/.ssh/authorized_keys
```

## 2. Harden SSH

Edit `/etc/ssh/sshd_config.d/00-hardening.conf`:
```bash
sudo nano /etc/ssh/sshd_config.d/00-hardening.conf
```

Paste:
```conf
PermitRootLogin no
PasswordAuthentication no
KbdInteractiveAuthentication no
PubkeyAuthentication yes
AllowUsers walrus
```

Validate and apply:
```bash
sudo sshd -t
sudo systemctl restart ssh
sudo sshd -T -C user=root,host=$(hostname),addr=127.0.0.1 | egrep 'permitrootlogin|allowusers'
```

## 3. Install Runtime and Security Packages

```bash
apt update && apt -y full-upgrade
apt install -y ca-certificates curl gnupg ufw fail2ban unattended-upgrades nginx certbot python3-certbot-nginx
```

Install Docker Engine + Compose plugin (official repo):
```bash
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc
dpkg --print-architecture
. /etc/os-release && echo "$VERSION_CODENAME"
sudo nano /etc/apt/sources.list.d/docker.list
```

Paste this line into `docker.list` (replace placeholders with previous command output):
```text
deb [arch=<arch> signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu <codename> stable
```

Then continue:
```bash
apt update
apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
systemctl enable --now docker
usermod -aG docker walrus
```

## 4. Firewall and Safety Baseline

UFW:
```bash
ufw default deny incoming
ufw default allow outgoing
ufw allow OpenSSH
ufw allow 80/tcp
ufw allow 443/tcp
ufw enable
```

Fail2ban:
```bash
sudo nano /etc/fail2ban/jail.local
```

Paste:
```ini
[DEFAULT]
bantime = 1h
findtime = 10m
maxretry = 5
backend = systemd

[sshd]
enabled = true
```

Enable:
```bash
systemctl enable --now fail2ban
dpkg-reconfigure --priority=low unattended-upgrades
```

Verify fail2ban is active:
```bash
systemctl status fail2ban
fail2ban-client ping
fail2ban-client status
fail2ban-client status sshd
```

What to check:
- `systemctl status fail2ban` shows `active (running)`
- `fail2ban-client ping` returns `Server replied: pong`
- `fail2ban-client status` lists `sshd` in `Jail list`
- `fail2ban-client status sshd` shows counters for `Currently failed` / `Total failed`

## 5. Application Layout

```bash
mkdir -p /opt/walrus
chown -R walrus:walrus /opt/walrus
```

Place these files into `/opt/walrus`:
- `docker-compose.yml` (from this repository)
- `.env` (runtime secrets and image config)

Example `.env`:
```env
WALRUS_IMAGE=ghcr.io/<github-user-or-org>/walrus-server
WALRUS_TAG=latest
POSTGRES_DB=walrus_db
POSTGRES_USER=walrus_app
POSTGRES_PASSWORD=<strong-password>
WALRUS_ORIGIN_PASSWORD=<strong-initial-origin-password>
```
`walrus-server` reads DB credentials from environment variables and is started by
compose with `--address 0.0.0.0:3000`.
`WALRUS_ORIGIN_PASSWORD` is required only for first bootstrap when origin user does not exist.

## 6. Nginx Reverse Proxy + TLS

Edit Nginx site config:
```bash
sudo nano /etc/nginx/sites-available/walrus.conf
```

Paste:
```nginx
server {
    listen 80;
    server_name walrus.<your-domain>;

    # Public and internal paths are the same.
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

Enable + issue cert:
```bash
ln -s /etc/nginx/sites-available/walrus.conf /etc/nginx/sites-enabled/walrus.conf
nginx -t && systemctl reload nginx
certbot --nginx -d walrus.<your-domain> --redirect -m <email> --agree-tos --no-eff-email
```

Client integration note:
- Set API base URL to `https://walrus.<your-domain>`.
- Routes are used directly (`/auth/login`, `/chats`, ...), no extra path prefix needed.

Adding hostnames to certificate later:
- Yes, this is supported. Re-run certbot with all hostnames you want on the cert, for example:
```bash
certbot --nginx -d walrus.<your-domain> -d api.<your-domain> --expand
```

## 7. Deploy from GitHub Actions

Use SSH-based deployment to run `deploy.sh` on VPS.

Required GitHub secrets:
- `PROD_SSH_HOST`
- `PROD_SSH_USER`
- `PROD_SSH_KEY`
- `GHCR_USER` (if image is private)
- `GHCR_TOKEN` (if image is private)

Deploy command over SSH:
```bash
cd /opt/walrus
GHCR_USER="$GHCR_USER" GHCR_TOKEN="$GHCR_TOKEN" ./deploy.sh "$GITHUB_REF_NAME"
```

## 8. First Start / Verification

```bash
cd /opt/walrus
docker compose pull
docker compose up -d
docker compose ps
curl -I http://127.0.0.1:3000/health
curl -I https://walrus.<your-domain>/health
```

HTTP `200` on `/health` confirms app is responding.
