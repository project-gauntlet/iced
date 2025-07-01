//! Platform specific settings for Linux.

/// The platform specific window settings of an application.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlatformSpecific {
    /// Sets the application id of the window.
    ///
    /// As a best practice, it is suggested to select an application id that match
    /// the basename of the application’s .desktop file.
    pub application_id: String,

    /// Whether bypass the window manager mapping for x11 windows
    ///
    /// This flag is particularly useful for creating UI elements that need precise
    /// positioning and immediate display without window manager interference.
    pub override_redirect: bool,

    ///
    pub layer_shell: LayerShellSettings
}

///
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerShellSettings {
    ///
    pub layer: Option<Layer>,
    ///
    pub anchor: Option<Anchor>,
    ///
    pub output: Option<u32>,
    ///
    pub exclusive_zone: Option<i32>,
    /// top, right, bottom, left
    pub margin: Option<(i32, i32, i32, i32)>,
    /// x, y, width, height
    pub input_region: Option<(i32, i32, i32, i32)>,
    ///
    pub keyboard_interactivity: Option<KeyboardInteractivity>,
    ///
    pub namespace: Option<String>,
}

/// The z-depth of a layer.
///
/// These values indicate which order in which layer surfaces are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    ///
    Background,
    ///
    Bottom,
    ///
    Top,
    ///
    Overlay,
}

bitflags::bitflags! {
    /// Specifies which edges and corners a layer should be placed at in the anchor rectangle.
    ///
    /// A combination of two orthogonal edges will cause the layer's anchor point to be the intersection of
    /// the edges. For example [`Anchor::TOP`] and [`Anchor::LEFT`] will result in an anchor point in the top
    /// left of the anchor rectangle.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Anchor: u32 {
        /// Top edge of the anchor rectangle.
        const TOP = 1;

        /// The bottom edge of the anchor rectangle.
        const BOTTOM = 2;

        /// The left edge of the anchor rectangle.
        const LEFT = 4;

        /// The right edge of the anchor rectangle.
        const RIGHT = 8;
    }
}

///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum KeyboardInteractivity {
    /// No keyboard focus is possible.
    ///
    /// This is the default value for all newly created layer shells.
    None,

    /// Request exclusive keyboard focus if the layer is above shell surfaces.
    ///
    /// For [`Layer::Top`] and [`Layer::Overlay`], the seat will always give exclusive access to the layer
    /// which has this interactivity mode set.
    ///
    /// This setting is intended for applications that need to ensure they receive all keyboard events, such
    /// as a lock screen or a password prompt.
    Exclusive,

    /// The compositor should focus and unfocus this surface by the user in an implementation specific manner.
    ///
    /// Compositors may use their normal mechanisms to manage keyboard focus between layers and regular
    /// desktop surfaces.
    ///
    /// This setting is intended for applications which allow keyboard interaction.
    #[default]
    OnDemand,
}
