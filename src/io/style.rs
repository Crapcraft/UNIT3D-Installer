//! Pretty-printed banners, headers, success/warning banners.

use owo_colors::OwoColorize;

#[derive(Debug, Clone, Copy, Default)]
pub struct Style;

impl Style {
    pub fn head(&self, text: &str) {
        let bar = "━".repeat(80);
        println!();
        println!("{}", bar.cyan());
        println!("  🚀  {}", text.bold());
        println!("{}", bar.cyan());
        println!();
    }

    pub fn ok(&self) {
        println!("{}", "  ✔  DONE  ".on_green().black().bold());
        println!();
    }

    pub fn warning(&self, msg: &str) {
        println!(
            "{} {}",
            "[Warning]".on_white().yellow().bold(),
            msg.yellow()
        );
    }

    pub fn info(&self, msg: &str) {
        println!("{} {}", "[INFO]".green().bold(), msg);
    }

    #[allow(dead_code)]
    pub fn error(&self, msg: &str) {
        println!("{} {}", "[ERROR]".red().bold(), msg.red());
    }

    pub fn section(&self, title: &str) {
        println!();
        println!("{}", title.blue());
        println!("{}", "─".repeat(80).blue());
    }

    pub fn sep(&self) {
        println!("{}", "=".repeat(80));
    }

    /// Final summary banner printed at the end of a successful run.
    pub fn final_summary(&self, config: &crate::config::Config) {
        use owo_colors::OwoColorize;
        println!();
        println!(
            "{}",
            "================================================================".green()
        );
        println!(
            "{}",
            "             UNIT3D Installation Complete!                      ".green()
        );
        println!(
            "{}",
            "================================================================".green()
        );
        println!();
        println!(
            "Your tracker is available at: {}",
            format!("https://{}", config.app.hostname).cyan()
        );
        println!();
        println!("Login with:");
        println!("  Username: {}", config.app.owner.yellow());
        println!("  Password: {}", config.app.password.yellow());
        println!();
        println!(
            "Credentials saved to: {}",
            "/root/unit3d-credentials.txt".yellow()
        );
        println!();
        println!(
            "{}",
            "IMPORTANT: Save your credentials and delete the file!"
                .red()
                .bold()
        );
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every style method must render without panicking on a default config.
    #[test]
    fn style_methods_render_without_panic() {
        let s = Style;
        let cfg = crate::config::Config::default();
        s.head("Test");
        s.ok();
        s.warning("warn");
        s.info("info");
        s.error("err");
        s.section("sec");
        s.sep();
        s.final_summary(&cfg);
    }

    #[test]
    fn final_summary_with_partial_config() {
        let mut cfg = crate::config::Config::default();
        cfg.app.hostname = "tracker.example.com".to_string();
        cfg.app.owner = "admin".to_string();
        cfg.app.password = "pw".to_string();
        Style.final_summary(&cfg);
    }
}
