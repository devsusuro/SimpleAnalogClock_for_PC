use std::time::{Duration, Instant};

use eframe::egui::{self, Pos2, Rect, Vec2, ViewportCommand, pos2};

const GAP: f32 = 8.0;
const SNAP_DISTANCE: f32 = 12.0;

#[derive(Clone, Copy)]
pub struct Placement {
    pub clock: Rect,
    work_area: Option<Rect>,
}

impl Placement {
    pub fn current(ctx: &egui::Context, frame: &eframe::Frame) -> Option<Self> {
        let clock = clock_rect(ctx, frame)?;
        Some(Self {
            clock,
            work_area: work_area(ctx, frame, clock),
        })
    }

    pub fn menu_position(self, pointer: Pos2, size: Vec2) -> Pos2 {
        let mut position = pointer + Vec2::splat(GAP);
        let Some(area) = self.work_area else {
            return position;
        };
        if position.x + size.x > area.right() {
            position.x = pointer.x - GAP - size.x;
        }
        if position.y + size.y > area.bottom() {
            position.y = pointer.y - GAP - size.y;
        }
        fit(area, position, size)
    }

    pub fn options_position(self, size: Vec2) -> Pos2 {
        let left = self.clock.left() - size.x - GAP;
        let preferred = pos2(left, self.clock.top());
        let Some(area) = self.work_area else {
            return preferred;
        };
        let x = if left >= area.left() {
            left
        } else {
            self.clock.right() + GAP
        };
        fit(area, pos2(x, self.clock.top()), size)
    }

    pub fn submenu_position(
        self,
        menu_position: Pos2,
        menu_size: Vec2,
        row_top: f32,
        size: Vec2,
    ) -> Pos2 {
        let right = pos2(
            menu_position.x + menu_size.x,
            menu_position.y + row_top - 8.0,
        );
        let Some(area) = self.work_area else {
            return right;
        };
        let position = if right.x + size.x <= area.right() {
            right
        } else {
            pos2(menu_position.x - size.x, right.y)
        };
        fit(area, position, size)
    }

    pub fn can_snap(self) -> bool {
        self.work_area.is_some()
    }

    pub fn resized_clock_position(self, size: Vec2) -> Pos2 {
        let mut position = self.clock.center() - size / 2.0;
        let Some(area) = self.work_area else {
            return position;
        };
        if (self.clock.left() - area.left()).abs() <= 1.0 {
            position.x = area.left();
        } else if (self.clock.right() - area.right()).abs() <= 1.0 {
            position.x = area.right() - size.x;
        }
        if (self.clock.top() - area.top()).abs() <= 1.0 {
            position.y = area.top();
        } else if (self.clock.bottom() - area.bottom()).abs() <= 1.0 {
            position.y = area.bottom() - size.y;
        }
        fit(area, position, size)
    }

    pub fn snapped_position(self) -> Option<Pos2> {
        let area = self.work_area?;
        let snap_axis = |start: f32, end: f32, near: f32, far: f32| {
            let near_distance = (start - near).abs();
            let far_distance = (end - far).abs();
            if near_distance <= SNAP_DISTANCE && near_distance <= far_distance {
                near
            } else if far_distance <= SNAP_DISTANCE {
                far - (end - start)
            } else {
                start
            }
        };
        let pos = pos2(
            snap_axis(
                self.clock.left(),
                self.clock.right(),
                area.left(),
                area.right(),
            ),
            snap_axis(
                self.clock.top(),
                self.clock.bottom(),
                area.top(),
                area.bottom(),
            ),
        );
        (pos != self.clock.min).then_some(pos)
    }
}

pub struct Drag {
    origin: Option<Pos2>,
    started_at: Instant,
    #[cfg(windows)]
    live: Option<LiveDrag>,
}

impl Drag {
    pub fn begin(ctx: &egui::Context, frame: &eframe::Frame, placement: Option<Placement>) -> Self {
        #[cfg(windows)]
        let live = LiveDrag::begin(frame);
        #[cfg(windows)]
        let use_native_move = live.is_none();
        #[cfg(not(windows))]
        let use_native_move = {
            let _ = frame;
            true
        };

        if use_native_move {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        Self {
            origin: placement.map(|p| p.clock.min),
            started_at: Instant::now(),
            #[cfg(windows)]
            live,
        }
    }

    pub fn update(&self, ctx: &egui::Context, frame: &eframe::Frame, snap_to_edges: bool) -> bool {
        let down = primary_button_down(ctx);
        #[cfg(windows)]
        if let Some(live) = &self.live {
            live.move_window(frame, ctx.pixels_per_point(), snap_to_edges);
            if down {
                ctx.request_repaint_after(Duration::from_millis(10));
            }
            return down;
        }
        if down && self.started_at.elapsed() < Duration::from_secs(20) {
            ctx.request_repaint_after(Duration::from_millis(50));
            return true;
        }
        if !down && snap_to_edges {
            if let Some(position) = Placement::current(ctx, frame)
                .filter(|p| Some(p.clock.min) != self.origin)
                .and_then(Placement::snapped_position)
            {
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(position));
            }
        }
        false
    }
}

