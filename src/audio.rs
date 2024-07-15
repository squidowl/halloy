use data::audio;

pub enum State {
    Ready { manager: audio::Manager },
    Unsupported,
}

impl State {
    pub fn new() -> Self {
        let Ok(manager) = audio::Manager::new() else {
            return Self::Unsupported;
        };

        State::Ready { manager }
    }

    pub fn play(&mut self, sound: &audio::Sound) {
        if let State::Ready { manager } = self {
            let _ = manager.play(sound);
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
