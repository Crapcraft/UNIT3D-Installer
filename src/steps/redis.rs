//! Configure Redis unix-socket access for ≤1ms IPC (PHP-FPM, queue workers,
//! and Laravel Echo Server) plus RAM-bounded eviction (gap G4 from the
//! v1.2 standalone script). Replaces `src/Installer/Redis/RedisSetup.php`.

use crate::steps::{Context, Step};
use anyhow::Result;

pub struct RedisSetupStep;

impl Step for RedisSetupStep {
    fn name(&self) -> &'static str {
        "Redis Setup & Configurations"
    }

    fn handle(&self, ctx: &mut Context) -> Result<()> {
        let web_user = ctx.config.web_user().to_string();
        ctx.style.info("Configuring Redis unix sockets");

        ctx.run_all([
            "mkdir -p /var/run/redis/".to_string(),
            "chown -R redis:www-data /var/run/redis".to_string(),
            format!("usermod -aG redis {web_user}"),
            "sed -i 's/^# unixsocket /unixsocket /' /etc/redis/redis.conf".to_string(),
            "sed -i 's/^# unixsocketperm /unixsocketperm /' /etc/redis/redis.conf".to_string(),
            "sed -i 's/unixsocket .*/unixsocket \\/var\\/run\\/redis\\/redis.sock/' /etc/redis/redis.conf".to_string(),
            "sed -i 's/unixsocketperm .*/unixsocketperm 770/' /etc/redis/redis.conf".to_string(),
            // G4: cap memory and enable LRU eviction so Redis can never OOM
            // the box.
            "sed -i 's/^# maxmemory .*/maxmemory 256mb/' /etc/redis/redis.conf".to_string(),
            "grep -q '^maxmemory ' /etc/redis/redis.conf || echo 'maxmemory 256mb' >> /etc/redis/redis.conf".to_string(),
            "grep -q '^maxmemory-policy' /etc/redis/redis.conf || echo 'maxmemory-policy allkeys-lru' >> /etc/redis/redis.conf".to_string(),
            "systemctl restart redis-server".to_string(),
        ])?;
        ctx.style.info("Redis configured successfully");
        Ok(())
    }
}
