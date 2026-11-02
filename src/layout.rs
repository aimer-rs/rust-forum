//! How the app lays itself out for the window it has, following Apple's
//! "Designing for iPhone Duo": one layout that resizes, not one per device.
//!
//! The outer display of an iPhone Duo is wider and shorter than any other
//! iPhone, so the system moves its bars to the side; the inner display is
//! larger and a fold may divide it. The app answers the same questions from the
//! window alone, so it adapts on any device that is shaped alike, and in a
//! window that is resized by hand.

use aimer::screen::{HingeStatus, MediaQuery, Panes};
use aimer::style::LayoutSpacing;
use aimer::{BuildContext, SafeAreaInsets};

use crate::components::side_bar::{RAIL_MARGIN, RAIL_SPACE, RAIL_WIDTH, rail_height};

/// Windows narrower than this (logical pixels) are phone-shaped.
pub const PHONE_MAX_WIDTH: f32 = 600.0;
/// Windows shorter than this are too short for bars above and below the page: the
/// outer display of an iPhone Duo, or a phone held on its side.
pub const SHORT_HEIGHT: f32 = 480.0;
/// A short window wider than this is a computer's, with room for the sidebar.
pub const RAIL_MAX_WIDTH: f32 = 1000.0;
/// A narrow window whose height is less than this many times its width is nearly
/// square, as the outer display of an iPhone Duo is: too short for a bar along the
/// bottom and a header along the top, whatever its width. A phone in portrait is
/// about twice as tall as it is wide.
pub const NEAR_SQUARE_MAX_ASPECT: f32 = 1.6;

/// What the window's shape asks of the app's chrome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellLayout {
    /// A narrow window: the page has the whole window, and the tab bar floats along
    /// the bottom.
    Phone,
    /// A wide but short window, or a nearly square one: the page keeps its height,
    /// and the tabs stand in a vertical rail on the trailing edge, where the system
    /// puts its own controls.
    Rail,
    /// A large window: the sidebar beside the page.
    Desktop,
}

impl ShellLayout {
    /// The layout for a window of `width` by `height` logical pixels.
    pub fn classify(width: f32, height: f32) -> Self {
        if width < PHONE_MAX_WIDTH {
            if height < width * NEAR_SQUARE_MAX_ASPECT {
                Self::Rail
            } else {
                Self::Phone
            }
        } else if height < SHORT_HEIGHT && width < RAIL_MAX_WIDTH {
            Self::Rail
        } else {
            Self::Desktop
        }
    }

    /// Whether the pages are laid out as a phone's: touch-sized and with the whole
    /// window, no sidebar.
    pub fn is_compact(self) -> bool {
        self != Self::Desktop
    }
}

/// The longest side (logical pixels) of a window that can be the full fold of an
/// iPhone Duo: larger than the outer display, still short of a computer's.
pub const FOLD_MAX_SIDE: f32 = 1200.0;
/// How much longer than the other a side of the full fold may be: the unfolded
/// display is nearly square, not stretched like a phone's.
pub const FOLD_MAX_ASPECT: f32 = 1.6;

/// How the full fold of an iPhone Duo is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldPosture {
    /// Wider than tall: the buttons of the bar stay on the left of the page.
    Landscape,
    /// Taller than wide: the buttons stand in a column on the trailing edge.
    Portrait,
}

/// Whether a window of `width` by `height` is the full fold of an iPhone Duo, and
/// how it is held.
///
/// The hinge decides when the device reports it: open (`PartiallyOpen` or
/// `FullyOpen`), the window is the full fold at any size and shape; `Closed`, it is the outer
/// display and never is. Without a hinge reading (`None` or `Unknown`), a large,
/// nearly square window, any upright window with the sidebar's layout, or any window
/// the system reports as divided by a fold (`divided`), is taken for it.
pub fn fold_posture(
    width: f32,
    height: f32,
    divided: bool,
    hinge: Option<HingeStatus>,
) -> Option<FoldPosture> {
    let (long, short) = (width.max(height), width.min(height));
    match hinge {
        // An open device is the full fold whatever the window's shape: held upright
        // it can be tall and narrow, which the size check would take for a phone's.
        Some(HingeStatus::PartiallyOpen | HingeStatus::FullyOpen) => {}
        Some(HingeStatus::Closed) => return None,
        Some(HingeStatus::Unknown) | None => {
            if ShellLayout::classify(width, height) != ShellLayout::Desktop {
                return None;
            }
            // Held upright the window is taller than wide and has the sidebar's
            // layout, which no phone has: it is the full fold however tall it is.
            // On its side it has to be large and nearly square to be told from a
            // computer's window.
            let upright = height > width;
            if !upright && !divided && (long > FOLD_MAX_SIDE || long > short * FOLD_MAX_ASPECT) {
                return None;
            }
        }
    }
    Some(if width > height {
        FoldPosture::Landscape
    } else {
        FoldPosture::Portrait
    })
}

