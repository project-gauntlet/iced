//! Change the apperance of a button.
use iced_core::{Background, Border, Color, Shadow, Vector};

/// The appearance of a button.
#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    /// The amount of offset to apply to the shadow of the button.
    pub shadow_offset: Vector,
    /// The [`Background`] of the button.
    pub background: Option<Background>,
    /// The text [`Color`] of the button.
    pub text_color: Color,
    /// The [`Border`] of the buton.
    pub border: Border,
    /// The [`Shadow`] of the butoon.
    pub shadow: Shadow,
}

impl std::default::Default for Appearance {
    fn default() -> Self {
        Self {
            shadow_offset: Vector::default(),
            background: None,
            text_color: Color::BLACK,
            border: Border::default(),
            shadow: Shadow::default(),
        }
    }
}

/// A set of rules that dictate the style of a button.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default;

    /// Produces the active [`Appearance`] of a button.
    fn active(&self, style: &Self::Style) -> Appearance;

    /// Produces the focused [`Appearance`] of a button.
    fn focused(&self, style: &Self::Style, _is_active: bool) -> Appearance {
        self.hovered(style)
    }

    /// Produces the hovered [`Appearance`] of a button.
    fn hovered(&self, style: &Self::Style) -> Appearance {
        let active = self.active(style);

        Appearance {
            shadow_offset: active.shadow_offset + Vector::new(0.0, 1.0),
            ..active
        }
    }

    /// Produces the pressed [`Appearance`] of a button.
    fn pressed(&self, style: &Self::Style) -> Appearance {
        Appearance {
            shadow_offset: Vector::default(),
            ..self.active(style)
        }
    }

    /// Produces the disabled [`Appearance`] of a button.
    fn disabled(&self, style: &Self::Style) -> Appearance {
        let active = self.active(style);

        Appearance {
            shadow_offset: Vector::default(),
            background: active.background.map(|background| match background {
                Background::Color(color) => Background::Color(Color {
                    a: color.a * 0.5,
                    ..color
                }),
                Background::Gradient(gradient) => {
                    Background::Gradient(gradient.mul_alpha(0.5))
                }
            }),
            text_color: Color {
                a: active.text_color.a * 0.5,
                ..active.text_color
            },
            ..active
        }
    }
}
