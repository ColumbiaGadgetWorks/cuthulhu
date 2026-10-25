// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, it, expect } from "vitest";
import { formatMm, scrubValue } from "./NumberField";

describe("scrubValue", () => {
  it("changes value by dx * step", () => {
    expect(scrubValue(10, 4, 0.5)).toBe(12); // 10 + 4*0.5
  });
  it("respects a min clamp", () => {
    expect(scrubValue(1, -100, 1, 0)).toBe(0);
  });
});

describe("formatMm", () => {
  it("shows at most two decimals", () => {
    expect(formatMm(39.27282321899738)).toBe(39.27);
    expect(formatMm(10)).toBe(10);
    expect(formatMm(0.005)).toBe(0.01);
    expect(formatMm(-2.499)).toBe(-2.5);
  });
});
