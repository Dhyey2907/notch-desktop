//! Pure geometry for placing the island window. No Tauri calls, so it is unit-testable
//! across scale factors and monitor layouts (design.md §19: never hardcode coordinates).

use tauri::{PhysicalPosition, PhysicalSize};

/// A monitor's work area (screen minus taskbar) in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Physical frame for a window of `logical` size, centered horizontally at the top
/// of `work`. The size is clamped so the window never exceeds the work area.
pub fn top_center_frame(
    work: WorkArea,
    scale_factor: f64,
    logical_width: f64,
    logical_height: f64,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let width = ((logical_width * scale_factor).round() as u32).min(work.width);
    let height = ((logical_height * scale_factor).round() as u32).min(work.height);
    let x = work.x + ((work.width - width) / 2) as i32;
    (PhysicalPosition::new(x, work.y), PhysicalSize::new(width, height))
}

/// The same frame on the bottom edge: centered, standing on the top of the taskbar.
pub fn bottom_center_frame(
    work: WorkArea,
    scale_factor: f64,
    logical_width: f64,
    logical_height: f64,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let (top, size) = top_center_frame(work, scale_factor, logical_width, logical_height);
    let y = work.y + (work.height - size.height) as i32;
    (PhysicalPosition::new(top.x, y), size)
}

/// Move a placed frame by the user's offset (logical px), keeping it inside the work area.
/// On the bottom edge `offset_y` lifts it up instead of pushing it down.
pub fn nudge(
    (position, size): (PhysicalPosition<i32>, PhysicalSize<u32>),
    work: WorkArea,
    scale_factor: f64,
    offset_x: i32,
    offset_y: u32,
    bottom: bool,
) -> PhysicalPosition<i32> {
    let max_x = work.x + (work.width - size.width) as i32;
    let max_y = work.y + (work.height - size.height) as i32;
    let x = position.x + (f64::from(offset_x) * scale_factor).round() as i32;
    let dy = (f64::from(offset_y) * scale_factor).round() as i32;
    let y = if bottom { position.y - dy } else { position.y + dy };
    PhysicalPosition::new(x.clamp(work.x, max_x), y.clamp(work.y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: WorkArea = WorkArea {
        x: 0,
        y: 0,
        width: 1920,
        height: 1032,
    };

    #[test]
    fn centers_at_100_percent() {
        let (pos, size) = top_center_frame(FHD, 1.0, 200.0, 40.0);
        assert_eq!(size, PhysicalSize::new(200, 40));
        assert_eq!(pos, PhysicalPosition::new(860, 0));
    }

    #[test]
    fn scales_size_and_recenters_at_150_percent() {
        let work = WorkArea {
            x: 0,
            y: 0,
            width: 2560,
            height: 1392,
        };
        let (pos, size) = top_center_frame(work, 1.5, 200.0, 40.0);
        assert_eq!(size, PhysicalSize::new(300, 60));
        assert_eq!(pos, PhysicalPosition::new(1130, 0));
    }

    #[test]
    fn respects_offset_monitors_and_top_taskbars() {
        let work = WorkArea {
            x: -1920,
            y: 48,
            width: 1920,
            height: 1032,
        };
        let (pos, _) = top_center_frame(work, 1.0, 200.0, 40.0);
        assert_eq!(pos, PhysicalPosition::new(-1060, 48));
    }

    #[test]
    fn nudges_by_the_users_offset_but_stays_on_screen() {
        let frame = top_center_frame(FHD, 1.5, 200.0, 40.0);
        assert_eq!(
            nudge(frame, FHD, 1.5, 100, 10, false),
            PhysicalPosition::new(960, 15)
        );
        assert_eq!(
            nudge(frame, FHD, 1.5, -5000, 0, false),
            PhysicalPosition::new(0, 0)
        );
        assert_eq!(
            nudge(frame, FHD, 1.5, 5000, 0, false),
            PhysicalPosition::new(1620, 0)
        );
    }

    #[test]
    fn bottom_edge_stands_on_the_taskbar_and_lifts_up() {
        let frame = bottom_center_frame(FHD, 1.0, 200.0, 40.0);
        assert_eq!(frame.0, PhysicalPosition::new(860, 992));
        assert_eq!(
            nudge(frame, FHD, 1.0, 0, 10, true),
            PhysicalPosition::new(860, 982)
        );
        // A taskbar on top: the work area starts lower, the bottom is the screen's bottom.
        let work = WorkArea {
            x: 0,
            y: 48,
            width: 1920,
            height: 1032,
        };
        assert_eq!(bottom_center_frame(work, 1.0, 200.0, 40.0).0.y, 1040);
    }

    #[test]
    fn clamps_to_work_area() {
        let (pos, size) = top_center_frame(FHD, 2.0, 1200.0, 900.0);
        assert_eq!(size, PhysicalSize::new(1920, 1032));
        assert_eq!(pos.x, 0);
    }
}
