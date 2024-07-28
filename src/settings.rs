//! Configure your application.
use crate::window;
use crate::{Font, Pixels};

use std::borrow::Cow;

/// The settings of an application.
#[derive(Debug, Clone)]
pub struct Settings<Flags> {
    /// The identifier of the application.
    ///
    /// If provided, this identifier may be used to identify the application or
    /// communicate with it through the windowing system.
    pub id: Option<String>,

    /// The window settings.
    ///
    /// They will be ignored on the Web.
    pub window: window::Settings,

    /// The window settings.
    #[cfg(feature = "wayland")]
    pub initial_surface: iced_sctk::settings::InitialSurface,

    /// The data needed to initialize the [`Application`].
    ///
    /// [`Application`]: crate::Application
    pub flags: Flags,

    /// The fonts to load on boot.
    pub fonts: Vec<Cow<'static, [u8]>>,

    /// The default [`Font`] to be used.
    ///
    /// By default, it uses [`Family::SansSerif`](crate::font::Family::SansSerif).
    pub default_font: Font,

    /// The text size that will be used by default.
    ///
    /// The default value is `16.0`.
    pub default_text_size: Pixels,

    /// If set to true, the renderer will try to perform antialiasing for some
    /// primitives.
    ///
    /// Enabling it can produce a smoother result in some widgets, like the
    /// [`Canvas`], at a performance cost.
    ///
    /// By default, it is disabled.
    ///
    /// [`Canvas`]: crate::widget::Canvas
    pub antialiasing: bool,

    /// If set to true the application will exit when the main window is closed.
    #[cfg(feature = "wayland")]
    pub exit_on_close_request: bool,
}

impl<Flags> Settings<Flags> {
    /// Initialize [`Application`] settings using the given data.
    ///
    /// [`Application`]: crate::Application
    pub fn with_flags(flags: Flags) -> Self {
        let default_settings = Settings::<()>::default();

        Self {
            flags,
            id: default_settings.id,
            window: default_settings.window,
            #[cfg(feature = "wayland")]
            initial_surface: default_settings.initial_surface,
            fonts: default_settings.fonts,
            default_font: default_settings.default_font,
            default_text_size: default_settings.default_text_size,
            antialiasing: default_settings.antialiasing,
            #[cfg(feature = "wayland")]
            exit_on_close_request: default_settings.exit_on_close_request,
        }
    }
}

impl<Flags> Default for Settings<Flags>
where
    Flags: Default,
{
    fn default() -> Self {
        Self {
            id: None,
            window: window::Settings::default(),
            #[cfg(feature = "wayland")]
            initial_surface: iced_sctk::settings::InitialSurface::default(),
            flags: Default::default(),
            fonts: Vec::new(),
            default_font: Font::default(),
            default_text_size: Pixels(16.0),
            antialiasing: false,
            #[cfg(feature = "wayland")]
            exit_on_close_request: true,
        }
    }
}

impl<Flags> From<Settings<Flags>> for iced_winit::Settings<Flags> {
    fn from(settings: Settings<Flags>) -> iced_winit::Settings<Flags> {
        iced_winit::Settings {
            id: settings.id,
            window: settings.window,
            flags: settings.flags,
            fonts: settings.fonts,
        }
    }
}

#[cfg(feature = "wayland")]
impl<Flags> From<Settings<Flags>> for iced_sctk::Settings<Flags> {
    fn from(settings: Settings<Flags>) -> iced_sctk::Settings<Flags> {
        iced_sctk::Settings {
            kbd_repeat: Default::default(),
            surface: settings.initial_surface,
            flags: settings.flags,
            exit_on_close_request: settings.exit_on_close_request,
            ptr_theme: None,
        }
    }
}
