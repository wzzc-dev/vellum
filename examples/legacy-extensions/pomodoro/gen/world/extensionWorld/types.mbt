pub(all) enum TimerMode {
  Focus
  ShortBreak
  LongBreak
} derive(Debug, Eq)

pub(all) enum TimerState {
  Idle
  Running(RunningData)
  Paused(PausedData)
  Completed
} derive(Debug, Eq)

pub(all) struct RunningData {
  mode : TimerMode
  started_at_ms : UInt64
  total_duration_ms : UInt64
  completed_focus_count : Int
} derive(Debug, Eq)

pub(all) struct PausedData {
  mode : TimerMode
  remaining_ms : UInt64
  total_duration_ms : UInt64
  completed_focus_count : Int
} derive(Debug, Eq)

pub fn TimerMode::duration_ms(self : TimerMode) -> UInt64 {
  match self {
    Focus => 25UL * 60UL * 1000UL
    ShortBreak => 5UL * 60UL * 1000UL
    LongBreak => 15UL * 60UL * 1000UL
  }
}

pub fn TimerMode::label(self : TimerMode) -> String {
  match self {
    Focus => "Focus"
    ShortBreak => "Short Break"
    LongBreak => "Long Break"
  }
}

pub fn next_mode(mode : TimerMode, completed_focus_count : Int) -> TimerMode {
  match mode {
    Focus => {
      if completed_focus_count % 4 == 0 { LongBreak } else { ShortBreak }
    }
    ShortBreak => Focus
    LongBreak => Focus
  }
}

pub(all) struct State {
  mut timer_state : TimerState
  mut completed_focus_count : Int
}

let state : State = {
  timer_state: Idle,
  completed_focus_count: 0,
}
