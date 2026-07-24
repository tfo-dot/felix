use crate::config::Config;
use crate::event::ActiveWindowGeometry;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClimbSide {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionState {
    None,
    Sitting {
        x_rel: i32,
        duration_secs: u32,
        start_time: Instant,
    },
    Walking {
        x_rel: f64,
        target_x_rel: i32,
        dir_right: bool,
    },
    Climbing {
        y_rel: f64,
        target_y_rel: i32,
        side: ClimbSide,
        dir_down: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetState {
    Idle,
    Typing,
    Tracking { dx: f64, dy: f64 },
    Sleeping,
    WorkingHard,
    Petting,
    WalkingLeft,
    WalkingRight,
    ClimbingUp,
    ClimbingDown,
    Dancing,
    PortalOut,
    PortalIn,
    Dragged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutineState {
    Coffee,
    Lunch,
    Slump,
    Reading,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PomodoroState {
    Work,
    ShortBreak,
    LongBreak,
    Paused { previous: Box<PomodoroState> },
}

impl PomodoroState {
    pub fn label(&self) -> &'static str {
        match self {
            PomodoroState::Work => "Working Time",
            PomodoroState::ShortBreak => "Short Break",
            PomodoroState::LongBreak => "Long Break",
            PomodoroState::Paused { .. } => "Paused",
        }
    }
}

pub struct PomodoroTimer {
    pub state: PomodoroState,
    pub seconds_remaining: u32,
    pub completed_sessions: u32,
}

impl PomodoroTimer {
    pub fn new(config: &Config) -> Self {
        let state = if config.pomodoro.start_paused {
            PomodoroState::Paused {
                previous: Box::new(PomodoroState::Work),
            }
        } else {
            PomodoroState::Work
        };
        Self {
            state,
            seconds_remaining: config.pomodoro.work_duration_mins * 60,
            completed_sessions: 0,
        }
    }

    pub fn tick(&mut self, config: &Config) -> Option<String> {
        if matches!(self.state, PomodoroState::Paused { .. }) {
            return None;
        }

        if self.seconds_remaining > 0 {
            self.seconds_remaining -= 1;
            None
        } else {
            let message = match self.state {
                PomodoroState::Work => {
                    self.completed_sessions += 1;
                    if self.completed_sessions % 4 == 0 {
                        self.state = PomodoroState::LongBreak;
                        self.seconds_remaining = config.pomodoro.long_break_mins * 60;
                        Some("Great job! Take a long break. ☕".to_string())
                    } else {
                        self.state = PomodoroState::ShortBreak;
                        self.seconds_remaining = config.pomodoro.short_break_mins * 60;
                        Some("Time for a short break! Stretch a bit. ✨".to_string())
                    }
                }
                PomodoroState::ShortBreak | PomodoroState::LongBreak => {
                    self.state = PomodoroState::Work;
                    self.seconds_remaining = config.pomodoro.work_duration_mins * 60;
                    Some("Back to work! You can do this. 💪".to_string())
                }
                PomodoroState::Paused { .. } => unreachable!(),
            };
            message
        }
    }

    pub fn toggle_pause(&mut self, _config: &Config) {
        let current = std::mem::replace(&mut self.state, PomodoroState::Work);
        match current {
            PomodoroState::Paused { previous } => {
                self.state = *previous;
            }
            other => {
                self.state = PomodoroState::Paused {
                    previous: Box::new(other),
                };
            }
        }
    }
}

pub struct PetAnimationState {
    pub current_state: PetState,
    pub current_frame: u32,
    pub last_keystroke: Option<Instant>,
    pub last_activity: Instant,
    pub last_petted: Option<Instant>,
    pub cpu_working_hard: bool,
    pub interaction_state: InteractionState,
    pub active_window: Option<ActiveWindowGeometry>,
    pub last_win_address: Option<String>,
    pub music_playing: bool,
    pub last_task_completed: Option<Instant>,
    pub portal_start: Option<Instant>,
    pub is_dragging: bool,
    pub drag_start_x: i32,
    pub drag_start_y: i32,
}

impl PetAnimationState {
    pub fn new() -> Self {
        Self {
            current_state: PetState::Idle,
            current_frame: 0,
            last_keystroke: None,
            last_activity: Instant::now(),
            last_petted: None,
            cpu_working_hard: false,
            interaction_state: InteractionState::None,
            active_window: None,
            last_win_address: None,
            music_playing: false,
            last_task_completed: None,
            portal_start: None,
            is_dragging: false,
            drag_start_x: 0,
            drag_start_y: 0,
        }
    }

    pub fn register_keystroke(&mut self) {
        self.last_keystroke = Some(Instant::now());
        self.last_activity = Instant::now();
    }

    pub fn register_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn register_petting(&mut self) {
        self.last_petted = Some(Instant::now());
        self.last_activity = Instant::now();
    }

    pub fn start_portal_transition(&mut self) {
        self.current_state = PetState::PortalOut;
        self.portal_start = Some(Instant::now());
        self.current_frame = 0;
    }

    pub fn update_state(&mut self, _config: &Config, cursor_moved_rapidly: Option<(f64, f64)>) {
        let now = Instant::now();

        // 0. Dragging state takes highest precedence
        if self.is_dragging {
            self.current_state = PetState::Dragged;
            return;
        }

        // 0. Portal travel transition takes highest precedence
        if let Some(start) = self.portal_start {
            let elapsed = now.duration_since(start).as_millis();
            if self.current_state == PetState::PortalOut {
                if elapsed >= 800 {
                    self.current_state = PetState::PortalIn;
                    self.current_frame = 0;
                    self.portal_start = Some(now);
                }
                return;
            } else if self.current_state == PetState::PortalIn {
                if elapsed >= 800 {
                    self.portal_start = None;
                }
                return;
            }
        }

        // 0. Task completion celebration takes highest precedence
        let is_celebrating = self
            .last_task_completed
            .map(|t| now.duration_since(t) < Duration::from_secs(3))
            .unwrap_or(false);

        if is_celebrating {
            self.current_state = PetState::Dancing;
            return;
        }

        // 0.5. Music playing dancing state
        if self.music_playing {
            self.current_state = PetState::Dancing;
            return;
        }

        // 1. Keystroke takes precedence (Typing within last 500ms)
        let is_typing = self
            .last_keystroke
            .map(|t| now.duration_since(t) < Duration::from_millis(500))
            .unwrap_or(false);

        if is_typing {
            if self.cpu_working_hard {
                self.current_state = PetState::WorkingHard;
            } else {
                self.current_state = PetState::Typing;
            }
            return;
        }

        // 2. Petting takes high priority (Petting within last 3 seconds)
        let is_petting = self
            .last_petted
            .map(|t| now.duration_since(t) < Duration::from_secs(3))
            .unwrap_or(false);

        if is_petting {
            self.current_state = PetState::Petting;
            return;
        }

        // 3. Process monitoring (Working Hard animation if typing/active process is heavy)
        if self.cpu_working_hard {
            self.current_state = PetState::WorkingHard;
            return;
        }

        // 4. Sleeping (No input/activity for 5 minutes)
        if now.duration_since(self.last_activity) > Duration::from_secs(300) {
            self.current_state = PetState::Sleeping;
            return;
        }

        // 5. Check if walking or climbing in interaction state
        match &self.interaction_state {
            InteractionState::Walking { dir_right, .. } => {
                if *dir_right {
                    self.current_state = PetState::WalkingRight;
                } else {
                    self.current_state = PetState::WalkingLeft;
                }
            }
            InteractionState::Climbing { dir_down, .. } => {
                if *dir_down {
                    self.current_state = PetState::ClimbingDown;
                } else {
                    self.current_state = PetState::ClimbingUp;
                }
            }
            _ => {
                // 6. Cursor tracking
                if let Some((dx, dy)) = cursor_moved_rapidly {
                    self.current_state = PetState::Tracking { dx, dy };
                    self.last_activity = now;
                } else {
                    // 7. Default to Idle
                    self.current_state = PetState::Idle;
                }
            }
        }
    }

    pub fn update_interaction(&mut self, config: &Config, pet_size: i32, monitor_width: i32) {
        if !config.pet.window_interaction {
            self.interaction_state = InteractionState::None;
            self.last_win_address = None;
            return;
        }

        let active_window = match &self.active_window {
            Some(w) => w,
            None => {
                self.interaction_state = InteractionState::None;
                self.last_win_address = None;
                return;
            }
        };

        let new_address = Some(active_window.address.clone());
        if self.last_win_address != new_address {
            self.last_win_address = new_address;

            // Random sitting position
            let max_x = (active_window.width - pet_size).max(0);
            let x_rel = if max_x > 0 {
                gtk4::glib::random_int_range(0, max_x)
            } else {
                0
            };
            self.interaction_state = InteractionState::Sitting {
                x_rel,
                duration_secs: gtk4::glib::random_int_range(5, 12) as u32,
                start_time: Instant::now(),
            };
            return;
        }

        let win_w = active_window.width;
        let win_h = active_window.height;
        let now = Instant::now();

        match self.interaction_state.clone() {
            InteractionState::None => {
                let max_x = (win_w - pet_size).max(0);
                let x_rel = if max_x > 0 {
                    gtk4::glib::random_int_range(0, max_x)
                } else {
                    0
                };
                self.interaction_state = InteractionState::Sitting {
                    x_rel,
                    duration_secs: gtk4::glib::random_int_range(5, 12) as u32,
                    start_time: now,
                };
            }
            InteractionState::Sitting {
                x_rel,
                duration_secs,
                start_time,
            } => {
                let x_rel = x_rel.clamp(0, (win_w - pet_size).max(0));

                if now.duration_since(start_time) >= Duration::from_secs(duration_secs as u64) {
                    let r = gtk4::glib::random_double();

                    let win_x = active_window.x;
                    let left_space = win_x > pet_size;
                    let right_space = win_x + win_w + pet_size < monitor_width;

                    if r < 0.45 {
                        let max_x = (win_w - pet_size).max(0);
                        let target_x_rel = if max_x > 0 {
                            gtk4::glib::random_int_range(0, max_x)
                        } else {
                            0
                        };
                        self.interaction_state = InteractionState::Walking {
                            x_rel: x_rel as f64,
                            target_x_rel,
                            dir_right: target_x_rel > x_rel,
                        };
                    } else if r < 0.80 && (left_space || right_space) {
                        let side = if left_space && right_space {
                            if gtk4::glib::random_double() < 0.5 {
                                ClimbSide::Left
                            } else {
                                ClimbSide::Right
                            }
                        } else if left_space {
                            ClimbSide::Left
                        } else {
                            ClimbSide::Right
                        };

                        let target_y_rel = (win_h - pet_size).max(0);
                        self.interaction_state = InteractionState::Climbing {
                            y_rel: 0.0,
                            target_y_rel,
                            side,
                            dir_down: true,
                        };
                    } else {
                        self.interaction_state = InteractionState::Sitting {
                            x_rel,
                            duration_secs: gtk4::glib::random_int_range(5, 12) as u32,
                            start_time: now,
                        };
                    }
                } else {
                    self.interaction_state = InteractionState::Sitting {
                        x_rel,
                        duration_secs,
                        start_time,
                    };
                }
            }
            InteractionState::Walking {
                mut x_rel,
                target_x_rel,
                dir_right: _,
            } => {
                let max_x = (win_w - pet_size).max(0);
                let target_x_rel = target_x_rel.clamp(0, max_x);
                x_rel = x_rel.clamp(0.0, max_x as f64);

                let diff = target_x_rel as f64 - x_rel;
                let step = 2.5;
                if diff.abs() <= step {
                    self.interaction_state = InteractionState::Sitting {
                        x_rel: target_x_rel,
                        duration_secs: gtk4::glib::random_int_range(5, 12) as u32,
                        start_time: now,
                    };
                } else {
                    x_rel += step * diff.signum();
                    self.interaction_state = InteractionState::Walking {
                        x_rel,
                        target_x_rel,
                        dir_right: diff > 0.0,
                    };
                }
            }
            InteractionState::Climbing {
                mut y_rel,
                target_y_rel,
                side,
                dir_down,
            } => {
                let max_y = (win_h - pet_size).max(0);
                let target_y_rel = target_y_rel.clamp(0, max_y);
                y_rel = y_rel.clamp(0.0, max_y as f64);

                let diff = target_y_rel as f64 - y_rel;
                let step = 1.8;
                if diff.abs() <= step {
                    if dir_down {
                        self.interaction_state = InteractionState::Climbing {
                            y_rel: target_y_rel as f64,
                            target_y_rel: 0,
                            side,
                            dir_down: false,
                        };
                    } else {
                        let x_rel = match side {
                            ClimbSide::Left => 0,
                            ClimbSide::Right => (win_w - pet_size).max(0),
                        };
                        self.interaction_state = InteractionState::Sitting {
                            x_rel,
                            duration_secs: gtk4::glib::random_int_range(5, 12) as u32,
                            start_time: now,
                        };
                    }
                } else {
                    y_rel += step * diff.signum();
                    self.interaction_state = InteractionState::Climbing {
                        y_rel,
                        target_y_rel,
                        side,
                        dir_down,
                    };
                }
            }
        }
    }

    pub fn get_sprite_coordinates(&mut self) -> (i32, i32) {
        let frame_size = 256;

        let row = match self.current_state {
            PetState::Idle => 0,
            PetState::Petting => 0,
            PetState::Typing => 1,
            PetState::WorkingHard => 1,
            PetState::Sleeping => 2,
            PetState::WalkingRight => {
                return (0 * frame_size, 3 * frame_size);
            }
            PetState::WalkingLeft => {
                return (1 * frame_size, 3 * frame_size);
            }
            PetState::ClimbingUp => {
                return (2 * frame_size, 3 * frame_size);
            }
            PetState::ClimbingDown => {
                return (3 * frame_size, 3 * frame_size);
            }
            PetState::Tracking { dx, dy } => {
                let col = if dx.abs() > dy.abs() {
                    if dx < 0.0 { 1 } else { 0 }
                } else {
                    if dy < 0.0 { 2 } else { 3 }
                };
                return (col * frame_size, 3 * frame_size);
            }
            PetState::Dancing => {
                self.current_frame = (self.current_frame + 1) % 4;
                return ((self.current_frame as i32) * frame_size, 0 * frame_size);
            }
            PetState::PortalOut | PetState::PortalIn => {
                self.current_frame = (self.current_frame + 1) % 8;
                return ((self.current_frame as i32) * frame_size, 0 * frame_size);
            }
            PetState::Dragged => {
                self.current_frame = (self.current_frame + 1) % 4;
                return ((self.current_frame as i32) * frame_size, 3 * frame_size);
            }
        };

        self.current_frame = (self.current_frame + 1) % 4;
        ((self.current_frame as i32) * frame_size, row * frame_size)
    }

    pub fn get_tick_interval(&self) -> u64 {
        match self.current_state {
            PetState::WorkingHard => 60,
            PetState::Typing => 120,
            PetState::Petting => 100,
            PetState::Sleeping => 500,
            PetState::Idle => 150,
            PetState::Tracking { .. } => 150,
            PetState::WalkingLeft | PetState::WalkingRight => 100,
            PetState::ClimbingUp | PetState::ClimbingDown => 120,
            PetState::Dancing => 160,
            PetState::PortalOut | PetState::PortalIn => 100,
            PetState::Dragged => 80,
        }
    }
}
