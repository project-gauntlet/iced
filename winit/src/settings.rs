//! Configure your application.
use std::borrow::Cow;

/// The settings of an application.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// The identifier of the application.
    ///
    /// If provided, this identifier may be used to identify the application or
    /// communicate with it through the windowing system.
    pub id: Option<String>,

    /// The fonts to load on boot.
    pub fonts: Vec<Cow<'static, [u8]>>,
    ///
    pub platform_specific: PlatformSpecific,
}

///
#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlatformSpecific;

///
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformSpecific {
    ///
    pub activation_policy: ActivationPolicy,
    ///
    pub activate_ignoring_other_apps: bool,
}

#[cfg(target_os = "macos")]
impl Default for PlatformSpecific {
    fn default() -> Self {
        Self {
            activation_policy: ActivationPolicy::default(),
            activate_ignoring_other_apps: true,
        }
    }
}

///
#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivationPolicy {
    ///
    #[default]
    Regular,
    ///
    Accessory,
    ///
    Prohibited,
}
