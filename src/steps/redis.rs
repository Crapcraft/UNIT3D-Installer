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
            format!("chown -R redis:{web_user} /var/run/redis"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use crate::process::Exec;
    use crate::steps::Context;
    use clap::Parser;
    use std::sync::{Arc, Mutex};

    fn recording_ctx(web_user: &str) -> (Context, Arc<Mutex<Vec<String>>>) {
        let args = Args::parse_from(["unit3d-installer", "--non-interactive"]);
        let mut ctx = Context::build(&args).unwrap();
        ctx.config.os.ubuntu.web_user = web_user.to_string();
        let cmds = Arc::new(Mutex::new(Vec::new()));
        let recording = {
            let cmds = cmds.clone();
            struct R(Arc<Mutex<Vec<String>>>);
            impl Exec for R {
                fn run(&self, cmd: &str) -> Result<std::process::Output> {
                    self.0.lock().unwrap().push(cmd.to_string());
                    Ok(std::process::Output {
                        status: std::os::unix::process::ExitStatusExt::from_raw(0),
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    })
                }
            }
            R(cmds)
        };
        ctx.exec = Arc::new(recording);
        (ctx, cmds)
    }

    #[test]
    fn redis_emits_socket_and_memory_config() {
        let (mut ctx, cmds) = recording_ctx("www-data");
        RedisSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.contains("usermod -aG redis www-data"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("unixsocket \\/var\\/run\\/redis\\/redis.sock"))
        );
        assert!(cmds.iter().any(|c| c.contains("maxmemory 256mb")));
        assert!(
            cmds.iter()
                .any(|c| c.contains("maxmemory-policy allkeys-lru"))
        );
        assert!(
            cmds.iter()
                .any(|c| c.contains("systemctl restart redis-server"))
        );
    }

    #[test]
    fn redis_uses_configured_web_user() {
        let (mut ctx, cmds) = recording_ctx("ubuntu");
        RedisSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(cmds.iter().any(|c| c.contains("usermod -aG redis ubuntu")));
    }

    #[test]
    fn redis_socket_dir_is_owned_by_redis_and_web_user_group() {
        let (mut ctx, cmds) = recording_ctx("www-data");
        RedisSetupStep.handle(&mut ctx).unwrap();
        let cmds = cmds.lock().unwrap();
        assert!(
            cmds.iter()
                .any(|c| c.contains("chown -R redis:www-data /var/run/redis"))
        );
        // The group must follow the configured web user, not be hardcoded.
        let (mut ctx2, cmds2) = recording_ctx("ubuntu");
        RedisSetupStep.handle(&mut ctx2).unwrap();
        let cmds2 = cmds2.lock().unwrap();
        assert!(
            cmds2
                .iter()
                .any(|c| c.contains("chown -R redis:ubuntu /var/run/redis"))
        );
    }

    #[test]
    fn redis_requires_prerequisites_ordering() {
        // redis-server (and its `redis` user/group) is installed by
        // PrerequisitesStep; RedisSetupStep must run after it. Assert the
        // ordered catalog reflects that dependency.
        use crate::steps::Steps;
        let ordered = Steps::ordered();
        let names: Vec<&str> = ordered.iter().map(|s| s.name()).collect();
        let redis_idx = names
            .iter()
            .position(|n| *n == "Redis Setup & Configurations")
            .unwrap();
        let prereq_idx = names.iter().position(|n| *n == "Prerequisites").unwrap();
        assert!(
            prereq_idx < redis_idx,
            "redis setup must run after prerequisites (got {prereq_idx} before {redis_idx})"
        );
    }
}
