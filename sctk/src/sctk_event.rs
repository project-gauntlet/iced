use crate::{
    application::SurfaceIdWrapper,
    conversion::{
        modifiers_to_native, pointer_axis_to_native, pointer_button_to_native,
    },
    dpi::PhysicalSize,
    keymap::{self, keysym_to_key},
};

use iced_futures::core::event::{
    wayland::{LayerEvent, PopupEvent, SessionLockEvent},
    PlatformSpecific,
};
use iced_runtime::{
    command::platform_specific::wayland::data_device::DndIcon,
    core::{event::wayland, keyboard, mouse, window, Point},
    keyboard::{key, Key, Location},
};
use sctk::{
    output::OutputInfo,
    reexports::client::{
        backend::ObjectId,
        protocol::{
            wl_data_device_manager::DndAction, wl_keyboard::WlKeyboard,
            wl_output::WlOutput, wl_pointer::WlPointer, wl_seat::WlSeat,
            wl_surface::WlSurface,
        },
        Proxy,
    },
    reexports::csd_frame::WindowManagerCapabilities,
    seat::{
        keyboard::{KeyEvent, Modifiers},
        pointer::{PointerEvent, PointerEventKind},
        Capability,
    },
    session_lock::SessionLockSurfaceConfigure,
    shell::{
        wlr_layer::LayerSurfaceConfigure,
        xdg::{popup::PopupConfigure, window::WindowConfigure},
    },
};
use std::{collections::HashMap, time::Instant};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use xkeysym::Keysym;
use crate::core::window::Id as SurfaceId;

pub enum IcedSctkEvent<T> {
    /// Emitted when new events arrive from the OS to be processed.
    ///
    /// This event type is useful as a place to put code that should be done before you start
    /// processing events, such as updating frame timing information for benchmarking or checking
    /// the [`StartCause`][crate::event::StartCause] to see if a timer set by
    /// [`ControlFlow::WaitUntil`](crate::event_loop::ControlFlow::WaitUntil) has elapsed.
    NewEvents(StartCause),

    /// Any user event from iced
    UserEvent(T),

    /// An event produced by sctk
    SctkEvent(SctkEvent),

    /// Emitted when all of the event loop's input events have been processed and redraw processing
    /// is about to begin.
    ///
    /// This event is useful as a place to put your code that should be run after all
    /// state-changing events have been handled and you want to do stuff (updating state, performing
    /// calculations, etc) that happens as the "main body" of your event loop. If your program only draws
    /// graphics when something changes, it's usually better to do it in response to
    /// [`Event::RedrawRequested`](crate::event::Event::RedrawRequested), which gets emitted
    /// immediately after this event. Programs that draw graphics continuously, like most games,
    /// can render here unconditionally for simplicity.
    MainEventsCleared,

    /// Emitted after [`MainEventsCleared`] when a window should be redrawn.
    ///
    /// This gets triggered in two scenarios:
    /// - The OS has performed an operation that's invalidated the window's contents (such as
    ///   resizing the window).
    /// - The application has explicitly requested a redraw via [`Window::request_redraw`].
    ///
    /// During each iteration of the event loop, Winit will aggregate duplicate redraw requests
    /// into a single event, to help avoid duplicating rendering work.
    ///
    /// Mainly of interest to applications with mostly-static graphics that avoid redrawing unless
    /// something changes, like most non-game GUIs.
    ///
    /// [`MainEventsCleared`]: Self::MainEventsCleared
    RedrawRequested(ObjectId),

    /// Emitted after all [`RedrawRequested`] events have been processed and control flow is about to
    /// be taken away from the program. If there are no `RedrawRequested` events, it is emitted
    /// immediately after `MainEventsCleared`.
    ///
    /// This event is useful for doing any cleanup or bookkeeping work after all the rendering
    /// tasks have been completed.
    ///
    /// [`RedrawRequested`]: Self::RedrawRequested
    RedrawEventsCleared,

    /// Emitted when the event loop is being shut down.
    ///
    /// This is irreversible - if this event is emitted, it is guaranteed to be the last event that
    /// gets emitted. You generally want to treat this as an "do on quit" event.
    LoopDestroyed,

    /// Dnd source created with an icon surface.
    DndSurfaceCreated(WlSurface, DndIcon, SurfaceId),

    /// Frame callback event
    Frame(WlSurface),
}

#[derive(Debug, Clone)]
pub enum SctkEvent {
    //
    // Input events
    //
    SeatEvent {
        variant: SeatEventVariant,
        id: WlSeat,
    },
    PointerEvent {
        variant: PointerEvent,
        ptr_id: WlPointer,
        seat_id: WlSeat,
    },
    KeyboardEvent {
        variant: KeyboardEventVariant,
        kbd_id: WlKeyboard,
        seat_id: WlSeat,
    },
    // TODO data device & touch