fn fit(area: Rect, position: Pos2, size: Vec2) -> Pos2 {
    pos2(
        position
            .x
            .clamp(area.left(), (area.right() - size.x).max(area.left())),
        position
            .y
            .clamp(area.top(), (area.bottom() - size.y).max(area.top())),
    )
}

#[cfg(windows)]
fn work_area(ctx: &egui::Context, frame: &eframe::Frame, _clock: Rect) -> Option<Rect> {
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
    let handle = window_handle(frame)?;
    // The frame owns the live HWND.
    let monitor = unsafe { MonitorFromWindow(handle, MONITOR_DEFAULTTONEAREST) };
    monitor_info(monitor).map(|info| screen_rect(ctx.pixels_per_point(), info.rcWork))
}

#[cfg(windows)]
fn monitor_info(
    monitor: windows_sys::Win32::Graphics::Gdi::HMONITOR,
) -> Option<windows_sys::Win32::Graphics::Gdi::MONITORINFO> {
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // `info` is writable for the duration of the call.
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info) };
    (ok != 0).then_some(info)
}

#[cfg(windows)]
fn monitor_for_rect(
    rect: &windows_sys::Win32::Foundation::RECT,
) -> Option<windows_sys::Win32::Graphics::Gdi::MONITORINFO> {
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromRect};
    // The rectangle is valid for the duration of the call.
    monitor_info(unsafe { MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST) })
}

#[cfg(windows)]
fn clock_rect(ctx: &egui::Context, frame: &eframe::Frame) -> Option<Rect> {
    let rect = window_handle(frame).and_then(window_rect);
    rect.map(|rect| screen_rect(ctx.pixels_per_point(), rect))
        .or_else(|| ctx.input(|input| input.viewport().outer_rect))
}

#[cfg(not(windows))]
fn clock_rect(ctx: &egui::Context, _frame: &eframe::Frame) -> Option<Rect> {
    ctx.input(|input| input.viewport().outer_rect)
}

#[cfg(windows)]
fn screen_rect(scale: f32, rect: windows_sys::Win32::Foundation::RECT) -> Rect {
    Rect::from_min_max(
        pos2(rect.left as f32 / scale, rect.top as f32 / scale),
        pos2(rect.right as f32 / scale, rect.bottom as f32 / scale),
    )
}

#[cfg(windows)]
fn window_handle(frame: &eframe::Frame) -> Option<windows_sys::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let RawWindowHandle::Win32(handle) = frame.window_handle().ok()?.as_raw() else {
        return None;
    };
    Some(handle.hwnd.get() as _)
}

#[cfg(windows)]
fn window_rect(
    handle: windows_sys::Win32::Foundation::HWND,
) -> Option<windows_sys::Win32::Foundation::RECT> {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::GetWindowRect};
    let mut rect = RECT::default();
    // The HWND belongs to the frame and the rectangle is valid writable storage.
    (unsafe { GetWindowRect(handle, &mut rect) } != 0).then_some(rect)
}

#[cfg(windows)]
pub fn keep_clock_on_screen(frame: &eframe::Frame) {
    if let Some(handle) = window_handle(frame)
        && let Some(rect) = window_rect(handle)
        && let Some(monitor) = monitor_for_rect(&rect)
    {
        move_window_if_needed(handle, rect, limit_clock_position(rect, monitor.rcMonitor));
    }
}

#[cfg(windows)]
fn limit_clock_position(
    clock: windows_sys::Win32::Foundation::RECT,
    monitor: windows_sys::Win32::Foundation::RECT,
) -> (i32, i32) {
    let limit_axis = |start: i32, end: i32, near: i32, far: i32| {
        let size = end - start;
        // Require strictly more than half of the window on each axis, in physical pixels.
        let visible = (size / 2 + 1).min(far - near);
        start.clamp(near - (size - visible), far - visible)
    };
    (
        limit_axis(clock.left, clock.right, monitor.left, monitor.right),
        limit_axis(clock.top, clock.bottom, monitor.top, monitor.bottom),
    )
}

