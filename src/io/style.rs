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