    //
    // Surface Events
    //
    WindowEvent {
        variant: WindowEventVariant,
        id: WlSurface,
    },
    LayerSurfaceEvent {
        variant: LayerSurfaceEventVariant,
        id: WlSurface,
    },
    PopupEvent {
        variant: PopupEventVariant,
        /// this may be the Id of a window or layer surface
        toplevel_id: WlSurface,
        /// this may be any SurfaceId
        parent_id: WlSurface,
        /// the id of this popup
        id: WlSurface,
    },

    //
    // output events
    //
    NewOutput {
        id: WlOutput,
        info: Option<OutputInfo>,
    },
    UpdateOutput {
        id: WlOutput,
        info: OutputInfo,
    },
    RemovedOutput(WlOutput),
    //
    // compositor events
    //
    ScaleFactorChanged {
        factor: f64,
        id: WlOutput,
        inner_size: PhysicalSize<u32>,
    },
    DataSource(DataSourceEvent),
    DndOffer {
        event: DndOfferEvent,
        surface: WlSurface,
    },
    /// session lock events
    SessionLocked,
    SessionLockFinished,
    SessionLockSurfaceCreated {
        surface: WlSurface,
        native_id: SurfaceId,
    },
    SessionLockSurfaceConfigure {
        surface: WlSurface,
        configure: SessionLockSurfaceConfigure,
        first: bool,
    },
    SessionUnlocked,
}

#[derive(Debug, Clone)]
pub enum DataSourceEvent {
    /// A DnD action has been accepted by the compositor for your source.
    DndActionAccepted(DndAction),
    /// A DnD mime type has been accepted by a client for your source.
    MimeAccepted(Option<String>),
    /// Dnd Finished event.
    DndFinished,
    /// Dnd Cancelled event.
    DndCancelled,
    /// Dnd Drop performed event.
    DndDropPerformed,
    /// Send the selection data to the clipboard.
    SendSelectionData {
        /// The mime type of the data to be sent
        mime_type: String,
    },
    /// Send the DnD data to the destination.
    SendDndData {
        /// The mime type of the data to be sent
        mime_type: String,
    },
}

#[derive(Debug, Clone)]
pub enum DndOfferEvent {
    /// A DnD offer has been introduced with the given mime types.
    Enter {
        x: f64,
        y: f64,
        mime_types: Vec<String>,
    },
    /// The DnD device has left.
    Leave,
    /// Drag and Drop Motion event.
    Motion {
        /// x coordinate of the pointer
        x: f64,
        /// y coordinate of the pointer
        y: f64,
    },
    /// A drop has been performed.
    DropPerformed,
    /// Read the DnD data
    Data {
        /// The raw data
        data: Vec<u8>,
        /// mime type of the data to read
        mime_type: String,
    },
    SourceActions(DndAction),
    SelectedAction(DndAction),
}

#[derive(Debug, Clone)]
pub enum SeatEventVariant {
    New,
    Remove,
    NewCapability(Capability, ObjectId),
    RemoveCapability(Capability, ObjectId),
}

#[derive(Debug, Clone)]
pub enum KeyboardEventVariant {
    Leave(WlSurface),
    Enter(WlSurface),
    Press(KeyEvent),
    Repeat(KeyEvent),
    Release(KeyEvent),
    Modifiers(Modifiers),
}

#[derive(Debug, Clone)]
pub enum WindowEventVariant {
    Created(ObjectId, SurfaceId),
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:close>
    Close,
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:wm_capabilities>
    WmCapabilities(WindowManagerCapabilities),
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:configure_bounds>
    ConfigureBounds {
        width: u32,
        height: u32,
    },
    /// <https://wayland.app/protocols/xdg-shell#xdg_toplevel:event:configure>
    Configure(WindowConfigure, WlSurface, bool),

    /// window state changed
    StateChanged(sctk::reexports::csd_frame::WindowState),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

#[derive(Debug, Clone)]
pub enum PopupEventVariant {
    /// Popup Created
    Created(ObjectId, SurfaceId),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:popup_done>
    Done,
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:configure>
    Configure(PopupConfigure, WlSurface, bool),
    /// <https://wayland.app/protocols/xdg-shell#xdg_popup:event:repositioned>
    RepositionionedPopup { token: u32 },
    /// size
    Size(u32, u32),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

#[derive(Debug, Clone)]
pub enum LayerSurfaceEventVariant {
    /// sent after creation of the layer surface
    Created(ObjectId, SurfaceId),
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:closed>
    Done,
    /// <https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:event:configure>
    Configure(LayerSurfaceConfigure, WlSurface, bool),
    /// Scale Factor
    ScaleFactorChanged(f64, Option<WpViewport>),
}

/// Describes the reason the event loop is resuming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCause {
    /// Sent if the time specified by [`ControlFlow::WaitUntil`] has been reached. Contains the
    /// moment the timeout was requested and the requested resume time. The actual resume time is
    /// guaranteed to be equal to or after the requested resume time.
    ///
    /// [`ControlFlow::WaitUntil`]: crate::event_loop::ControlFlow::WaitUntil
    ResumeTimeReached {
        start: Instant,
        requested_resume: Instant,
    },

