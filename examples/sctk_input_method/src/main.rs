use iced::{
    event::{self, wayland::InputMethodEvent},
    keyboard::{KeyCode, Modifiers},
    subscription,
    wayland::{virtual_keyboard::virtual_keyboard_key_press, InitialSurface},
    widget::container,
    window, Application, Color, Command, Element, Event, Subscription, Theme,
};
use iced_style::application;
use std::fmt::Debug;

fn main() {
    let mut settings = iced::Settings::default();
    settings.initial_surface = InitialSurface::None;
    InputMethod::run(settings).unwrap();
}

#[derive(Debug, Clone, Default)]
pub struct InputMethod {}

#[derive(Clone)]
pub enum Message {
    Activate,
    Deactivate,
    KeyPressed {
        key_code: KeyCode,
        modifiers: Modifiers,
    },
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Activate => write!(f, "Message::Activate"),
            Message::Deactivate => write!(f, "Message::Deactivate"),
            Message::KeyPressed {
                key_code,
                modifiers,
            } => todo!(),
        }
    }
}

impl Application for InputMethod {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (InputMethod, Command<Self::Message>) {
        (InputMethod {}, Command::none())
    }

    fn title(&self) -> String {
        String::from("InputMethod")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Activate => {}
            Message::Deactivate => {}
            Message::KeyPressed {
                key_code,
                modifiers,
            } => {
                virtual_keyboard_key_press(key_code.into());
            }
        };
        Command::none()
    }

    fn view(&self, id: window::Id) -> Element<Self::Message> {
        unimplemented!();
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, status| match (event, status) {
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethod(
                        InputMethodEvent::Activate,
                    ),
                )),
                _,
            ) => Some(Message::Activate),
            (
                Event::Keyboard(Event::KeyPressed {
                    key_code,
                    modifiers,
                    ..
                }),
                _,
            ) => Some(Message::KeyPressed {
                key_code,
                modifiers,
            }),
            _ => None,
        })
    }

    fn close_requested(&self, id: window::Id) -> Self::Message {
        unimplemented!()
    }

    fn style(&self) -> <Self::Theme as application::StyleSheet>::Style {
        <Self::Theme as application::StyleSheet>::Style::Custom(Box::new(
            CustomTheme,
        ))
    }
}

pub struct CustomTheme;

impl container::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        container::Appearance {
            border_color: Color::from_rgb(1.0, 0.0, 0.0),
            border_radius: 2.0.into(),
            border_width: 2.0,
            background: Some(Color::from_rgb(1.0, 0.0, 0.0).into()),
            ..container::Appearance::default()
        }
    }
}

impl iced_style::application::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, style: &Self::Style) -> application::Appearance {
        iced_style::application::Appearance {
            background_color: Color::from_rgba(1.0, 0.0, 1.0, 1.0),
            text_color: Color::from_rgb(0.0, 1.0, 0.0),
            icon_color: Color::from_rgb(0.0, 0.0, 1.0),
        }
    }
}
