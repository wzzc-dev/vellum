fn handle_start_pause() -> Unit {
  match state.timer_state {
    Idle => {
      let mode = Focus
      let now = @timer.now_ms()
      let duration = mode.duration_ms()
      state.timer_state = Running({
        mode,
        started_at_ms: now,
        total_duration_ms: duration,
        completed_focus_count: state.completed_focus_count,
      })
      ignore(@timer.request_tick(1000U))
      refresh_ui()
    }
    Running(data) => {
      let now = @timer.now_ms()
      let elapsed = now - data.started_at_ms
      let remaining = if data.total_duration_ms > elapsed { data.total_duration_ms - elapsed } else { 0UL }
      if remaining > 0UL {
        state.timer_state = Paused({
          mode: data.mode,
          remaining_ms: remaining,
          total_duration_ms: data.total_duration_ms,
          completed_focus_count: data.completed_focus_count,
        })
        ignore(@timer.cancel_tick())
        refresh_ui()
      }
    }
    Paused(data) => {
      let now = @timer.now_ms()
      let elapsed_from_start = data.total_duration_ms - data.remaining_ms
      let new_started_at = now - elapsed_from_start
      state.timer_state = Running({
        mode: data.mode,
        started_at_ms: new_started_at,
        total_duration_ms: data.total_duration_ms,
        completed_focus_count: data.completed_focus_count,
      })
      ignore(@timer.request_tick(1000U))
      refresh_ui()
    }
    Completed => {
      handle_reset()
      handle_start_pause()
    }
  }
}

fn handle_reset() -> Unit {
  ignore(@timer.cancel_tick())
  state.timer_state = Idle
  state.completed_focus_count = 0
  refresh_ui()
}

fn handle_skip() -> Unit {
  match state.timer_state {
    Running(data) => {
      ignore(@timer.cancel_tick())
      if data.mode is Focus {
        state.completed_focus_count = state.completed_focus_count + 1
      }
      let next = next_mode(data.mode, state.completed_focus_count)
      let duration = next.duration_ms()
      state.timer_state = Paused({
        mode: next,
        remaining_ms: duration,
        total_duration_ms: duration,
        completed_focus_count: state.completed_focus_count,
      })
      ignore(@host.show_status_message("Skipped to " + next.label()))
      refresh_ui()
    }
    Paused(data) => {
      ignore(@timer.cancel_tick())
      if data.mode is Focus {
        state.completed_focus_count = state.completed_focus_count + 1
      }
      let next = next_mode(data.mode, state.completed_focus_count)
      let duration = next.duration_ms()
      state.timer_state = Paused({
        mode: next,
        remaining_ms: duration,
        total_duration_ms: duration,
        completed_focus_count: state.completed_focus_count,
      })
      ignore(@host.show_status_message("Skipped to " + next.label()))
      refresh_ui()
    }
    _ => ()
  }
}

fn handle_timer_tick(now : UInt64) -> Unit {
  match state.timer_state {
    Running(data) => {
      let elapsed = now - data.started_at_ms
      let remaining = if data.total_duration_ms > elapsed { data.total_duration_ms - elapsed } else { 0UL }
      if remaining <= 0UL {
        ignore(@timer.cancel_tick())
        if data.mode is Focus {
          state.completed_focus_count = state.completed_focus_count + 1
        }
        let next = next_mode(data.mode, state.completed_focus_count)
        let next_duration = next.duration_ms()
        state.timer_state = Paused({
          mode: next,
          remaining_ms: next_duration,
          total_duration_ms: next_duration,
          completed_focus_count: state.completed_focus_count,
        })
        ignore(@host.show_status_message(next.label() + " - Ready"))
        refresh_ui()
      } else {
        refresh_ui()
      }
    }
    _ => {
      ignore(@timer.cancel_tick())
    }
  }
}

pub fn activate(_ctx : @types.ActivationContext) -> Result[Unit, @types.ExtensionError] {
  refresh_ui()
  Ok(())
}

pub fn deactivate() -> Result[Unit, @types.ExtensionError] {
  ignore(@timer.cancel_tick())
  Ok(())
}

pub fn handle_event(event : @types.ExtensionEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_type == "timer.tick" {
    let ts = match event.timestamp_ms {
      Some(t) => t
      None => @timer.now_ms()
    }
    handle_timer_tick(ts)
  }
  Ok(())
}

pub fn execute_command(command_id : String) -> Result[Unit, @types.ExtensionError] {
  match command_id {
    "vellum.pomodoro.pomodoro.start-pause" => handle_start_pause()
    "vellum.pomodoro.pomodoro.reset" => handle_reset()
    "vellum.pomodoro.pomodoro.skip" => handle_skip()
    _ => ()
  }
  Ok(())
}

pub fn handle_ui_event(event : @types.UiEvent) -> Result[Unit, @types.ExtensionError] {
  if event.event_kind == "button.clicked" {
    match event.element_id {
      "pomodoro-start-pause" => handle_start_pause()
      "pomodoro-reset" => handle_reset()
      "pomodoro-skip" => handle_skip()
      _ => ()
    }
  }
  Ok(())
}

pub fn handle_hover(_hover_data : String) -> Result[FixedArray[Byte]?, @types.ExtensionError] {
  Ok(None)
}