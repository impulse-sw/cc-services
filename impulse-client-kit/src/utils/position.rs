//! Calculate position utility.

/// Minimum gap kept between an overlay and the edge of the viewport.
const VIEWPORT_PADDING: f64 = 8.0;

/// Overlay side to show at.
#[derive(Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub enum OverlaySide {
  Top,
  Right,
  Bottom,
  Left,
}

/// Overlay align to show with.
#[derive(Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub enum OverlayAlign {
  Start,
  Center,
  End,
}

/// The side an overlay actually gets, given the room around the trigger.
///
/// The requested side when the content fits there; the opposite side when it
/// fits and the requested one doesn't; otherwise whichever of the two has more
/// room, so that the clamp below has the least to take back.
///
/// Flipping matters more than it looks: an overlay that doesn't fit used to be
/// clamped into the viewport, which for a trigger near an edge put it *on top
/// of its own trigger*. That takes the pointer off the trigger, and for
/// anything driven by hover it is a loop — the overlay closes because the
/// pointer "left" the trigger, which uncovers the trigger, which opens it
/// again, with the pointer never moving.
#[allow(clippy::too_many_arguments)]
fn fitting_side(
  side: OverlaySide,
  trigger_top: f64,
  trigger_left: f64,
  trigger_width: f64,
  trigger_height: f64,
  content_width: f64,
  content_height: f64,
  offset: f64,
  viewport_width: f64,
  viewport_height: f64,
) -> OverlaySide {
  let room = |side: OverlaySide| match side {
    OverlaySide::Top => trigger_top - offset - VIEWPORT_PADDING,
    OverlaySide::Bottom => viewport_height - VIEWPORT_PADDING - (trigger_top + trigger_height + offset),
    OverlaySide::Left => trigger_left - offset - VIEWPORT_PADDING,
    OverlaySide::Right => viewport_width - VIEWPORT_PADDING - (trigger_left + trigger_width + offset),
  };
  let opposite = match side {
    OverlaySide::Top => OverlaySide::Bottom,
    OverlaySide::Bottom => OverlaySide::Top,
    OverlaySide::Left => OverlaySide::Right,
    OverlaySide::Right => OverlaySide::Left,
  };
  let needed = match side {
    OverlaySide::Top | OverlaySide::Bottom => content_height,
    OverlaySide::Left | OverlaySide::Right => content_width,
  };

  if room(side) >= needed || room(opposite) < room(side) {
    side
  } else {
    opposite
  }
}

/// Calculates overlay position based on given positions, side and align, then
/// clamps the result so the overlay stays fully within the viewport.
///
/// The requested side is a preference, not a promise: an overlay that would not
/// fit there is flipped to the opposite side (see [`fitting_side`]) rather than
/// squeezed back over its own trigger. Only the position flips — a caller that
/// keys an entrance animation or an arrow off the side it asked for still draws
/// the side it asked for.
#[allow(clippy::too_many_arguments)]
pub fn calculate_position(
  trigger_top: f64,
  trigger_left: f64,
  trigger_width: f64,
  trigger_height: f64,
  content_width: f64,
  content_height: f64,
  side: OverlaySide,
  align: OverlayAlign,
  side_offset: i32,
  viewport_width: f64,
  viewport_height: f64,
) -> (f64, f64) {
  let offset = side_offset as f64;
  let side = fitting_side(
    side,
    trigger_top,
    trigger_left,
    trigger_width,
    trigger_height,
    content_width,
    content_height,
    offset,
    viewport_width,
    viewport_height,
  );

  let (mut top, mut left) = match side {
    OverlaySide::Top => (trigger_top - content_height - offset, trigger_left),
    OverlaySide::Bottom => (trigger_top + trigger_height + offset, trigger_left),
    OverlaySide::Left => (trigger_top, trigger_left - content_width - offset),
    OverlaySide::Right => (trigger_top, trigger_left + trigger_width + offset),
  };

  match side {
    OverlaySide::Top | OverlaySide::Bottom => {
      left += match align {
        OverlayAlign::Start => 0.0,
        OverlayAlign::Center => (trigger_width - content_width) / 2.0,
        OverlayAlign::End => trigger_width - content_width,
      };
    }
    OverlaySide::Left | OverlaySide::Right => {
      top += match align {
        OverlayAlign::Start => 0.0,
        OverlayAlign::Center => (trigger_height - content_height) / 2.0,
        OverlayAlign::End => trigger_height - content_height,
      };
    }
  }

  clamp_to_viewport(
    top,
    left,
    content_width,
    content_height,
    viewport_width,
    viewport_height,
  )
}

