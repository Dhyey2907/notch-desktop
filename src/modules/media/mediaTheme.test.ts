import { describe, expect, it } from "vitest";

import { brandBackground, brandOf } from "./mediaTheme";

const m = (sourceApp: string, title = "", artist = "") => ({ sourceApp, title, artist, album: "" });

describe("media backgrounds", () => {
  it("recognises services by app, or by title in a browser", () => {
    expect(brandOf(m("Netflix", "Stranger Things"))?.name).toBe("Netflix");
    expect(brandOf(m("Microsoft Edge", "Stranger Things | Netflix"))?.name).toBe("Netflix");
    expect(brandOf(m("Prime Video"))?.name).toBe("Prime Video");
    expect(brandOf(m("Google Chrome", "The Boys - Prime Video"))?.name).toBe("Prime Video");
    expect(brandOf(m("Microsoft Edge", "Lofi beats - YouTube"))?.name).toBe("YouTube");
    expect(brandOf(m("Microsoft Edge", "Song", "YouTube Music"))?.name).toBe("YouTube Music");
    expect(brandOf(m("Spotify"))?.name).toBe("Spotify");
    expect(brandOf(m("Media Player", "Holiday video"))).toBeNull();
  });

  it("gives Netflix a red gradient and unknown apps a neutral one", () => {
    expect(brandBackground(brandOf(m("Netflix")))).toContain("#e50914");
    expect(brandBackground(null)).toContain("#5b6378");
  });
});
