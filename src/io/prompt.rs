//! Interactive prompts (text, password, confirm, select). Wraps `dialoguer`
//! and falls back to defaults when `non_interactive` is set.

use crate::config::Config;
use anyhow::Result;
use dialoguer::{Confirm, FuzzySelect, Input, Password};

pub struct Prompter {
    pub non_interactive: bool,
}

impl Prompter {
    pub fn new(non_interactive: bool) -> Self {
        Self { non_interactive }
    }

    /// Free-form text input. Returns `default` when non-interactive.
    pub fn text(&self, prompt: &str, default: &str) -> Result<String> {
        if self.non_interactive {
            return Ok(default.to_string());
        }
        let val: String = Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .allow_empty(true)
            .interact_text()?;
        Ok(val.trim().to_string())
    }

    /// Text input that disallows empty or whitespace-only values.
    #[allow(dead_code)]
    pub fn text_required(&self, prompt: &str, default: &str) -> Result<String> {
        if self.non_interactive {
            if default.is_empty() {
                anyhow::bail!("non-interactive mode requires a value for: {prompt}");
            }
            return Ok(default.to_string());
        }
        loop {
            let val: String = Input::new()
                .with_prompt(prompt)
                .default(default.to_string())
                .interact_text()?;
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() && !trimmed.contains(' ') {
                return Ok(trimmed);
            }
            eprintln!("Input cannot be empty or contain spaces!");
        }
    }

    pub fn password(&self, prompt: &str) -> Result<String> {
        if self.non_interactive {
            return Ok(String::new());
        }
        let val = Password::new().with_prompt(prompt).interact()?;
        Ok(val)
    }

    pub fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        if self.non_interactive {
            return Ok(default);
        }
        Ok(Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact()?)
    }

    pub fn select(&self, prompt: &str, items: &[&str], default: usize) -> Result<usize> {
        if self.non_interactive {
            return Ok(default);
        }
        Ok(FuzzySelect::new()
            .with_prompt(prompt)
            .items(items)
            .default(default)
            .interact()?)
    }

    /// Convenience: ask a question and store the answer into the config's
    /// `app` field via a setter closure.
    #[allow(dead_code)]
    pub fn ask_into<F>(&self, prompt: &str, default: &str, setter: F) -> Result<()>
    where
        F: FnOnce(String),
    {
        let val = self.text(prompt, default)?;
        setter(val);
        Ok(())
    }
}

/// Helper that mirrors the PHP `ConsoleTools::question()` loop: keeps
/// prompting until a non-empty, space-free answer is given.
#[allow(dead_code)]
pub fn valid_identifier(prompt: &str, default: &str, non_interactive: bool) -> Result<String> {
    let p = Prompter::new(non_interactive);
    p.text_required(prompt, default)
}

/// Convenience helper for the post-prompt "Configuration Summary" block.
pub fn print_summary(cfg: &Config) {
    let s = crate::io::Style;
    s.section("Configuration Summary");
    println!("  Domain       : {}", cfg.app.hostname);
    println!("  Install Path : {}", cfg.os.ubuntu.install_dir.display());
    println!(
        "  Owner        : {} ({})",
        cfg.app.owner, cfg.app.owner_email
    );
    println!(
        "  Database     : {} ({})",
        cfg.app.db_driver.as_db_connection(),
        cfg.app.db
    );
    println!("  PHP Version  : {}", cfg.unit3d.min_php_version);
    println!("  Echo Port    : {}", cfg.app.echo_port);
    println!();
}
