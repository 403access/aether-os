export type ClientInputEvent =
  | { kind: "key"; scancode: number; pressed: boolean; modifiers: number }
  | { kind: "pointer-motion"; dx: number; dy: number }
  | { kind: "pointer-button"; button: number; pressed: boolean }
  | { kind: "wheel"; dx: number; dy: number }
  | { kind: "focus"; focused: boolean };

export const MAX_SCANCODE = 0x2ff;
export const MAX_MOTION_PER_EVENT = 2048;
export const DEFAULT_EVENTS_PER_SECOND = 500;

/** Clamp a signed pointer delta, rejecting NaN/Infinity instead of forwarding them. */
function clampDelta(value: number): number | undefined {
  if (!Number.isFinite(value)) return undefined;
  return Math.max(-MAX_MOTION_PER_EVENT, Math.min(MAX_MOTION_PER_EVENT, value));
}

/** Browser `KeyboardEvent.code` → Linux evdev scancode. */
export const SCANCODES: Readonly<Record<string, number>> = {
  Escape: 1,
  Digit1: 2,
  Digit2: 3,
  Digit3: 4,
  Digit4: 5,
  Digit5: 6,
  Digit6: 7,
  Digit7: 8,
  Digit8: 9,
  Digit9: 10,
  Digit0: 11,
  Minus: 12,
  Equal: 13,
  Backspace: 14,
  Tab: 15,
  KeyQ: 16,
  KeyW: 17,
  KeyE: 18,
  KeyR: 19,
  KeyT: 20,
  KeyY: 21,
  KeyU: 22,
  KeyI: 23,
  KeyO: 24,
  KeyP: 25,
  BracketLeft: 26,
  BracketRight: 27,
  Enter: 28,
  ControlLeft: 29,
  KeyA: 30,
  KeyS: 31,
  KeyD: 32,
  KeyF: 33,
  KeyG: 34,
  KeyH: 35,
  KeyJ: 36,
  KeyK: 37,
  KeyL: 38,
  Semicolon: 39,
  Quote: 40,
  Backquote: 41,
  ShiftLeft: 42,
  Backslash: 43,
  KeyZ: 44,
  KeyX: 45,
  KeyC: 46,
  KeyV: 47,
  KeyB: 48,
  KeyN: 49,
  KeyM: 50,
  Comma: 51,
  Period: 52,
  Slash: 53,
  ShiftRight: 54,
  NumpadMultiply: 55,
  AltLeft: 56,
  Space: 57,
  CapsLock: 58,
  F1: 59,
  F2: 60,
  F3: 61,
  F4: 62,
  F5: 63,
  F6: 64,
  F7: 65,
  F8: 66,
  F9: 67,
  F10: 68,
  NumLock: 69,
  ScrollLock: 70,
  Numpad7: 71,
  Numpad8: 72,
  Numpad9: 73,
  NumpadSubtract: 74,
  Numpad4: 75,
  Numpad5: 76,
  Numpad6: 77,
  NumpadAdd: 78,
  Numpad1: 79,
  Numpad2: 80,
  Numpad3: 81,
  Numpad0: 82,
  NumpadDecimal: 83,
  F11: 87,
  F12: 88,
  NumpadEnter: 96,
  ControlRight: 97,
  NumpadDivide: 98,
  PrintScreen: 99,
  AltRight: 100,
  Home: 102,
  ArrowUp: 103,
  PageUp: 104,
  ArrowLeft: 105,
  ArrowRight: 106,
  End: 107,
  ArrowDown: 108,
  PageDown: 109,
  Insert: 110,
  Delete: 111,
  MetaLeft: 125,
  MetaRight: 126,
  ContextMenu: 127,
};

export function scancodeForCode(code: string): number | undefined {
  return SCANCODES[code];
}

/**
 * Turns raw browser events into a bounded, coalesced input stream.
 *
 * Motion is accumulated until the next frame; repeated motion events never
 * consume the per-second budget. A viewer that loses focus stops sending
 * keyboard and pointer input until focus returns.
 */