/// The two areas a fold leaves the window, if it divides it. The hinge has the last
/// word: the system's report of the fold can lag behind it, and a device that has
/// opened fully is not divided whatever the last report said.
pub fn fold_panes(ctx: &BuildContext) -> Option<Panes> {
    let panes = MediaQuery::panes_of(ctx)?;
    let open = matches!(
        MediaQuery::hinge_of(ctx).map(|hinge| hinge.status),
        Some(HingeStatus::FullyOpen)
    );
    if open { None } else { Some(panes) }
}

/// The posture of the full fold in the window being built for, if it is one.
pub fn current_fold_posture(ctx: &BuildContext) -> Option<FoldPosture> {
    let size = MediaQuery::of(ctx).size;
    let divided = fold_panes(ctx).is_some();
    let hinge = MediaQuery::hinge_of(ctx).map(|hinge| hinge.status);
    fold_posture(size.width, size.height, divided, hinge)
}

/// The layout of the window being built for, rebuilding the widget only where the
/// answer changes, not on every pixel of a resize.
pub fn shell_layout(ctx: &BuildContext) -> ShellLayout {
    MediaQuery::select(ctx, |media| ShellLayout::classify(media.size.width, media.size.height))
}

/// How far from each edge of the window the area for content starts when a fold
/// divides it: everything outside the larger of the two panes.
///
/// Nothing is drawn across the fold, and the layout is not changed more than
/// that: it is the same layout in a smaller room, so controls do not jump as the
/// device is folded. With panes of the same size the leading one is kept.
pub fn fold_insets(width: f32, height: f32, panes: Option<&Panes>) -> SafeAreaInsets {
    let Some(panes) = panes else {
        return SafeAreaInsets::ZERO;
    };
    let area = |bounds: &aimer::screen::Bounds| bounds.width().max(0.0) * bounds.height().max(0.0);
    let kept = if area(&panes.first) >= area(&panes.second) {
        &panes.first
    } else {
        &panes.second
    };
    SafeAreaInsets::new(
        kept.left,
        kept.top,
        width - kept.right,
        height - kept.bottom,
    )
}

/// What of a fold's `insets` the content keeps clear of. With the sidebar the
/// window is not narrowed to one pane: the pane can end just past the sidebar and
/// leave the page a sliver. The sidebar keeps its width and the page takes the
/// rest; only the fold's top and bottom are kept.
pub fn sidebar_fold_insets(fold: SafeAreaInsets, sidebar: bool) -> SafeAreaInsets {
    if sidebar {
        SafeAreaInsets::new(0.0, fold.top, 0.0, fold.bottom)
    } else {
        fold
    }
}

/// The room around the window's content that the app keeps clear: the side
/// safe areas, which hold the system's controls on the vertical axis, and the
/// fold. The top and bottom are left to the pages, which already space
/// themselves from the bars there.
pub fn content_insets(ctx: &BuildContext) -> SafeAreaInsets {
    let system = ctx.watch_safe_area_insets();
    let sides = SafeAreaInsets::new(system.left, 0.0, system.right, 0.0);
    match fold_panes(ctx) {
        Some(panes) => {
            let size = MediaQuery::of(ctx).size;
            let fold = fold_insets(size.width, size.height, Some(&panes));
            sides.max(sidebar_fold_insets(fold, shell_layout(ctx) == ShellLayout::Desktop))
        }
        None => sides,
    }
}

/// How far left of the middle of the system's band its own icons stand: the
/// camera, the time and the connection indicator sit a little toward the inside
/// of the band, so centring the controls on the band leaves them right of the
/// icons. Measured from a screenshot of the outer display.
pub const STATUS_ICON_SHIFT: f32 = 6.0;

/// How tall the buttons of the column on a post are, back, like and comment, with
/// the gaps between them: the room the column needs below a centred rail.
pub const COLUMN_BELOW_HEIGHT: f32 = 180.0;

