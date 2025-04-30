#[macro_use]
extern crate lazy_static;

use cctk::wayland_client::protocol::wl_keyboard::KeyState;
use cctk::wayland_protocols::wp::commit_timing;
use iced::event::{self, listen_raw, Event};
use iced::keyboard::Key;
use iced::platform_specific::shell::commands::input_method::{
    commit, forward_key, set_preedit_string, set_string,
};
use iced::widget::{container, text};
use iced::{window, Element, Program, Task};
use iced::{Subscription, Vector};
use iced_core::keyboard::key::Named;
use iced_core::{keyboard, SmolStr};

use std::collections::HashMap;
use std::fmt::Debug;

lazy_static! {
    static ref ACCENTKEYS: HashMap<char, Vec<char>> = [
        ('A', vec!['À', 'Á', 'Â', 'Ã', 'Ä', 'Å', 'Ā', 'Ă', 'Ą', 'Æ']),
        ('C', vec!['Ç', 'Ć', 'Ĉ', 'Ċ', 'Č']),
        ('D', vec!['Ď', 'Đ']),
        ('E', vec!['É', 'È', 'Ê', 'Ë', 'Ē', 'Ĕ', 'Ė', 'Ę', 'Ě']),
        ('G', vec!['Ĝ', 'Ğ', 'Ġ', 'Ģ']),
        ('H', vec!['Ĥ', 'Ħ']),
        ('I', vec!['Í', 'Ì', 'Î', 'Ï', 'Ĩ', 'Ī', 'Ĭ', 'Į', 'İ']),
        ('J', vec!['Ĵ']),
        ('K', vec!['Ķ']),
        ('L', vec!['Ĺ', 'Ļ', 'Ľ', 'Ŀ', 'Ł']),
        ('N', vec!['Ñ', 'Ń', 'Ņ', 'Ň']),
        ('O', vec!['Ó', 'Ò', 'Ô', 'Õ', 'Ö', 'Ō', 'Ŏ', 'Ő', 'Ø', 'Œ']),
        ('R', vec!['Ŕ', 'Ŗ', 'Ř']),
        ('S', vec!['Ś', 'Ŝ', 'Ş', 'Š']),
        ('T', vec!['Ţ', 'Ť', 'Ŧ']),
        ('U', vec!['Ú', 'Ù', 'Û', 'Ü', 'Ũ', 'Ū', 'Ŭ', 'Ů', 'Ű', 'Ų']),
        ('W', vec!['Ŵ']),
        ('Y', vec!['Ý', 'Ŷ', 'Ÿ']),
        ('Z', vec!['Ź', 'Ż', 'Ž']),
        ('a', vec!['à', 'á', 'â', 'ã', 'ä', 'å', 'ā', 'ă', 'ą', 'æ']),
        ('c', vec!['ç', 'ć', 'ĉ', 'ċ', 'č']),
        ('d', vec!['ď', 'đ']),
        ('e', vec!['é', 'è', 'ê', 'ë', 'ē', 'ĕ', 'ė', 'ę', 'ě']),
        ('g', vec!['ĝ', 'ğ', 'ġ', 'ģ']),
        ('h', vec!['ĥ', 'ħ']),
        ('i', vec!['í', 'ì', 'î', 'ï', 'ĩ', 'ī', 'ĭ', 'į', 'ı']),
        ('j', vec!['ĵ']),
        ('k', vec!['ķ']),
        ('l', vec!['ĺ', 'ļ', 'ľ', 'ŀ', 'ł']),
        ('n', vec!['ñ', 'ń', 'ņ', 'ň']),
        ('o', vec!['ó', 'ò', 'ô', 'õ', 'ö', 'ō', 'ŏ', 'ő', 'ø', 'œ']),
        ('r', vec!['ŕ', 'ŗ', 'ř']),
        ('s', vec!['ś', 'ŝ', 'ş', 'š']),
        ('t', vec!['ţ', 'ť', 'ŧ']),
        ('u', vec!['ú', 'ù', 'û', 'ü', 'ũ', 'ū', 'ŭ', 'ů', 'ű', 'ų']),
        ('w', vec!['ŵ']),
        ('y', vec!['ý', 'ÿ', 'ŷ']),
        ('z', vec!['ź', 'ż', 'ž']),
    ]
    .into();
}

