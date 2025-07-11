use iced::widget::{button, column, container, text};
use iced::window;
use iced::{Element, Subscription, Task};
use image::GenericImageView;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

use ksni::TrayMethods;

static TRAY_COMMANDS: LazyLock<Arc<Mutex<Vec<TrayCommand>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

#[derive(Debug, Clone)]
enum TrayCommand {
    ShowWindow,
    HideWindow,
    Exit,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tray = SystemTray;
            if let Err(e) = tray.spawn().await {
                println!("failed to spawn tray {e}");
            } else {
                std::future::pending::<()>().await;
            }
        });
    });

    let result = iced::daemon(Example::new, Example::update, Example::view)
        .subscription(Example::subscription)
        .run();

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
    }
}

struct Example {
    counter: i32,
    current_window_id: Option<window::Id>,
    window_is_open: bool,
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
    WindowClosed(window::Id),
    HideToTray,
    ExitApp,
    Increment,
    Decrement,
    CheckTrayCommands,
}

impl Example {
    fn new() -> (Self, Task<Message>) {
        let (_id, open) = window::open(window::Settings {
            size: iced::Size::new(400.0, 300.0),
            position: window::Position::Centered,
            ..window::Settings::default()
        });

        (
            Self {
                counter: 0,
                current_window_id: None,
                window_is_open: false,
            },
            open.map(Message::WindowOpened),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                self.current_window_id = Some(id);
                self.window_is_open = true;
                Task::none()
            }
            Message::WindowClosed(_id) => {
                self.current_window_id = None;
                self.window_is_open = false;
                Task::none()
            }
            Message::HideToTray => {
                if let Some(window_id) = self.current_window_id {
                    window::close(window_id)
                } else {
                    Task::none()
                }
            }
            Message::ExitApp => iced::exit(),
            Message::Increment => {
                self.counter += 1;
                Task::none()
            }
            Message::Decrement => {
                self.counter -= 1;
                Task::none()
            }
            Message::CheckTrayCommands => {
                if let Ok(mut commands) = TRAY_COMMANDS.lock() {
                    if let Some(cmd) = commands.pop() {
                        match cmd {
                            TrayCommand::ShowWindow => {
                                if !self.window_is_open {
                                    let (_id, open) = window::open(window::Settings {
                                        size: iced::Size::new(400.0, 300.0),
                                        position: window::Position::Centered,
                                        ..window::Settings::default()
                                    });
                                    return open.map(Message::WindowOpened);
                                }
                            }
                            TrayCommand::HideWindow => {
                                if let Some(window_id) = self.current_window_id {
                                    return window::close(window_id);
                                }
                            }
                            TrayCommand::Exit => {
                                return iced::exit();
                            }
                        }
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self, _window_id: window::Id) -> Element<'_, Message> {
        let content = column![
            text("Trayer Application").size(24),
            text(format!("Counter: {}", self.counter)).size(18),
            text("Simple system tray app").size(16),
            button("Increment").on_press(Message::Increment),
            button("Decrement").on_press(Message::Decrement),
            button("Hide to Tray").on_press(Message::HideToTray),
            button("Exit").on_press(Message::ExitApp),
        ]
        .spacing(15)
        .align_x(iced::Alignment::Center);

        container(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch([
            window::close_events().map(Message::WindowClosed),
            iced::time::every(std::time::Duration::from_millis(500))
                .map(|_| Message::CheckTrayCommands),
        ])
    }
}

//tray related boilerplate
struct SystemTray;

impl ksni::Tray for SystemTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn title(&self) -> String {
        "Trayer".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
            let img = image::load_from_memory_with_format(
                include_bytes!("../icons/custom_icon.png"),
                image::ImageFormat::Png,
            )
            .expect("valid image");
            let (width, height) = img.dimensions();
            let mut data = img.into_rgba8().into_vec();
            assert_eq!(data.len() % 4, 0);
            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1) // rgba to argb
            }
            ksni::Icon {
                width: width as i32,
                height: height as i32,
                data,
            }
        });

        vec![ICON.clone()]
    }

    fn icon_name(&self) -> String {
        "application-default-icon".into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;

        vec![
            StandardItem {
                label: "Show Window".into(),
                activate: Box::new(|_this: &mut Self| {
                    Self::send_command(TrayCommand::ShowWindow);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Hide Window".into(),
                activate: Box::new(|_this: &mut Self| {
                    Self::send_command(TrayCommand::HideWindow);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Exit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_this: &mut Self| {
                    Self::send_command(TrayCommand::Exit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Show window on tray click
        Self::send_command(TrayCommand::ShowWindow);
    }
}

impl SystemTray {
    fn send_command(cmd: TrayCommand) {
        if let Ok(mut commands) = TRAY_COMMANDS.lock() {
            commands.push(cmd);
        }
    }
}
