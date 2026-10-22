//! Pure geometry for placing the island window. No Tauri calls, so it is unit-testable
//! across scale factors and monitor layouts (design.md §19: never hardcode coordinates).

use tauri::{PhysicalPosition, PhysicalSize};

use crate::settings::ScreenEdge;

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

/// On the left or right edge: against that side, `from_top` logical px down, `gap` logical px
/// in from the side. Kept inside the work area.
pub fn side_frame(
    work: WorkArea,
    scale_factor: f64,
    (logical_width, logical_height): (f64, f64),
    right: bool,
    from_top: i32,
    gap: u32,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let width = ((logical_width * scale_factor).round() as u32).min(work.width);
    let height = ((logical_height * scale_factor).round() as u32).min(work.height);
    let gap = (f64::from(gap) * scale_factor).round() as i32;
    let max_x = work.x + (work.width - width) as i32;
    let x = if right { max_x - gap } else { work.x + gap }.clamp(work.x, max_x);
    let max_y = work.y + (work.height - height) as i32;
    let y = (work.y + (f64::from(from_top) * scale_factor).round() as i32).clamp(work.y, max_y);
    (PhysicalPosition::new(x, y), PhysicalSize::new(width, height))
}

/// On a side, the cursor holds the bubble about this far (logical px) below its top.
const SIDE_GRAB: i32 = 8;

/// Where a dragged notch lands: the work-area edge closest to the cursor (physical px), and
/// its place along that edge in logical px: left/right of center on the top or bottom, down
/// from the top on a side.
pub fn snap(work: WorkArea, scale_factor: f64, cursor: (i32, i32)) -> (ScreenEdge, i32) {
    let x = cursor.0.clamp(work.x, work.x + work.width as i32);
    let y = cursor.1.clamp(work.y, work.y + work.height as i32);
    let to = [
        (ScreenEdge::Top, y - work.y),
        (ScreenEdge::Bottom, work.y + work.height as i32 - y),
        (ScreenEdge::Left, x - work.x),
        (ScreenEdge::Right, work.x + work.width as i32 - x),
    ];
    let edge = to
        .iter()
        .min_by_key(|(_, d)| *d)
        .map(|(e, _)| *e)
        .unwrap_or_default();
    (edge, along(work, scale_factor, cursor, edge))
}

/// The cursor's place along `edge` (Alt+drag keeps the edge and only slides along it).
pub fn along(work: WorkArea, scale_factor: f64, cursor: (i32, i32), edge: ScreenEdge) -> i32 {
    let x = cursor.0.clamp(work.x, work.x + work.width as i32);
    let y = cursor.1.clamp(work.y, work.y + work.height as i32);
    let logical = |px: i32| (f64::from(px) / scale_factor).round() as i32;
    if edge.is_side() {
        (logical(y - work.y) - SIDE_GRAB).max(0)
    } else {
        logical(x - (work.x + work.width as i32 / 2)).clamp(-4000, 4000)
    }
}

/// Whether a physical point is on this monitor (`origin`, `size` in physical px).
pub fn contains((x, y): (i32, i32), origin: (i32, i32), size: (u32, u32)) -> bool {
    x >= origin.0 && y >= origin.1 && x < origin.0 + size.0 as i32 && y < origin.1 + size.1 as i32
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
    fn snaps_to_the_closest_edge() {
        assert_eq!(snap(FHD, 1.0, (960, 5)), (ScreenEdge::Top, 0));
        assert_eq!(snap(FHD, 1.0, (1060, 1075)), (ScreenEdge::Bottom, 100));
        assert_eq!(snap(FHD, 1.0, (3, 500)), (ScreenEdge::Left, 492));
        assert_eq!(snap(FHD, 2.0, (1915, 400)), (ScreenEdge::Right, 192));
    }

    #[test]
    fn alt_drag_keeps_the_edge() {
        // Near the left side, but sliding along the top keeps it on the top.
        assert_eq!(along(FHD, 1.0, (10, 600), ScreenEdge::Top), 10 - 960);
        assert_eq!(along(FHD, 1.0, (10, 600), ScreenEdge::Right), 592);
    }

    #[test]
    fn finds_the_monitor_under_the_cursor() {
        assert!(contains((2000, 100), (1920, 0), (1280, 720)));
        assert!(!contains((1919, 100), (1920, 0), (1280, 720)));
    }

    #[test]
    fn side_frame_hugs_the_side_and_stays_on_screen() {
        let (pos, size) = side_frame(FHD, 1.0, (200.0, 40.0), false, 300, 0);
        assert_eq!((pos.x, pos.y, size.width), (0, 300, 200));
        let (pos, _) = side_frame(FHD, 1.0, (200.0, 40.0), true, 5000, 6);
        assert_eq!((pos.x, pos.y), (1920 - 200 - 6, 1032 - 40));
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
