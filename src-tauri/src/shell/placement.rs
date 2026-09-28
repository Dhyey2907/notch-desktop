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
    fn clamps_to_work_area() {
        let (pos, size) = top_center_frame(FHD, 2.0, 1200.0, 900.0);
        assert_eq!(size, PhysicalSize::new(1920, 1032));
        assert_eq!(pos.x, 0);
    }
}
