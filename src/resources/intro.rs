use askama::Template;

/// Startup banner shown by the installer. Mirrors
/// `src/Resources/intro.stub`.
#[derive(Debug, Clone, Template)]
#[template(path = "intro.txt", escape = "none")]
pub struct IntroTemplate;