export class InputBatcher {
  readonly maxPerSecond: number;
  #windowStart = 0;
  #inWindow = 0;
  #focused = false;
  #motion = { dx: 0, dy: 0 };
  #events: ClientInputEvent[] = [];
  #coalescedMotion = 0;
  #dropped = 0;
  #rejected = 0;

  constructor(maxPerSecond = DEFAULT_EVENTS_PER_SECOND) {
    this.maxPerSecond = Number.isFinite(maxPerSecond)
      ? Math.max(1, Math.trunc(maxPerSecond))
      : DEFAULT_EVENTS_PER_SECOND;
  }

  get focused(): boolean {
    return this.#focused;
  }

  setFocused(focused: boolean, nowMs: number): void {
    this.#focused = focused;
    if (!focused) this.#motion = { dx: 0, dy: 0 };
    this.#pushEvent({ kind: "focus", focused }, nowMs, true);
  }

  pushKey(
    code: string,
    pressed: boolean,
    modifiers = 0,
    nowMs = 0,
  ): boolean {
    const scancode = scancodeForCode(code);
    if (scancode === undefined) {
      this.#rejected += 1;
      return false;
    }
    return this.#pushEvent({ kind: "key", scancode, pressed, modifiers }, nowMs);
  }

  pushMotion(dx: number, dy: number, nowMs = 0): boolean {
    if (!this.#focused || !Number.isFinite(dx) || !Number.isFinite(dy)) {
      this.#rejected += 1;
      return false;
    }
    this.#motion.dx += dx;
    this.#motion.dy += dy;
    this.#coalescedMotion += 1;
    return true;
  }

  pushButton(button: number, pressed: boolean, nowMs = 0): boolean {
    if (!Number.isInteger(button) || button < 0 || button > 7) {
      this.#rejected += 1;
      return false;
    }
    return this.#pushEvent({ kind: "pointer-button", button, pressed }, nowMs);
  }

  pushWheel(dx: number, dy: number, nowMs = 0): boolean {
    const boundedX = clampDelta(dx);
    const boundedY = clampDelta(dy);
    if (boundedX === undefined || boundedY === undefined) {
      this.#rejected += 1;
      return false;
    }
    return this.#pushEvent({ kind: "wheel", dx: boundedX, dy: boundedY }, nowMs);
  }

  drain(): {
    events: ClientInputEvent[];
    coalescedMotion: number;
    rejected: number;
    dropped: number;
  } {
    this.#flushMotion();
    const events = this.#events;
    this.#events = [];
    const result = {
      events,
      coalescedMotion: this.#coalescedMotion,
      rejected: this.#rejected,
      dropped: this.#dropped,
    };
    this.#coalescedMotion = 0;
    this.#rejected = 0;
    this.#dropped = 0;
    return result;
  }

  #pushEvent(event: ClientInputEvent, nowMs: number, always = false): boolean {
    if (!always && !this.#focused) {
      this.#rejected += 1;
      return false;
    }
    if (!always && !this.#admitEvent(nowMs)) return false;
    this.#events.push(event);
    return true;
  }

  #admitEvent(nowMs: number): boolean {
    if (nowMs >= this.#windowStart + 1000) {
      this.#windowStart = nowMs;
      this.#inWindow = 0;
    }
    if (this.#inWindow >= this.maxPerSecond) {
      this.#dropped += 1;
      return false;
    }
    this.#inWindow += 1;
    return true;
  }

  #flushMotion(): void {
    if (this.#motion.dx === 0 && this.#motion.dy === 0) return;
    const dx = Math.max(-MAX_MOTION_PER_EVENT, Math.min(MAX_MOTION_PER_EVENT, this.#motion.dx));
    const dy = Math.max(-MAX_MOTION_PER_EVENT, Math.min(MAX_MOTION_PER_EVENT, this.#motion.dy));
    this.#motion = { dx: 0, dy: 0 };
    if (dx !== 0 || dy !== 0) this.#events.push({ kind: "pointer-motion", dx, dy });
  }
}
