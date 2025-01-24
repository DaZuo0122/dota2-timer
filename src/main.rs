use iced::{
    time, widget, Size,
    window::{self, Level},
    Application, Element, Subscription, Theme,
};
//use iced::executor::Default;
use std::default::Default;
use rodio::{Decoder, OutputStream, Sink};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    time::{Duration, Instant},
};
//use iced::window::Position::Default;

#[derive(Debug, Deserialize)]
struct Config {
    audio: HashMap<u16, String>,
}

#[derive(Debug)]
struct TimerApp {
    state: TimerState,
    yaml_files: Vec<String>,
    selected_file: Option<String>,
    audio_map: HashMap<Duration, String>,
    elapsed: Duration,
    countdown: Duration,

}

#[derive(Debug)]
enum TimerState {
    Idle,
    CountingDown(Instant),
    Running(Instant),
    Paused(Duration),
}

impl Default for TimerState {
    fn default() -> Self {
        TimerState::Idle
    }
}

#[derive(Debug, Clone)]
enum Message {
    StartPause,
    LoadYaml(String),
    Tick(Instant),
}

impl Default for TimerApp {
    fn default() -> Self {
        Self {
            yaml_files: get_yaml_files(),
            countdown: Duration::from_secs(60),
            selected_file: None,
            audio_map: HashMap::new(),
            elapsed: Duration::ZERO,
            state: TimerState::default(),
        }
    }
}

fn get_yaml_files() -> Vec<String> {
    fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? == "yaml" {
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect()
}

impl TimerApp {
    fn check_audio_triggers(&self) {
        if let Some(path) = self.audio_map.get(&self.elapsed) {
            play_audio(path);
        }
    }
}

fn play_audio(path: &str) {
    let path = path.to_string();
    std::thread::spawn(move || {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        let file = fs::File::open(&path).unwrap();
        let source = Decoder::new(file).unwrap();
        sink.append(source);
        sink.sleep_until_end();
    });
}

fn main() -> iced::Result {
    iced::application("Dota Timer", update, view)
        .subscription(subscription)
        .window(window::Settings {
            level: Level::AlwaysOnTop,
            size: Size::new(300.0, 200.0),
            ..window::Settings::default()
        })
        .run()
}

fn subscription(state: &TimerApp) -> Subscription<Message> {
    match &state.state {
        TimerState::CountingDown(_) | TimerState::Running(_) => {
            time::every(Duration::from_millis(10)).map(Message::Tick)
        }
        _ => Subscription::none(),
    }
}

// UPDATE FUNCTION
fn update(state: &mut TimerApp, message: Message) {
    match message {
        Message::StartPause => match &state.state {
            TimerState::Idle => {
                state.state = TimerState::CountingDown(Instant::now());
                state.countdown = Duration::from_secs(60);
            }
            TimerState::Running(_) => {
                state.state = TimerState::Paused(state.elapsed);
            }
            TimerState::Paused(d) => {
                state.state = TimerState::Running(Instant::now() - *d);
            }
            TimerState::CountingDown(_) => {
                state.state = TimerState::Idle;
                state.countdown = Duration::from_secs(60);
            }
        },
        Message::LoadYaml(file) => {
            state.selected_file = Some(file.clone());
            if let Ok(contents) = fs::read_to_string(&file) {
                let config: Config = serde_yaml::from_str(&contents).unwrap();
                state.audio_map = config
                    .audio
                    .into_iter()
                    .map(|(k, v)| (Duration::from_secs(k.into()), v))
                    .collect();
            }
        },
        Message::Tick(now) => match &mut state.state {
            TimerState::CountingDown(last_tick) => {
                let elapsed = now - *last_tick;
                state.countdown = state.countdown.saturating_sub(elapsed);
                *last_tick = now; // Update last_tick reference

                if state.countdown.is_zero() {
                    state.state = TimerState::Running(now);
                    state.elapsed = Duration::ZERO;
                }
            }
            TimerState::Running(last_tick) => {
                state.elapsed += now - *last_tick;
                *last_tick = now;
                state.check_audio_triggers();
            }
            _ => {}
        },
    }
}

// VIEW FUNCTION
fn view(state: &TimerApp) -> iced::Element<Message> {
    let time_text = match &state.state {
        TimerState::CountingDown(_) => format!(
            "{:02}:{:02}",
            state.countdown.as_secs() / 60,
            state.countdown.as_secs() % 60
        ),
        _ => format!(
            "{:02}:{:02}",
            state.elapsed.as_secs() / 60,
            state.elapsed.as_secs() % 60
        ),
    };

    let button_label = match state.state {
        TimerState::Idle => "Start",
        TimerState::CountingDown(_) => "Cancel",
        TimerState::Running(_) => "Pause",
        TimerState::Paused(_) => "Resume",
    };

    let pick_list = widget::PickList::new(
        state.yaml_files.as_slice(),
        state.selected_file.clone(),
        Message::LoadYaml,
    )
        .placeholder("Select YAML File");

    widget::column![
        widget::text("Audio Timer").size(30),
        widget::text(time_text).size(40),
        widget::row![
            widget::button(button_label).on_press(Message::StartPause),
        ],
        pick_list
    ]
        .padding(20)
        .into()
}