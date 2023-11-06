use iced::{
    event::{
        self,
        wayland::{InputMethodEvent, InputMethodKeyboardEvent, KeyEvent, RawModifiers, Modifiers},
    },
    subscription,
    wayland::{
        actions::{virtual_keyboard::ActionInner, input_method_popup::InputMethodPopupSettings},
        virtual_keyboard::virtual_keyboard_action, InitialSurface, input_method::{show_input_method_popup, get_input_method_popup},
    },
    widget::{container, Container, Row, Column, Text},
    window, Application, Color, Command, Element, Event, Subscription, Theme,
    Settings, alignment::{Vertical, Horizontal}, Length,
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
    Modifiers(Modifiers, RawModifiers)
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
            Message::Activate => Command::none(),
            Message::Deactivate => Command::none(),
            Message::KeyPressed(key) => {
                // get_input_method_popup(InputMethodPopupSettings::default());
                show_input_method_popup()
                // virtual_keyboard_action(ActionInner::KeyPressed(key))
            },
            Message::KeyReleased(key) => virtual_keyboard_action(ActionInner::KeyReleased(key)),
            Message::Modifiers(_, raw_modifiers) => virtual_keyboard_action(ActionInner::Modifiers(raw_modifiers)),
        }
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        let row = Row::new().push(
            Text::new("Hello World!")
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Center),
        );
        let column = Column::new().push(row);
        let content: Element<_> = column.into();
        Container::new(content)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
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
                InputMethodEvent::Activate => {
                    // dbg!("activate");
                    Some(Message::Activate)
                }
                InputMethodEvent::Deactivate => Some(Message::Deactivate),
                InputMethodEvent::SurroundingText { text, cursor, anchor } => None,
                InputMethodEvent::TextChangeCause(_) => None,
                InputMethodEvent::ContentType(_, _) => None,
                InputMethodEvent::Done => None,
            },
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethodKeyboard(event),
                )),
                _,
            ) => match event {
                InputMethodKeyboardEvent::Press(key) => {
                    // dbg!(&key);
                    Some(Message::KeyPressed(key))
                }
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
}