    /// Sent if the OS has new events to send to the window, after a wait was requested. Contains
    /// the moment the wait was requested and the resume time, if requested.
    WaitCancelled {
        start: Instant,
        requested_resume: Option<Instant>,
    },

    /// Sent if the event loop is being resumed after the loop's control flow was set to
    /// [`ControlFlow::Poll`].
    ///
    /// [`ControlFlow::Poll`]: crate::event_loop::ControlFlow::Poll
    Poll,

    /// Sent once, immediately after `run` is called. Indicates that the loop was just initialized.
    Init,
}

/// Pending update to a window requested by the user.
#[derive(Default, Debug, Clone, Copy)]
pub struct SurfaceUserRequest {
    /// Whether `redraw` was requested.
    pub redraw_requested: bool,

    /// Wether the frame should be refreshed.
    pub refresh_frame: bool,
}

// The window update coming from the compositor.
#[derive(Default, Debug, Clone)]
pub struct SurfaceCompositorUpdate {
    /// New window configure.
    pub configure: Option<WindowConfigure>,

    /// New scale factor.
    pub scale_factor: Option<i32>,
}

impl SctkEvent {
    pub fn to_native(
        self,
        modifiers: &mut Modifiers,
        surface_ids: &HashMap<ObjectId, SurfaceIdWrapper>,
        destroyed_surface_ids: &HashMap<ObjectId, SurfaceIdWrapper>,
    ) -> Vec<iced_runtime::core::Event> {
        match self {
            // TODO Ashley: Platform specific multi-seat events?
            SctkEvent::SeatEvent { .. } => Default::default(),
            SctkEvent::PointerEvent { variant, .. } => match variant.kind {
                PointerEventKind::Enter { .. } => {
                    vec![iced_runtime::core::Event::Mouse(
                        mouse::Event::CursorEntered,
                    )]
                }
                PointerEventKind::Leave { .. } => {
                    vec![iced_runtime::core::Event::Mouse(
                        mouse::Event::CursorLeft,
                    )]
                }
                PointerEventKind::Motion { .. } => {
                    vec![iced_runtime::core::Event::Mouse(
                        mouse::Event::CursorMoved {
                            position: Point::new(
                                variant.position.0 as f32,
                                variant.position.1 as f32,
                            ),
                        },
                    )]
                }
                PointerEventKind::Press {
                    time: _,
                    button,
                    serial: _,
                } => pointer_button_to_native(button)
                    .map(|b| {
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::ButtonPressed(b),
                        )
                    })
                    .into_iter()
                    .collect(), // TODO Ashley: conversion
                PointerEventKind::Release {
                    time: _,
                    button,
                    serial: _,
                } => pointer_button_to_native(button)
                    .map(|b| {
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::ButtonReleased(b),
                        )
                    })
                    .into_iter()
                    .collect(), // TODO Ashley: conversion
                PointerEventKind::Axis {
                    time: _,
                    horizontal,
                    vertical,
                    source,
                } => pointer_axis_to_native(source, horizontal, vertical)
                    .map(|a| {
                        iced_runtime::core::Event::Mouse(
                            mouse::Event::WheelScrolled { delta: a },
                        )
                    })
                    .into_iter()
                    .collect(), // TODO Ashley: conversion
            },
            SctkEvent::KeyboardEvent {
                variant,
                kbd_id: _,
                seat_id,
            } => match variant {
                KeyboardEventVariant::Leave(surface) => surface_ids
                    .get(&surface.id())
                    .and_then(|id| match id {
                        SurfaceIdWrapper::LayerSurface(_id) => {
                            Some(iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::Layer(
                                        LayerEvent::Unfocused,
                                        surface,
                                        id.inner(),
                                    ),
                                ),
                            ))
                        }
                        SurfaceIdWrapper::Window(id) => {
                            Some(iced_runtime::core::Event::Window(
                                *id,
                                window::Event::Unfocused,
                            ))
                        }
                        SurfaceIdWrapper::Popup(_id) => {
                            Some(iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::Popup(
                                        PopupEvent::Unfocused,
                                        surface,
                                        id.inner(),
                                    ),
                                ),
                            ))
                        }
                        SurfaceIdWrapper::Dnd(_) => None,
                        SurfaceIdWrapper::SessionLock(_) => {
                            Some(iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::SessionLock(
                                        SessionLockEvent::Unfocused(
                                            surface,
                                            id.inner(),
                                        ),
                                    ),
                                ),
                            ))
                        }
                    })
                    .into_iter()
                    .chain([iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::Seat(
                            wayland::SeatEvent::Leave,
                            seat_id,
                        )),
                    )])
                    .collect(),
                KeyboardEventVariant::Enter(surface) => surface_ids
                    .get(&surface.id())
                    .and_then(|id| match id {
                        SurfaceIdWrapper::LayerSurface(_id) => {
                            Some(iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::Layer(
                                        LayerEvent::Focused,
                                        surface,
                                        id.inner(),
                                    ),
                                ),
                            ))
                        }
                        SurfaceIdWrapper::Window(id) => {
                            Some(iced_runtime::core::Event::Window(
                                *id,
                                window::Event::Focused,
                            ))
                        }
                        SurfaceIdWrapper::Popup(_id) => {
                            Some(iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::Popup(
                                        PopupEvent::Focused,
                                        surface,
                                        id.inner(),
                                    ),
                                ),
                            ))
                        }
                        SurfaceIdWrapper::Dnd(_) => None,
                        SurfaceIdWrapper::SessionLock(_) => {
                            Some(iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::SessionLock(
                                        SessionLockEvent::Focused(
                                            surface,
                                            id.inner(),
                                        ),
                                    ),
                                ),
                            ))
                        }
                    })
                    .into_iter()
                    .chain([iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::Seat(
                            wayland::SeatEvent::Enter,
                            seat_id,
                        )),
                    )])
                    .collect(),
                KeyboardEventVariant::Press(ke) => {
                    let (key, location) = keysym_to_vkey_location(
                        ke.keysym.raw(),
                        ke.utf8.as_deref(),
                    );
                    Some(iced_runtime::core::Event::Keyboard(
                        keyboard::Event::KeyPressed {
                            key: key,
                            location: location,
                            physical_key: physical_key(ke.raw_code),
                            text: ke.utf8.map(|s| s.into()),
                            modifiers: modifiers_to_native(*modifiers),
                        },
                    ))
                    .into_iter()
                    .collect()
                }
                KeyboardEventVariant::Repeat(KeyEvent {
                    raw_code,
                    utf8,
                    ..
                }) => {
                    let (key, location) =
                        keysym_to_vkey_location(raw_code, utf8.as_deref());
                    Some(iced_runtime::core::Event::Keyboard(
                        keyboard::Event::KeyPressed {
                            key: key,
                            location: location,
                            physical_key: physical_key(raw_code),
                            text: utf8.map(|s| s.into()),
                            modifiers: modifiers_to_native(*modifiers),
                        },
                    ))
                    .into_iter()
                    .collect()
                }
                KeyboardEventVariant::Release(ke) => {
                    let (k, location) = keysym_to_vkey_location(
                        ke.keysym.raw(),
                        ke.utf8.as_deref(),
                    );
                    Some(iced_runtime::core::Event::Keyboard(
                        keyboard::Event::KeyReleased {
                            key: k,
                            location: location,
                            physical_key: physical_key(ke.raw_code),
                            modifiers: modifiers_to_native(*modifiers),
                        },
                    ))
                    .into_iter()
                    .collect()
                }
                KeyboardEventVariant::Modifiers(new_mods) => {
                    *modifiers = new_mods;
                    vec![iced_runtime::core::Event::Keyboard(
                        keyboard::Event::ModifiersChanged(modifiers_to_native(
                            new_mods,
                        )),
                    )]
                }
            },
            SctkEvent::WindowEvent {
                variant,
                id: surface,
            } => match variant {
                // TODO Ashley: platform specific events for window
                WindowEventVariant::Created(..) => Default::default(),
                WindowEventVariant::Close => destroyed_surface_ids
                    .get(&surface.id())
                    .map(|id| {
                        iced_runtime::core::Event::Window(
                            id.inner(),
                            window::Event::CloseRequested,
                        )
                    })
                    .into_iter()
                    .collect(),
                WindowEventVariant::WmCapabilities(caps) => surface_ids
                    .get(&surface.id())
                    .map(|id| id.inner())
                    .map(|id| {
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(wayland::Event::Window(
                                wayland::WindowEvent::WmCapabilities(caps),
                                surface,
                                id,
                            )),
                        )
                    })
                    .into_iter()
                    .collect(),
                WindowEventVariant::ConfigureBounds { .. } => {
                    Default::default()
                }
                WindowEventVariant::Configure(configure, surface, _) => {
                    if configure.is_resizing() {
                        surface_ids
                            .get(&surface.id())
                            .map(|id| {
                                iced_runtime::core::Event::Window(
                                    id.inner(),
                                    window::Event::Resized {
                                        width: configure
                                            .new_size
                                            .0
                                            .unwrap()
                                            .get(),
                                        height: configure
                                            .new_size
                                            .1
                                            .unwrap()
                                            .get(),
                                    },
                                )
                            })
                            .into_iter()
                            .collect()
                    } else {
                        Default::default()
                    }
                }
                WindowEventVariant::ScaleFactorChanged(..) => {
                    Default::default()
                }
                WindowEventVariant::StateChanged(s) => surface_ids
                    .get(&surface.id())
                    .map(|id| {
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(wayland::Event::Window(
                                wayland::WindowEvent::State(s),
                                surface,
                                id.inner(),
                            )),
                        )
                    })
                    .into_iter()
                    .collect(),
            },
            SctkEvent::LayerSurfaceEvent {
                variant,
                id: surface,
            } => match variant {
                LayerSurfaceEventVariant::Done => destroyed_surface_ids
                    .get(&surface.id())
                    .map(|id| {
                        iced_runtime::core::Event::PlatformSpecific(
                            PlatformSpecific::Wayland(wayland::Event::Layer(
                                LayerEvent::Done,
                                surface,
                                id.inner(),
                            )),
                        )
                    })
                    .into_iter()
                    .collect(),
                _ => Default::default(),
            },
            SctkEvent::PopupEvent {
                variant,
                id: surface,
                ..
            } => {
                match variant {
                    PopupEventVariant::Done => destroyed_surface_ids
                        .get(&surface.id())
                        .map(|id| {
                            iced_runtime::core::Event::PlatformSpecific(
                                PlatformSpecific::Wayland(
                                    wayland::Event::Popup(
                                        PopupEvent::Done,
                                        surface,
                                        id.inner(),
                                    ),
                                ),
                            )
                        })
                        .into_iter()
                        .collect(),
                    PopupEventVariant::Created(_, _) => Default::default(), // TODO
                    PopupEventVariant::Configure(_, _, _) => Default::default(), // TODO
                    PopupEventVariant::RepositionionedPopup { token: _ } => {
                        Default::default()
                    }
                    PopupEventVariant::Size(_, _) => Default::default(),
                    PopupEventVariant::ScaleFactorChanged(..) => {
                        Default::default()
                    } // TODO
                }
            }
            SctkEvent::NewOutput { id, info } => {
                Some(iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::Output(
                        wayland::OutputEvent::Created(info),
                        id,
                    )),
                ))
                .into_iter()
                .collect()
            }
            SctkEvent::UpdateOutput { id, info } => {
                vec![iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::Output(
                        wayland::OutputEvent::InfoUpdate(info),
                        id,
                    )),
                )]
            }
            SctkEvent::RemovedOutput(id) => {
                Some(iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::Output(
                        wayland::OutputEvent::Removed,
                        id,
                    )),
                ))
                .into_iter()
                .collect()
            }
            SctkEvent::ScaleFactorChanged {
                factor: _,
                id: _,
                inner_size: _,
            } => Default::default(),
            SctkEvent::DndOffer { event, .. } => match event {
                DndOfferEvent::Enter { mime_types, x, y } => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::Enter { mime_types, x, y },
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DndOfferEvent::Motion { x, y } => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::Motion { x, y },
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DndOfferEvent::DropPerformed => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::DropPerformed,
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DndOfferEvent::Leave => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::Leave,
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DndOfferEvent::Data { mime_type, data } => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::DndData { data, mime_type },
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DndOfferEvent::SourceActions(actions) => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::SourceActions(actions),
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DndOfferEvent::SelectedAction(action) => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DndOffer(
                            wayland::DndOfferEvent::SelectedAction(action),
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
            },
            SctkEvent::DataSource(event) => match event {
                DataSourceEvent::DndDropPerformed => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::DndDropPerformed,
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DataSourceEvent::DndFinished => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::DndFinished,
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DataSourceEvent::DndCancelled => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::Cancelled,
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DataSourceEvent::MimeAccepted(mime_type) => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::MimeAccepted(mime_type),
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DataSourceEvent::DndActionAccepted(action) => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::DndActionAccepted(action),
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DataSourceEvent::SendDndData { mime_type } => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::SendDndData(mime_type),
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
                DataSourceEvent::SendSelectionData { mime_type } => {
                    Some(iced_runtime::core::Event::PlatformSpecific(
                        PlatformSpecific::Wayland(wayland::Event::DataSource(
                            wayland::DataSourceEvent::SendSelectionData(
                                mime_type,
                            ),
                        )),
                    ))
                    .into_iter()
                    .collect()
                }
            },
            SctkEvent::SessionLocked => {
                Some(iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::SessionLock(
                        wayland::SessionLockEvent::Locked,
                    )),
                ))
                .into_iter()
                .collect()
            }
            SctkEvent::SessionLockFinished => {
                Some(iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::SessionLock(
                        wayland::SessionLockEvent::Finished,
                    )),
                ))
                .into_iter()
                .collect()
            }
            SctkEvent::SessionLockSurfaceCreated { .. } => vec![],
            SctkEvent::SessionLockSurfaceConfigure { .. } => vec![],
            SctkEvent::SessionUnlocked => {
                Some(iced_runtime::core::Event::PlatformSpecific(
                    PlatformSpecific::Wayland(wayland::Event::SessionLock(
                        wayland::SessionLockEvent::Unlocked,
                    )),
                ))
                .into_iter()
                .collect()
            }
        }
    }
}