/// The bottom of the tab rail, with `tabs` in it, when it stands in the middle of
/// the trailing edge of a window `height` tall.
fn centred_rail_bottom(height: f32, tabs: usize) -> f32 {
    (height + rail_height(tabs)) / 2.0
}

/// Whether the tab rail stands in the middle of the trailing edge: when the window
/// is tall enough to keep the column of buttons below it. Otherwise the rail keeps
/// the bottom corner and the column the room above it.
pub fn rail_centred(height: f32, tabs: usize) -> bool {
    centred_rail_bottom(height, tabs) + RAIL_MARGIN as f32 * 2.0 + COLUMN_BELOW_HEIGHT <= height
}

/// Where the tab rail stands along the trailing edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RailPlace {
    /// Halfway down the edge.
    Middle,
    /// Pushed down, its top this far from the top of the window, to clear the
    /// column of buttons above it.
    Below(f32),
    /// In the bottom corner.
    Corner,
}

/// Where the tab rail stands in a window `height` tall, with `tabs` in it, when the
/// column of buttons reaches down to `clear` from the top (`0.0` when the page has
/// none): in the middle, pushed down just far enough to clear the column when the
/// middle is too high, or in the corner when there is no room below the column.
pub fn rail_place(height: f32, tabs: usize, clear: f32) -> RailPlace {
    if !rail_centred(height, tabs) {
        return RailPlace::Corner;
    }
    let rail = rail_height(tabs);
    let middle = (height - rail) / 2.0;
    let lowest = clear + RAIL_MARGIN as f32;
    if middle >= lowest {
        RailPlace::Middle
    } else if lowest + rail + RAIL_MARGIN as f32 <= height {
        RailPlace::Below(lowest)
    } else {
        RailPlace::Corner
    }
}

/// How far from the trailing edge the tab rail and the column of buttons float,
/// given the system's safe area there (`system_right`).
///
/// Where the system keeps a band of its own along the edge (the camera, the time
/// and the connection indicator of an iPhone Duo's outer display), the controls
/// stand in the middle of it, on the same vertical line as the system's; where
/// it keeps none, they keep the usual margin.
pub fn rail_margin(system_right: f32) -> u32 {
    if system_right >= RAIL_SPACE {
        ((system_right - RAIL_WIDTH) / 2.0 + STATUS_ICON_SHIFT).round() as u32
    } else {
        RAIL_MARGIN
    }
}

/// How much of the window's width, from the trailing edge, the rail and the column
/// take: what a page beside them keeps clear. It holds the system's band, so the
/// page is not under the controls that stand in it.
pub fn rail_band(insets: &SafeAreaInsets) -> f32 {
    insets.right.max(RAIL_SPACE)
}

/// The padding of the shell in the rail layout: `insets` on every side but the
/// trailing one, where the rail and the column float over the system's band
/// instead (see [`rail_band`]).
pub fn rail_padding(insets: SafeAreaInsets) -> LayoutSpacing {
    padding_of(SafeAreaInsets::new(insets.left, insets.top, 0.0, insets.bottom))
}

