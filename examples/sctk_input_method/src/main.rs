use iced::{
    event::{
        self,
        wayland::{InputMethodEvent, InputMethodKeyboardEvent, KeyEvent},
    },
    subscription,
    wayland::{virtual_keyboard::virtual_keyboard_key_press, InitialSurface, actions::window::SctkWindowSettings},
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
    KeyPressed(KeyEvent),
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Activate => write!(f, "Message::Activate"),
            Message::Deactivate => write!(f, "Message::Deactivate"),
            Message::KeyPressed(_key) => write!(f, "Message::KeyPressed"),
        }
    }
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
            Message::KeyPressed(key) => virtual_keyboard_key_press(key),
        }
    }

    fn view(&self, _id: window::Id) -> Element<Message> {
        unimplemented!();
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events_with(|event, status| match (event, status) {
            // (
            //     Event::PlatformSpecific(event::PlatformSpecific::Wayland(
            //         event::wayland::Event::InputMethod(
            //             InputMethodEvent::Activate,
            //         ),
            //     )),
            //     _,
            // ) => Some(Message::Activate),
            (
                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::InputMethodKeyboard(
                        InputMethodKeyboardEvent::Press(key),
                    ),
                )),
                _,
            ) => {
                dbg!(&key);
                Some(Message::KeyPressed(key))
            }
            // (Event::Keyboard(_), event::Status::Ignored) => todo!(),
            // (Event::Keyboard(_), event::Status::Captured) => None,
            // (Event::Mouse(_), event::Status::Ignored) => None,
            // (Event::Mouse(_), event::Status::Captured) => None,
            // (Event::Window(_, _), event::Status::Ignored) => None,
            // (Event::Window(_, _), event::Status::Captured) => None,
            // (Event::Touch(_), event::Status::Ignored) => None,
            // (Event::Touch(_), event::Status::Captured) => None,
            // // (Event::A11y(_, _), event::Status::Ignored) => None,
            // // (Event::A11y(_, _), event::Status::Captured) => None,
            // (Event::PlatformSpecific(ps), _) => match ps {
            //     event::PlatformSpecific::Wayland(event) => match event {
            //         event::wayland::Event::Layer(_, _, _) => None,
            //         event::wayland::Event::Popup(_, _, _) => None,
            //         event::wayland::Event::Output(_, _) => None,
            //         event::wayland::Event::Window(_, _, _) => None,
            //         event::wayland::Event::Seat(_, _) => None,
            //         event::wayland::Event::DataSource(_) => None,
            //         event::wayland::Event::DndOffer(_) => None,
            //         event::wayland::Event::SelectionOffer(_) => None,
            //         event::wayland::Event::Frame(_, _, _) => None,
            //         event::wayland::Event::InputMethod(_) => None,
            //         event::wayland::Event::InputMethodKeyboard(ke) => {
            //             match ke {
            //                 InputMethodKeyboardEvent::Press(ke) => {
            //                     Some(Message::KeyPressed(ke))
            //                 }
            //                 InputMethodKeyboardEvent::Release(_) => None,
            //                 InputMethodKeyboardEvent::Repeat(_) => None,
            //                 InputMethodKeyboardEvent::Modifiers(_) => None,
            //             }
            //         }
            //     },
            //     event::PlatformSpecific::MacOS(_) => None,
            _ => None,
            // },
            // (Event::PlatformSpecific(ps), event::Status::Captured) => None,
        })
    }

    fn close_requested(&self, _id: window::Id) -> Message {
        unimplemented!()
    }
}