fn keysym_to_vkey_location(keysym: u32, utf8: Option<&str>) -> (Key, Location) {
    let mut key = keysym_to_key(keysym);
    if matches!(key, key::Key::Unidentified) {
        if let Some(utf8) = utf8 {
            key = Key::Character(utf8.into());
        }
    }

    let location = keymap::keysym_location(keysym);
    (key, location)
}

// may this help you
// https://github.com/qemu/keycodemapdb/blob/master/data/keymaps.csv
pub fn physical_key(rawcode: u32) -> keyboard::PhysicalKey {
    use keyboard::PhysicalKey;

    // note: 8 is not added to this rawcode, unlike is done for wayland and x11  
    match rawcode {
        1 => PhysicalKey::Escape,
        2 => PhysicalKey::Digit1,
        3 => PhysicalKey::Digit2,
        4 => PhysicalKey::Digit3,
        5 => PhysicalKey::Digit4,
        6 => PhysicalKey::Digit5,
        7 => PhysicalKey::Digit6,
        8 => PhysicalKey::Digit7,
        9 => PhysicalKey::Digit8,
        10 => PhysicalKey::Digit9,
        11 => PhysicalKey::Digit0,
        12 => PhysicalKey::Minus,
        13 => PhysicalKey::Equal,
        14 => PhysicalKey::Backspace,
        15 => PhysicalKey::Tab,
        16 => PhysicalKey::KeyQ,
        17 => PhysicalKey::KeyW,
        18 => PhysicalKey::KeyE,
        19 => PhysicalKey::KeyR,
        20 => PhysicalKey::KeyT,
        21 => PhysicalKey::KeyY,
        22 => PhysicalKey::KeyU,
        23 => PhysicalKey::KeyI,
        24 => PhysicalKey::KeyO,
        25 => PhysicalKey::KeyP,
        26 => PhysicalKey::BracketLeft,
        27 => PhysicalKey::BracketRight,
        28 => PhysicalKey::Enter,
        29 => PhysicalKey::ControlLeft,
        30 => PhysicalKey::KeyA,
        31 => PhysicalKey::KeyS,
        32 => PhysicalKey::KeyD,
        33 => PhysicalKey::KeyF,
        34 => PhysicalKey::KeyG,
        35 => PhysicalKey::KeyH,
        36 => PhysicalKey::KeyJ,
        37 => PhysicalKey::KeyK,
        38 => PhysicalKey::KeyL,
        39 => PhysicalKey::Semicolon,
        40 => PhysicalKey::Quote,
        41 => PhysicalKey::Backquote,
        42 => PhysicalKey::ShiftLeft,
        43 => PhysicalKey::Backslash,
        44 => PhysicalKey::KeyZ,
        45 => PhysicalKey::KeyX,
        46 => PhysicalKey::KeyC,
        47 => PhysicalKey::KeyV,
        48 => PhysicalKey::KeyB,
        49 => PhysicalKey::KeyN,
        50 => PhysicalKey::KeyM,
        51 => PhysicalKey::Comma,
        52 => PhysicalKey::Period,
        53 => PhysicalKey::Slash,
        54 => PhysicalKey::ShiftRight,
        55 => PhysicalKey::NumpadMultiply,
        56 => PhysicalKey::AltLeft,
        57 => PhysicalKey::Space,
        58 => PhysicalKey::CapsLock,
        59 => PhysicalKey::F1,
        60 => PhysicalKey::F2,
        61 => PhysicalKey::F3,
        62 => PhysicalKey::F4,
        63 => PhysicalKey::F5,
        64 => PhysicalKey::F6,
        65 => PhysicalKey::F7,
        66 => PhysicalKey::F8,
        67 => PhysicalKey::F9,
        68 => PhysicalKey::F10,
        69 => PhysicalKey::NumLock,
        70 => PhysicalKey::ScrollLock,
        71 => PhysicalKey::Numpad7,
        72 => PhysicalKey::Numpad8,
        73 => PhysicalKey::Numpad9,
        74 => PhysicalKey::NumpadSubtract,
        75 => PhysicalKey::Numpad4,
        76 => PhysicalKey::Numpad5,
        77 => PhysicalKey::Numpad6,
        78 => PhysicalKey::NumpadAdd,
        79 => PhysicalKey::Numpad1,
        80 => PhysicalKey::Numpad2,
        81 => PhysicalKey::Numpad3,
        82 => PhysicalKey::Numpad0,
        83 => PhysicalKey::NumpadDecimal,
        85 => PhysicalKey::Lang5,
        86 => PhysicalKey::IntlBackslash,
        87 => PhysicalKey::F11,
        88 => PhysicalKey::F12,
        89 => PhysicalKey::IntlRo,
        90 => PhysicalKey::Katakana, // or Lang3
        91 => PhysicalKey::Hiragana, // or Lang4
        92 => PhysicalKey::Convert,
        93 => PhysicalKey::KanaMode,
        94 => PhysicalKey::NonConvert,
        // 95 => PhysicalKey::KEY_KPJPCOMMA,
        96 => PhysicalKey::NumpadEnter,
        97 => PhysicalKey::ControlRight,
        98 => PhysicalKey::NumpadDivide,
        99 => PhysicalKey::PrintScreen,
        100 => PhysicalKey::AltRight,
        // 101 => PhysicalKey::KEY_LINEFEED,
        102 => PhysicalKey::Home,
        103 => PhysicalKey::ArrowUp,
        104 => PhysicalKey::PageUp,
        105 => PhysicalKey::ArrowLeft,
        106 => PhysicalKey::ArrowRight,
        107 => PhysicalKey::End,
        108 => PhysicalKey::ArrowDown,
        109 => PhysicalKey::PageDown,
        110 => PhysicalKey::Insert,
        111 => PhysicalKey::Delete,
        // 112 => PhysicalKey::KEY_MACRO,
        113 => PhysicalKey::AudioVolumeMute,
        114 => PhysicalKey::AudioVolumeDown,
        115 => PhysicalKey::AudioVolumeUp,
        116 => PhysicalKey::Power,
        117 => PhysicalKey::NumpadEqual,
        // 118 => PhysicalKey::KEY_KPPLUSMINUS,
        119 => PhysicalKey::Pause,
        // 120 => PhysicalKey::KEY_SCALE,
        121 => PhysicalKey::NumpadComma,
        122 => PhysicalKey::Lang1,
        123 => PhysicalKey::Lang2,
        124 => PhysicalKey::IntlYen,
        125 => PhysicalKey::SuperLeft, // technically this should be MetaLeft but winit uses SuperLeft instead
        126 => PhysicalKey::SuperRight, // technically this should be MetaRight but winit uses SuperRight instead
        127 => PhysicalKey::ContextMenu,
        128 => PhysicalKey::BrowserStop,
        129 => PhysicalKey::Again,
        130 => PhysicalKey::Props,
        131 => PhysicalKey::Undo,
        // 132 => PhysicalKey::KEY_FRONT,
        133 => PhysicalKey::Copy,
        134 => PhysicalKey::Open,
        135 => PhysicalKey::Paste,
        136 => PhysicalKey::Find,
        137 => PhysicalKey::Cut,
        138 => PhysicalKey::Help,
        // 139 => PhysicalKey::KEY_MENU,
        140 => PhysicalKey::LaunchApp2,
        // 141 => PhysicalKey::KEY_SETUP,
        142 => PhysicalKey::Sleep,
        143 => PhysicalKey::WakeUp,
        // 144 => PhysicalKey::KEY_FILE,
        // 145 => PhysicalKey::KEY_SENDFILE,
        // 146 => PhysicalKey::KEY_DELETEFILE,
        // 147 => PhysicalKey::KEY_XFER,
        // 148 => PhysicalKey::KEY_PROG1,
        // 149 => PhysicalKey::KEY_PROG2,
        // 150 => PhysicalKey::KEY_WWW,
        // 151 => PhysicalKey::KEY_MSDOS,
        // 152 => PhysicalKey::KEY_COFFEE,
        // 153 => PhysicalKey::KEY_ROTATE_DISPLAY,
        // 154 => PhysicalKey::KEY_CYCLEWINDOWS,
        155 => PhysicalKey::LaunchMail,
        156 => PhysicalKey::BrowserFavorites,
        157 => PhysicalKey::LaunchApp1,
        158 => PhysicalKey::BrowserBack,
        159 => PhysicalKey::BrowserForward,
        // 160 => PhysicalKey::KEY_CLOSECD,
        // 161 => PhysicalKey::KEY_EJECTCD,
        162 => PhysicalKey::Eject,
        163 => PhysicalKey::MediaTrackNext,
        164 => PhysicalKey::MediaPlayPause,
        165 => PhysicalKey::MediaTrackPrevious,
        166 => PhysicalKey::MediaStop,
        // 167 => PhysicalKey::KEY_RECORD,
        // 168 => PhysicalKey::KEY_REWIND,
        // 169 => PhysicalKey::KEY_PHONE,
        // 170 => PhysicalKey::KEY_ISO,
        // 171 => PhysicalKey::KEY_CONFIG,
        172 => PhysicalKey::BrowserHome,
        173 => PhysicalKey::BrowserRefresh,
        // 174 => PhysicalKey::KEY_EXIT,
        // 175 => PhysicalKey::KEY_MOVE,
        // 176 => PhysicalKey::KEY_EDIT,
        // 177 => PhysicalKey::KEY_SCROLLUP,
        // 178 => PhysicalKey::KEY_SCROLLDOWN,
        179 => PhysicalKey::NumpadParenLeft,
        180 => PhysicalKey::NumpadParenRight,
        // 181 => PhysicalKey::KEY_NEW,
        // 182 => PhysicalKey::KEY_REDO,
        183 => PhysicalKey::F13,
        184 => PhysicalKey::F14,
        185 => PhysicalKey::F15,
        186 => PhysicalKey::F16,
        187 => PhysicalKey::F17,
        188 => PhysicalKey::F18,
        189 => PhysicalKey::F19,
        190 => PhysicalKey::F20,
        191 => PhysicalKey::F21,
        192 => PhysicalKey::F22,
        193 => PhysicalKey::F23,
        194 => PhysicalKey::F24,
        // 200 => PhysicalKey::KEY_PLAYCD,
        // 201 => PhysicalKey::KEY_PAUSECD,
        // 202 => PhysicalKey::KEY_PROG3,
        // 203 => PhysicalKey::KEY_PROG4,
        // 204 => PhysicalKey::KEY_ALL_APPLICATIONS,
        205 => PhysicalKey::Suspend,
        // 206 => PhysicalKey::KEY_CLOSE,
        // 207 => PhysicalKey::KEY_PLAY,
        // 208 => PhysicalKey::KEY_FASTFORWARD,
        // 209 => PhysicalKey::KEY_BASSBOOST,
        // 210 => PhysicalKey::KEY_PRINT,
        // 211 => PhysicalKey::KEY_HP,
        // 212 => PhysicalKey::KEY_CAMERA,
        // 213 => PhysicalKey::KEY_SOUND,
        // 214 => PhysicalKey::KEY_QUESTION,
        // 215 => PhysicalKey::KEY_EMAIL,
        // 216 => PhysicalKey::KEY_CHAT,
        217 => PhysicalKey::BrowserSearch,
        // 218 => PhysicalKey::KEY_CONNECT,
        // 219 => PhysicalKey::KEY_FINANCE,
        // 220 => PhysicalKey::KEY_SPORT,
        // 221 => PhysicalKey::KEY_SHOP,
        // 222 => PhysicalKey::KEY_ALTERASE,
        // 223 => PhysicalKey::KEY_CANCEL,
        // 224 => PhysicalKey::KEY_BRIGHTNESSDOWN,
        // 225 => PhysicalKey::KEY_BRIGHTNESSUP,
        226 => PhysicalKey::MediaSelect,
        // 227 => PhysicalKey::KEY_SWITCHVIDEOMODE,
        // 228 => PhysicalKey::KEY_KBDILLUMTOGGLE,
        // 229 => PhysicalKey::KEY_KBDILLUMDOWN,
        // 230 => PhysicalKey::KEY_KBDILLUMUP,
        // 231 => PhysicalKey::KEY_SEND,
        // 232 => PhysicalKey::KEY_REPLY,
        // 233 => PhysicalKey::KEY_FORWARDMAIL,
        // 234 => PhysicalKey::KEY_SAVE,
        // 235 => PhysicalKey::KEY_DOCUMENTS,
        // 236 => PhysicalKey::KEY_BATTERY,
        // 237 => PhysicalKey::KEY_BLUETOOTH,
        // 238 => PhysicalKey::KEY_WLAN,
        // 239 => PhysicalKey::KEY_UWB,
        // 240 => PhysicalKey::KEY_UNKNOWN,
        // 241 => PhysicalKey::KEY_VIDEO_NEXT,
        // 242 => PhysicalKey::KEY_VIDEO_PREV,
        // 243 => PhysicalKey::KEY_BRIGHTNESS_CYCLE,
        // 244 => PhysicalKey::KEY_BRIGHTNESS_AUTO,
        // 245 => PhysicalKey::KEY_DISPLAY_OFF,
        // 246 => PhysicalKey::KEY_WWAN,
        // 247 => PhysicalKey::KEY_RFKILL,
        // 248 => PhysicalKey::KEY_MICMUTE,
        _ => PhysicalKey::Unidentified,
    }
}

