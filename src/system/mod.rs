//! System-introspection helpers: OS detection, privileges, memory, and
//! network facts. Rust equivalent of the legacy `src/Helpers/helpers.php`.

pub mod mem;
pub mod network;
pub mod os_detect;
pub mod privileges;

#[allow(unused_imports)]
pub use mem::memory;
#[allow(unused_imports)]
pub use network::{fqdn, hostname, ip};
pub use os_detect::detect;
pub use privileges::require_root;
