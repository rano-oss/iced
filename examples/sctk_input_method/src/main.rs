use env_logger::fmt::Style;
use iced::{
    event::{
        self,
        wayland::{InputMethodEvent, InputMethodKeyboardEvent, KeyEvent, RawModifiers, Modifiers},
    },
    subscription,
    wayland::{
        actions::{virtual_keyboard::ActionInner, input_method_popup::InputMethodPopupSettings},
        virtual_keyboard::virtual_keyboard_action, InitialSurface, input_method::{show_input_method_popup, hide_input_method_popup},
    },
    widget::{container, button, row, text, column, combo_box},
    window, Application, Color, Command, Element, Event, Subscription, Theme,
    Settings, alignment::{Vertical, Horizontal}, Length, Alignment, Padding,
};
use iced_style::application;
use std::fmt::Debug;

fn main() -> iced::Result {
    let initial_surface = InputMethodPopupSettings::default();
    let settings = Settings {
        initial_surface: InitialSurface::InputMethodPopup(initial_surface),
        ..Settings::default()
    };
    InputMethod::run(settings)
}

#[derive(Debug, Clone, Default)]
pub struct InputMethod {}

#[derive(Clone, Debug)]
pub enum Message {
    Activate,
    Deactivate,
    KeyPressed(KeyEvent),
    KeyReleased(KeyEvent),
    Modifiers(Modifiers, RawModifiers),
    Done,
}

impl Application for InputMethod {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (InputMethod, Command<Message>) {
        (InputMethod {}, Command::none())
    }

    fn title(&self) -> String {
        String::from("InputMethod")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Activate => show_input_method_popup(),
            Message::Deactivate => hide_input_method_popup(),
            Message::KeyPressed(key) => {
                virtual_keyboard_action(ActionInner::KeyPressed(key))
            },
            Message::KeyReleased(key) => virtual_keyboard_action(ActionInner::KeyReleased(key)),
            Message::Modifiers(_, raw_modifiers) => virtual_keyboard_action(ActionInner::Modifiers(raw_modifiers)),
            Message::Done => Command::none(),
        }
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        let characters = vec!["我".to_string(), "的".to_string(), "名".to_string()];
        container(
            row(vec![
                column(
                    characters.iter().map(|c| text(c.clone()).into()).collect()
                ).align_items(Alignment::Center).push(button("button").on_press(Message::Deactivate)).into(),
                column(characters.iter().map(|c| text(c.clone()).into()).collect()
                ).align_items(Alignment::Center).into()
            ]).padding(Padding::new(2.0))
        )
        .style(<iced_style::Theme as container::StyleSheet>::Style::Custom(Box::new(CustomTheme)))
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, status| match (event, status) {
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethod(event),
                )),
                _,
            ) => match event{
                InputMethodEvent::Activate => Some(Message::Activate),
                InputMethodEvent::Deactivate => Some(Message::Deactivate),
                InputMethodEvent::SurroundingText { text, cursor, anchor } => None,
                InputMethodEvent::TextChangeCause(_) => None,
                InputMethodEvent::ContentType(_, _) => None,
                InputMethodEvent::Done => Some(Message::Done),
            },
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethodKeyboard(event),
                )),
                _,
            ) => match event {
                InputMethodKeyboardEvent::Press(key) =>Some(Message::KeyPressed(key)),
                InputMethodKeyboardEvent::Release(key) => Some(Message::KeyReleased(key)),
                InputMethodKeyboardEvent::Repeat(key) => Some(Message::KeyPressed(key)),
                InputMethodKeyboardEvent::Modifiers(modifiers, raw_modifiers) => 
                    Some(Message::Modifiers(modifiers, raw_modifiers)),
            }
            _ => None,
        })
    }

    fn close_requested(&self, _id: window::Id) -> Message {
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

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            border_color: Color::from_rgb(1.0, 1.0, 0.0),
            border_radius: 2.5.into(),
            border_width: 5.0,
            background: Some(Color::from_rgb(1.0, 1.0, 0.0).into()),
            ..container::Appearance::default()
        }
    }
}

impl iced_style::application::StyleSheet for CustomTheme {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        iced_style::application::Appearance {
            background_color: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            icon_color: Color::BLACK,
            text_color: Color::BLACK,
        }
    }
}