#[cfg(windows)]
fn move_window_if_needed(
    handle: windows_sys::Win32::Foundation::HWND,
    current: windows_sys::Win32::Foundation::RECT,
    (x, y): (i32, i32),
) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };
    if (current.left, current.top) == (x, y) {
        return;
    }
    // Move only the live window, bypassing Windows' optional outline-only drag loop.
    unsafe {
        SetWindowPos(
            handle,
            std::ptr::null_mut(),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

#[cfg(windows)]
struct LiveDrag {
    offset_x: i32,
    offset_y: i32,
}

#[cfg(windows)]
impl LiveDrag {
    fn begin(frame: &eframe::Frame) -> Option<Self> {
        let start = window_rect(window_handle(frame)?)?;
        let cursor = cursor_position()?;
        Some(Self {
            offset_x: cursor.x - start.left,
            offset_y: cursor.y - start.top,
        })
    }

    fn move_window(&self, frame: &eframe::Frame, scale: f32, snap_to_edges: bool) {
        let Some(handle) = window_handle(frame) else {
            return;
        };
        let Some(cursor) = cursor_position() else {
            return;
        };
        self.move_window_to(handle, cursor, scale, snap_to_edges);
    }

    fn move_window_to(
        &self,
        handle: windows_sys::Win32::Foundation::HWND,
        cursor: windows_sys::Win32::Foundation::POINT,
        scale: f32,
        snap_to_edges: bool,
    ) {
        use windows_sys::Win32::Foundation::RECT;
        let Some(current) = window_rect(handle) else {
            return;
        };
        let (x, y) = (cursor.x - self.offset_x, cursor.y - self.offset_y);
        let width = current.right - current.left;
        let height = current.bottom - current.top;
        let rect_at = |left, top| RECT {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        let mut candidate = rect_at(x, y);
        let Some(monitor) = monitor_for_rect(&candidate) else {
            move_window_if_needed(handle, current, (x, y));
            return;
        };
        if snap_to_edges {
            // Check the unsnapped cursor position so moving away releases the clock.
            let placement = Placement {
                clock: screen_rect(scale, candidate),
                work_area: Some(screen_rect(scale, monitor.rcWork)),
            };
            if let Some(position) = placement.snapped_position() {
                candidate = rect_at(
                    (position.x * scale).round() as i32,
                    (position.y * scale).round() as i32,
                );
            }
        }
        move_window_if_needed(
            handle,
            current,
            limit_clock_position(candidate, monitor.rcMonitor),
        );
    }
}

#[cfg(windows)]
fn cursor_position() -> Option<windows_sys::Win32::Foundation::POINT> {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
    let mut point = POINT::default();
    // `point` is writable for the duration of the call.
    (unsafe { GetCursorPos(&mut point) } != 0).then_some(point)
}

#[cfg(not(windows))]
fn work_area(ctx: &egui::Context, _frame: &eframe::Frame, clock: Rect) -> Option<Rect> {
    let size = ctx.input(|input| input.viewport().monitor_size)?;
    let area = Rect::from_min_size(Pos2::ZERO, size);
    area.contains(clock.center()).then_some(area)
}

#[cfg(windows)]
fn primary_button_down(_ctx: &egui::Context) -> bool {
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
        WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON},
    };
    // Poll only during a drag so it still ends if the pointer leaves the clock.
    unsafe {
        let button = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        };
        GetAsyncKeyState(i32::from(button)) < 0
    }
}

#[cfg(not(windows))]
fn primary_button_down(ctx: &egui::Context) -> bool {
    ctx.input(|input| input.pointer.primary_down())
}

#[cfg(all(test, windows))]
mod tests {
    use super::limit_clock_position;
    use windows_sys::Win32::Foundation::RECT;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn requires_more_than_half_visible_on_both_axes() {
        let monitor = rect(0, 0, 1920, 1080);
        assert_eq!(
            limit_clock_position(rect(-60, -50, 60, 51), monitor),
            (-59, -50)
        );
        assert_eq!(
            limit_clock_position(rect(1860, 1030, 1980, 1131), monitor),
            (1859, 1029)
        );
        assert_eq!(
            limit_clock_position(rect(80, 90, 200, 211), monitor),
            (80, 90)
        );
    }

    #[test]
    fn limits_position_again_when_a_monitor_is_removed() {
        let clock = rect(-1300, 1300, -1180, 1421);
        assert_eq!(
            limit_clock_position(clock, rect(-1920, 300, 0, 1500)),
            (-1300, 1300)
        );
        assert_eq!(
            limit_clock_position(clock, rect(0, 0, 1920, 1080)),
            (-59, 1019)
        );
    }
}
