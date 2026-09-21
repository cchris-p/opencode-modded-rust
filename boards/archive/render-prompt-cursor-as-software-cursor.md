---
id: "FEAT-034"
title: "Render prompt cursor as a software cursor"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "archive"
created: "2026-09-21"
---

# Render prompt cursor as a software cursor

## Summary

If the prompt cursor remains visually unreliable after the concrete off-by-one issue in `BUG-018` is isolated, replace the visible hardware-terminal cursor with a cursor rendered directly into the prompt text buffer.

This item is archived intentionally. It records a fallback strategy, not the current active fix direction.

## Problem

The current prompt cursor model renders prompt text into the ratatui buffer, then places the terminal cursor afterward with `frame.set_cursor(x, y)`. That split can be fragile around wrapped input, prompt scrolling, exact line-width boundaries, and terminal-specific cursor rendering.

`BUG-018` remains focused on the current observed off-by-one behavior. This item should only be revived if the remaining behavior proves structural rather than a single fixable layout mismatch.

## Proposed Direction

- Keep wrapped prompt lines as the single source of truth for layout.
- Render the visible cursor as part of the prompt `Line`/`Span` content.
- Use a highlighted/reversed grapheme when the cursor is before a grapheme.
- Render a highlighted space cell when the cursor is at end-of-line, at the end of input, or on a phantom wrap row.
- Hide or de-emphasize reliance on the hardware terminal cursor for visual placement.
- Test the rendered buffer directly for cursor styling across wrap, scroll, full-width boundary, CJK, combining grapheme, and multiline cases.

## Revival Criteria

Revive this item only if investigation of `BUG-018` shows either:

- the off-by-one appears across multiple unrelated wrap/scroll/edge cases, or
- the internal insertion point is correct while the visible hardware cursor remains unreliable despite correct layout math.

## Related Items

- `BUG-018` Prompt input cursor renders out of place in the TUI - active focus remains fixing the concrete off-by-one behavior first.
- `BUG-013` Cursor on the input field needs to always be visible - prior visibility requirement for the prompt cursor.
