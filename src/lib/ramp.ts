import type { Ramp } from "./types";

// Progress is carried by hue as well as length, so how close a run is reads
// from across the room: cool early, warm past the threshold, hot in the 90s.

type Rgb = [number, number, number];

function parse(hex: string): Rgb {
  const clean = hex.replace("#", "").trim();
  const full =
    clean.length === 3
      ? clean
          .split("")
          .map((c) => c + c)
          .join("")
      : clean;
  const value = Number.parseInt(full, 16);
  if (Number.isNaN(value) || full.length !== 6) return [255, 255, 255];
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}

function mix(a: Rgb, b: Rgb, t: number): Rgb {
  return [
    Math.round(a[0] + (b[0] - a[0]) * t),
    Math.round(a[1] + (b[1] - a[1]) * t),
    Math.round(a[2] + (b[2] - a[2]) * t),
  ];
}

/** Colour for a percentage, interpolated across the four ramp stops. */
export function rampColor(percent: number, ramp: Ramp): string {
  const stops: Array<{ at: number; rgb: Rgb }> = [
    { at: 0, rgb: parse(ramp.low) },
    { at: 45, rgb: parse(ramp.mid) },
    { at: 72, rgb: parse(ramp.high) },
    { at: 92, rgb: parse(ramp.peak) },
  ];

  const value = Math.min(Math.max(percent, 0), 100);
  let lower = stops[0] as { at: number; rgb: Rgb };
  let upper = stops[stops.length - 1] as { at: number; rgb: Rgb };

  for (let i = 0; i < stops.length - 1; i++) {
    const a = stops[i];
    const b = stops[i + 1];
    if (a && b && value >= a.at && value <= b.at) {
      lower = a;
      upper = b;
      break;
    }
  }

  const span = upper.at - lower.at;
  const t = span === 0 ? 0 : (value - lower.at) / span;
  const [r, g, b] = mix(lower.rgb, upper.rgb, t);
  return `rgb(${r} ${g} ${b})`;
}

/** Same ramp at reduced alpha, for the unfilled part of a bar. */
export function rampTrack(ramp: Ramp): string {
  const [r, g, b] = parse(ramp.low);
  return `rgb(${r} ${g} ${b} / 0.14)`;
}
