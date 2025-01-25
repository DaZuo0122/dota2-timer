use iced::{
    time, widget, Size,
    window::{self, Level},
    Subscription,
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
use std::collections::HashSet;
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
    current_display: Duration,
    triggered_audio: HashSet<Duration>,

}

#[derive(Debug)]
enum TimerState {
    Idle,
    CountingDown(Instant),
    Running {
        base_time: Duration,
        last_start: Instant,
    },
    Paused(Duration),
}

impl Default for TimerState {
    fn default() -> Self {
        TimerState::Idle
    }
}

#[derive(Debug, Clone)]
enum Message {
    StartRestart,
    PauseResume,
    LoadYaml(String),
    Tick(Instant),
}

impl Default for TimerApp {
    fn default() -> Self {
        Self {
            yaml_files: get_yaml_files(),
            selected_file: None,
            audio_map: HashMap::new(),
            state: TimerState::default(),
            current_display: Duration::ZERO,
            triggered_audio: HashSet::new(),
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
    fn check_audio_triggers(&mut self) {
        let current_sec = self.current_display.as_secs();
        let trigger_point = Duration::from_secs(current_sec);

        if self.audio_map.contains_key(&trigger_point)
            && !self.triggered_audio.contains(&trigger_point)
        {
            if let Some(path) = self.audio_map.get(&trigger_point) {
                play_audio(path);
                self.triggered_audio.insert(trigger_point);
            }
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
            size: Size::new(200.0, 120.0),
            ..window::Settings::default()
        })
        .run()
}

fn subscription(state: &TimerApp) -> Subscription<Message> {
    match &state.state {
        TimerState::CountingDown(_) | TimerState::Running{ .. } => {
            time::every(Duration::from_millis(10)).map(Message::Tick)
        }
        _ => Subscription::none(),
    }
}

// UPDATE FUNCTION
fn update(state: &mut TimerApp, message: Message) {
    match message {
        Message::StartRestart => {
            // Always reset to initial state when clicking Start/Restart
            state.state = TimerState::CountingDown(Instant::now());
            state.current_display = Duration::from_secs(90);
            state.triggered_audio.clear();

            // Reload the selected YAML file if present
            if let Some(file) = &state.selected_file {
                if let Ok(contents) = fs::read_to_string(file) {
                    let config: Config = serde_yaml::from_str(&contents).unwrap();
                    state.audio_map = config
                        .audio
                        .into_iter()
                        .map(|(k, v)| (Duration::from_secs(k.into()), v))
                        .collect();
                }
            }
        },
        Message::PauseResume => match &state.state {
            TimerState::Running { base_time, last_start } => {
                let elapsed = *base_time + last_start.elapsed();
                state.state = TimerState::Paused(elapsed);
                state.current_display = elapsed;
            },
            TimerState::Paused(elapsed) => {
                state.state = TimerState::Running {
                    base_time: *elapsed,
                    last_start: Instant::now(),
                };
            },
            _ => {}
        },
        Message::LoadYaml(file) => {
            state.selected_file = Some(file.clone());
            state.audio_map.clear();  // Clear previous entries
            state.triggered_audio.clear();

            if let Ok(contents) = fs::read_to_string(&file) {
                if let Ok(config) = serde_yaml::from_str::<Config>(&contents) {
                    state.audio_map = config.audio.into_iter().map(|(k, v)| {
                        (Duration::from_secs(k.into()), v)
                    }).collect();
                }
            }
        },
        Message::Tick(now) => match &mut state.state {
            TimerState::CountingDown(start_time) => {
                let remaining = Duration::from_secs(60).saturating_sub(now.duration_since(*start_time));
                state.current_display = remaining;

                if remaining.is_zero() {
                    state.state = TimerState::Running {
                        base_time: Duration::ZERO,
                        last_start: Instant::now(),
                    };
                }
            },
            TimerState::Running { base_time, last_start } => {
                let elapsed = *base_time + last_start.elapsed();
                state.current_display = elapsed;
                state.check_audio_triggers();
            },
            TimerState::Paused(elapsed) => {
                state.current_display = *elapsed;
            },
            _ => {}
        },
    }
}

// VIEW FUNCTION
fn view(state: &TimerApp) -> iced::Element<Message> {
    let time_text = match &state.state {
        TimerState::CountingDown(_) => format!(
            "{:02}:{:02}",
            state.current_display.as_secs() / 60,
            state.current_display.as_secs() % 60
        ),
        _ => format!(
            "{:02}:{:02}",
            state.current_display.as_secs() / 60,
            state.current_display.as_secs() % 60
        ),
    };

    // Start/Restart button logic
    let (start_label, _is_restart) = match state.state {
        TimerState::Idle => ("Start", false),
        _ => ("Restart", true),
    };

    let start_restart_button = widget::button(start_label)
        .on_press(Message::StartRestart)
        .padding(10);

    // Pause/Resume button logic
    let pause_resume_button = match state.state {
        TimerState::Running{base_time:_, last_start:_} => Some(widget::button("Pause")
            .on_press(Message::PauseResume)
            .padding(10)),
        TimerState::Paused(_) => Some(widget::button("Resume")
            .on_press(Message::PauseResume)
            .padding(10)),
        _ => None,
    };


    let pick_list = widget::PickList::new(
        state.yaml_files.as_slice(),
        state.selected_file.clone(),
        Message::LoadYaml,
    )
        .placeholder("Select Strategy File");

    let mut buttons = widget::row![start_restart_button];
    if let Some(btn) = pause_resume_button {
        buttons = buttons.push(btn);
    }

    widget::column![
        widget::text(time_text).size(25),
        buttons,
        pick_list
    ]
        .padding(12)
        .into()
}