/// Clamps a `top`/`left` position so a `content_width` x `content_height` box
/// stays fully inside the viewport, leaving a small edge padding.
///
/// If the content itself is larger than the viewport (minus padding), it is
/// pinned to the padded edge rather than centered off-screen.
pub fn clamp_to_viewport(
  top: f64,
  left: f64,
  content_width: f64,
  content_height: f64,
  viewport_width: f64,
  viewport_height: f64,
) -> (f64, f64) {
  let max_left = (viewport_width - content_width - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
  let max_top = (viewport_height - content_height - VIEWPORT_PADDING).max(VIEWPORT_PADDING);

  (
    top.clamp(VIEWPORT_PADDING, max_top),
    left.clamp(VIEWPORT_PADDING, max_left),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clamps_left_edge() {
    let (top, left) = calculate_position(
      100.0,
      -50.0,
      30.0,
      20.0,
      200.0,
      100.0,
      OverlaySide::Bottom,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(left, VIEWPORT_PADDING);
    assert_eq!(top, 124.0);
  }

  #[test]
  fn clamps_right_edge() {
    let (_, left) = calculate_position(
      100.0,
      1000.0,
      30.0,
      20.0,
      200.0,
      100.0,
      OverlaySide::Bottom,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(left, 1024.0 - 200.0 - VIEWPORT_PADDING);
  }

  /// A trigger near the bottom edge: the content goes *above* it rather than
  /// being clamped down onto it. Covering the trigger is what makes a hover
  /// card flicker — it takes the pointer off the thing that opened it.
  #[test]
  fn flips_to_the_side_that_fits() {
    let (top, _) = calculate_position(
      700.0,
      100.0,
      30.0,
      20.0,
      100.0,
      300.0,
      OverlaySide::Bottom,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(top, 700.0 - 300.0 - 4.0);
  }

  /// Neither side has room: the one with more of it wins, so the clamp has the
  /// least to take back — and the overlap, if any, is as small as it can be.
  #[test]
  fn keeps_the_roomier_side_when_neither_fits() {
    // 200 above the trigger, ~468 below it, content 600 tall: below stays.
    let (top, _) = calculate_position(
      200.0,
      100.0,
      30.0,
      100.0,
      100.0,
      600.0,
      OverlaySide::Top,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(top, 768.0 - 600.0 - VIEWPORT_PADDING);
  }

  /// A side that fits is never abandoned, even when the other side has more
  /// room: the caller asked for this one.
  #[test]
  fn keeps_the_requested_side_when_it_fits() {
    let (top, _) = calculate_position(
      400.0,
      100.0,
      30.0,
      20.0,
      100.0,
      100.0,
      OverlaySide::Top,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(top, 400.0 - 100.0 - 4.0);
  }

  #[test]
  fn oversized_content_pins_to_padded_edge() {
    let (top, left) = clamp_to_viewport(-500.0, -500.0, 2000.0, 2000.0, 1024.0, 768.0);
    assert_eq!(top, VIEWPORT_PADDING);
    assert_eq!(left, VIEWPORT_PADDING);
  }

  #[test]
  fn keeps_position_when_within_viewport() {
    let (top, left) = calculate_position(
      100.0,
      100.0,
      30.0,
      20.0,
      50.0,
      40.0,
      OverlaySide::Bottom,
      OverlayAlign::Start,
      4,
      1024.0,
      768.0,
    );
    assert_eq!(top, 124.0);
    assert_eq!(left, 100.0);
  }
}
