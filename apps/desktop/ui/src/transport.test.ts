// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { isWeb, serverFileName } from "./transport";

describe("serverFileName", () => {
  it("keeps a name the server accepts and replaces what it would refuse", () => {
    expect(serverFileName("My design 2.cut")).toBe("My design 2.cut");
    expect(serverFileName("café/logo (v2).png")).toBe("caf__logo _v2_.png");
    expect(serverFileName("..secret")).toBe("secret");
    expect(serverFileName("...")).toBe("file");
    expect(serverFileName("a".repeat(300))).toHaveLength(128);
  });
});

describe("isWeb", () => {
  it("is false unless the web server marked the page, so tests keep reaching the Tauri mock", () => {
    expect(isWeb()).toBe(false);
  });
});