pub fn main() -> iced::Result {
    let mut settings = iced::Settings::default();
    settings.id = Some("inputmethod.deamon".to_string());
    iced::daemon(InputMethod::title, InputMethod::update, InputMethod::view)
        .settings(settings)
        .subscription(InputMethod::subscription)
        .run_with(InputMethod::new)
}

#[derive(Debug, Clone, Default)]
struct InputMethod {
    index: usize,
    popup: bool,
    list: Vec<char>,
    preedit_string: String,
}

#[derive(Debug, Clone)]
enum Message {
    Activate { app_id: String },
    Deactivate,
    TextInputDestroyed { app_id: String },
    KeyPressed(Key<SmolStr>),
    KeyReleased(Key<SmolStr>),
    UpdatePopup { index: usize },
    Done,
}

impl InputMethod {
    fn new() -> (InputMethod, Task<Message>) {
        (InputMethod::default(), Task::none())
    }

    fn title(&self, _id: window::Id) -> String {
        "Input Method".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Activate { app_id } => Task::none(),
            Message::Deactivate => Task::none(),
            Message::KeyPressed(key) => {
                if !self.popup {
                    match key {
                        Key::Named(Named::Enter) => {
                            let commit_string = self.preedit_string.clone();
                            self.preedit_string = String::new();
                            Task::batch([set_string(commit_string), commit()])
                        }
                        Key::Named(Named::Backspace) => {
                            if self.preedit_string.len() > 0 {
                                self.preedit_string.pop();
                                let len = self.preedit_string.len() as i32;
                                Task::batch([
                                    set_preedit_string(
                                        self.preedit_string.clone(),
                                        len,
                                        len,
                                    ),
                                    commit(),
                                ])
                            } else {
                                forward_key(KeyState::Pressed)
                            }
                        }
                        Key::Named(Named::Space) => {
                            if self.preedit_string.len() > 0 {
                                self.preedit_string += " ";
                                let len = self.preedit_string.len() as i32;
                                Task::batch([
                                    set_preedit_string(
                                        self.preedit_string.clone(),
                                        len,
                                        len,
                                    ),
                                    commit(),
                                ])
                            } else {
                                forward_key(KeyState::Pressed)
                            }
                        }
                        Key::Character(c) => {
                            self.preedit_string += &c;
                            let len = self.preedit_string.len() as i32;
                            Task::batch([
                                set_preedit_string(
                                    self.preedit_string.clone(),
                                    len,
                                    len,
                                ),
                                commit(),
                            ])
                        }
                        Key::Unidentified => Task::none(),
                        _ => forward_key(KeyState::Pressed),
                    }
                } else {
                    match key {
                        Key::Named(Named::Enter) => {
                            set_string(self.preedit_string.clone())
                        }
                        Key::Character(c) => {
                            self.preedit_string += &c;
                            let len = self.preedit_string.len() as i32;
                            set_preedit_string(
                                self.preedit_string.clone(),
                                len,
                                len,
                            )
                        }
                        Key::Unidentified => Task::none(),
                        // KeyCode::Left => {
                        //     if self.index > 0 {
                        //         self.index -= 1;
                        //     }
                        //     Task::none()
                        // }
                        // KeyCode::Right => {
                        //     if self.index < self.list.len() - 1 {
                        //         self.index += 1;
                        //     }
                        //     Task::none()
                        // }
                        // KeyCode::Enter => {
                        //     self.commit_string(self.list[self.index])
                        // }
                        _ => Task::none(),
                    }
                }
            }
            // Message::KeyRepeat(key) => {
            //     if !self.popup {
            //         if let Some(utf8) = key
            //             .utf8
            //             .as_ref()
            //             .map(|str| str.chars().last().unwrap_or_default())
            //         {
            //             if let Some(list) = ACCENTKEYS.get(&utf8) {
            //                 self.open_popup(utf8, list)
            //             } else {
            //                 forward_key()
            //             }
            //         } else {
            //             forward_key()
            //         }
            //     } else {
            //         match key_code {
            //             KeyCode::Left => {
            //                 if self.index > 0 {
            //                     self.index -= 1;
            //                 }
            //                 Task::none()
            //             }
            //             KeyCode::Right => {
            //                 if self.index < self.list.len() - 1 {
            //                     self.index += 1;
            //                 }
            //                 Task::none()
            //             }
            //             _ => Task::none(),
            //         }
            //     }
            // }
            Message::KeyReleased(key) => {
                if !self.popup {
                    forward_key(KeyState::Released)
                } else {
                    // match key {
                    //     KeyCode::Enter => self.popup = false,
                    //     _ => {}
                    // }
                    Task::none()
                }
            }
            Message::Done => Task::none(),
            Message::UpdatePopup { index } => {
                self.index = index;
                Task::none()
            }
            Message::TextInputDestroyed { app_id } => Task::none(),
        }
    }

    fn view(&self, id: window::Id) -> Element<Message> {
        container(
            text("hello world"), // row(self
                                 //     .list
                                 //     .iter()
                                 //     .enumerate()
                                 //     .map(|(index, char)| {
                                 //         selection_field(
                                 //             column(vec![
                                 //                 text((index + 1) % 10).size(50).into(),
                                 //                 text(char).size(50).into(),
                                 //             ])
                                 //             .align_items(Alignment::Center)
                                 //             .padding(5.0)
                                 //             .spacing(4.0),
                                 //         )
                                 //         .set_indexes(index)
                                 //         .selected(self.index)
                                 //         .on_press(Message::Deactivate)
                                 //         .on_select(Message::UpdatePopup { index })
                                 //         .into()
                                 //     })
                                 //     .collect())
                                 // .padding(2.0),
        )
        .padding(5.0)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        listen_raw(|event, status, window| {
            // dbg!(&event);
            // println!("{:?}", event);
            match (event, status, window) {
                (
                    Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                        event::wayland::Event::InputMethod(event),
                    )),
                    _,
                    _,
                ) => match event {
                    event::wayland::InputMethodEvent::Activate { app_id } => {
                        Some(Message::Activate { app_id })
                    }
                    event::wayland::InputMethodEvent::Deactivate => {
                        Some(Message::Deactivate)
                    }
                    event::wayland::InputMethodEvent::TextInputDestroyed {
                        app_id,
                    } => Some(Message::TextInputDestroyed { app_id }),
                    event::wayland::InputMethodEvent::SurroundingText {
                        text,
                        cursor,
                        anchor,
                    } => None,
                    event::wayland::InputMethodEvent::TextChangeCause {
                        cause,
                    } => None,
                    event::wayland::InputMethodEvent::ContentType {
                        hint,
                        purpose,
                    } => None,
                    event::wayland::InputMethodEvent::Done => {
                        Some(Message::Done)
                    }
                    event::wayland::InputMethodEvent::AvailableActions {
                        available_actions,
                    } => None,
                },
                (Event::Keyboard(event), event::Status::Ignored, _) => {
                    match event {
                        keyboard::Event::KeyPressed {
                            key,
                            modified_key,
                            physical_key,
                            location,
                            modifiers,
                            text,
                        } => Some(Message::KeyPressed(modified_key)),
                        keyboard::Event::KeyReleased {
                            key,
                            modified_key,
                            physical_key,
                            location,
                            modifiers,
                        } => Some(Message::KeyReleased(modified_key)),
                        keyboard::Event::ModifiersChanged(_) => None,
                    }
                }
                _ => None,
            }
        })
    }
}
