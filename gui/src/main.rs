use iced::{Element, Sandbox, Settings};

fn main() -> iced::Result {
    BBImager::run(Settings::default())
}

struct BBImager;

impl Sandbox for BBImager {
    type Message = ();

    fn new() -> Self {
        Self
    }

    fn title(&self) -> String {
        String::from("A cool application")
    }

    fn update(&mut self, _message: Self::Message) {
        // This application has no interactions
    }

    fn view(&self) -> Element<Self::Message> {
        "Hello, world!".into()
    }
}