/// `insets` as the padding of a container, to the nearest pixel.
pub fn padding_of(insets: SafeAreaInsets) -> LayoutSpacing {
    LayoutSpacing::new()
        .left(insets.left.round() as u32)
        .top(insets.top.round() as u32)
        .right(insets.right.round() as u32)
        .bottom(insets.bottom.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aimer::screen::Bounds;

    #[test]
    fn insets_become_the_padding_of_their_own_edges() {
        let padding = padding_of(SafeAreaInsets::new(1.4, 2.0, 3.6, 0.0));
        assert_eq!(
            padding,
            LayoutSpacing::new().left(1u32).top(2u32).right(4u32).bottom(0u32)
        );
    }

    #[test]
    fn the_rail_is_centred_only_where_the_column_fits_below_it() {
        // The outer display held upright, with all five tabs.
        assert!(rail_centred(678.0, 5));
        // On its side there is no room below the rail.
        assert!(!rail_centred(400.0, 5));
        assert!(!rail_centred(600.0, 5));
    }

    #[test]
    fn the_full_fold_is_a_large_nearly_square_window_or_one_a_fold_divides() {
        use FoldPosture::*;
        assert_eq!(fold_posture(820.0, 600.0, false, None), Some(Landscape));
        assert_eq!(fold_posture(700.0, 900.0, false, None), Some(Portrait));
        // Held upright it is the full fold however tall it is.
        assert_eq!(fold_posture(820.0, 1400.0, false, None), Some(Portrait));
        assert_eq!(fold_posture(900.0, 1500.0, false, Some(HingeStatus::Unknown)), Some(Portrait));
        // A computer's window is too wide, a phone's and the outer display's
        // are not the sidebar's layout.
        assert_eq!(fold_posture(1600.0, 900.0, false, None), None);
        assert_eq!(fold_posture(1100.0, 500.0, false, None), None);
        assert_eq!(fold_posture(466.0, 678.0, false, None), None);
        assert_eq!(fold_posture(740.0, 400.0, false, None), None);
        // A fold divides the window: it is the full fold at any size.
        assert_eq!(fold_posture(1600.0, 900.0, true, None), Some(Landscape));
        assert_eq!(fold_posture(466.0, 678.0, true, None), None);
    }

    #[test]
    fn an_open_hinge_makes_the_full_fold_at_any_size_and_a_closed_one_never() {
        use FoldPosture::*;
        use HingeStatus::*;
        // Open, even in a window too wide or too stretched for the size check.
        assert_eq!(fold_posture(1600.0, 900.0, false, Some(FullyOpen)), Some(Landscape));
        assert_eq!(fold_posture(700.0, 1300.0, false, Some(PartiallyOpen)), Some(Portrait));
        // Closed: the outer display, even in a window the size check would take.
        assert_eq!(fold_posture(820.0, 600.0, false, Some(Closed)), None);
        assert_eq!(fold_posture(820.0, 600.0, true, Some(Closed)), None);
        // Held upright the open window can be tall and narrow.
        assert_eq!(fold_posture(466.0, 1356.0, false, Some(FullyOpen)), Some(Portrait));
        // The outer display's own layouts are not the full fold without a hinge.
        assert_eq!(fold_posture(466.0, 678.0, false, Some(Closed)), None);
        assert_eq!(fold_posture(466.0, 678.0, false, Some(Unknown)), None);
        // A hinge that is not read falls back to the size check.
        assert_eq!(fold_posture(820.0, 600.0, false, Some(Unknown)), Some(Landscape));
        assert_eq!(fold_posture(1600.0, 900.0, false, Some(Unknown)), None);
    }

    #[test]
    fn the_rail_is_pushed_down_only_as_far_as_the_column_needs() {
        // No column: the middle.
        assert_eq!(rail_place(678.0, 5, 0.0), RailPlace::Middle);
        // A column that ends above the middle leaves it where it is.
        assert_eq!(rail_place(678.0, 5, 200.0), RailPlace::Middle);
        // A column that reaches into the middle (the rail starts at 213) pushes the
        // rail to 8 px below it.
        assert_eq!(rail_place(678.0, 5, 216.0), RailPlace::Below(224.0));
        // Too low for the rail to fit under it: the corner.
        assert_eq!(rail_place(678.0, 5, 500.0), RailPlace::Corner);
        // A window that never centres the rail keeps it in the corner.
        assert_eq!(rail_place(400.0, 5, 0.0), RailPlace::Corner);
    }

    #[test]
    fn the_rail_stands_in_the_middle_of_the_systems_band() {
        assert_eq!(rail_margin(0.0), RAIL_MARGIN);
        assert_eq!(rail_margin(RAIL_SPACE - 1.0), RAIL_MARGIN);
        assert_eq!(rail_margin(92.0), 16 + STATUS_ICON_SHIFT as u32);
        // Centred on the band, then moved onto the line of the system's icons.
        assert_eq!(
            rail_margin(92.0) as f32 - STATUS_ICON_SHIFT,
            (92.0 - RAIL_WIDTH) / 2.0
        );
    }

    #[test]
    fn the_page_clears_the_larger_of_the_rail_and_the_systems_band() {
        assert_eq!(rail_band(&SafeAreaInsets::ZERO), RAIL_SPACE);
        assert_eq!(rail_band(&SafeAreaInsets::new(0.0, 0.0, 92.0, 0.0)), 92.0);
        assert_eq!(
            rail_padding(SafeAreaInsets::new(3.0, 4.0, 92.0, 5.0)),
            LayoutSpacing::new().left(3u32).top(4u32).right(0u32).bottom(5u32)
        );
    }

    #[test]
    fn a_narrow_tall_window_is_a_phone() {
        assert_eq!(ShellLayout::classify(390.0, 844.0), ShellLayout::Phone);
        assert_eq!(ShellLayout::classify(450.0, 780.0), ShellLayout::Phone);
    }

    #[test]
    fn a_nearly_square_window_gets_the_rail_as_the_outer_display_of_a_duo() {
        assert_eq!(ShellLayout::classify(466.0, 678.0), ShellLayout::Rail);
        assert_eq!(ShellLayout::classify(390.0, 300.0), ShellLayout::Rail);
    }

    #[test]
    fn a_wide_but_short_window_gets_the_rail() {
        assert_eq!(ShellLayout::classify(740.0, 400.0), ShellLayout::Rail);
        assert_eq!(ShellLayout::classify(PHONE_MAX_WIDTH, SHORT_HEIGHT - 1.0), ShellLayout::Rail);
    }

    #[test]
    fn a_tall_or_very_wide_window_gets_the_sidebar() {
        assert_eq!(ShellLayout::classify(820.0, 900.0), ShellLayout::Desktop);
        assert_eq!(ShellLayout::classify(RAIL_MAX_WIDTH, 400.0), ShellLayout::Desktop);
        assert_eq!(ShellLayout::classify(1440.0, 900.0), ShellLayout::Desktop);
    }

    #[test]
    fn the_breakpoints_belong_to_the_larger_layout() {
        assert_eq!(ShellLayout::classify(PHONE_MAX_WIDTH - 0.5, 1200.0), ShellLayout::Phone);
        assert_eq!(ShellLayout::classify(PHONE_MAX_WIDTH, 900.0), ShellLayout::Desktop);
        assert_eq!(ShellLayout::classify(700.0, SHORT_HEIGHT), ShellLayout::Desktop);
    }

    #[test]
    fn only_the_desktop_layout_is_not_compact() {
        assert!(ShellLayout::Phone.is_compact());
        assert!(ShellLayout::Rail.is_compact());
        assert!(!ShellLayout::Desktop.is_compact());
    }

    #[test]
    fn without_a_fold_nothing_is_kept_clear() {
        assert_eq!(fold_insets(800.0, 600.0, None), SafeAreaInsets::ZERO);
    }

    #[test]
    fn a_fold_keeps_the_content_in_the_larger_pane() {
        let panes = Panes {
            first: Bounds::new(0.0, 0.0, 300.0, 600.0),
            second: Bounds::new(340.0, 0.0, 800.0, 600.0),
        };
        let insets = fold_insets(800.0, 600.0, Some(&panes));
        assert_eq!(insets, SafeAreaInsets::new(340.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn with_the_sidebar_a_fold_does_not_squeeze_the_page_into_the_pane_beside_it() {
        // The left pane ends just past the sidebar; the right one is the larger.
        let panes = Panes {
            first: Bounds::new(0.0, 0.0, 560.0, 900.0),
            second: Bounds::new(700.0, 0.0, 1300.0, 900.0),
        };
        let fold = fold_insets(1300.0, 900.0, Some(&panes));
        assert_eq!(sidebar_fold_insets(fold, true), SafeAreaInsets::ZERO);
        assert_eq!(sidebar_fold_insets(fold, false), fold);

        // A fold across the window still keeps the content above or below it.
        let panes = Panes {
            first: Bounds::new(0.0, 0.0, 1300.0, 400.0),
            second: Bounds::new(0.0, 440.0, 1300.0, 900.0),
        };
        let fold = fold_insets(1300.0, 900.0, Some(&panes));
        assert_eq!(sidebar_fold_insets(fold, true), SafeAreaInsets::new(0.0, 440.0, 0.0, 0.0));
    }

    #[test]
    fn panes_of_the_same_size_keep_the_leading_one() {
        let panes = Panes {
            first: Bounds::new(0.0, 0.0, 380.0, 600.0),
            second: Bounds::new(420.0, 0.0, 800.0, 600.0),
        };
        let insets = fold_insets(800.0, 600.0, Some(&panes));
        assert_eq!(insets, SafeAreaInsets::new(0.0, 0.0, 420.0, 0.0));
    }

    #[test]
    fn a_horizontal_fold_keeps_the_content_above_or_below_it() {
        let panes = Panes {
            first: Bounds::new(0.0, 0.0, 800.0, 200.0),
            second: Bounds::new(0.0, 240.0, 800.0, 600.0),
        };
        let insets = fold_insets(800.0, 600.0, Some(&panes));
        assert_eq!(insets, SafeAreaInsets::new(0.0, 240.0, 0.0, 0.0));
    }
}
