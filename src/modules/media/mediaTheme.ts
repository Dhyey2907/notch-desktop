// The colors behind the Media panel: the album's own colors when there's artwork, otherwise the
// playing service's brand colors (Netflix red, Prime Video blue, …), matched on the app name and,
// for video playing in a browser, on the title too.
import type { MediaState } from "../../core/generated/MediaState";

export interface Brand {
  name: string;
  /** A video service (Netflix, YouTube…) rather than music. */
  video?: boolean;
  /** Two gradient stops. */
  colors: [string, string];
}

const BRANDS: { match: RegExp; brand: Brand }[] = [
  { match: /netflix/i, brand: { name: "Netflix", video: true, colors: ["#e50914", "#5c0307"] } },
  {
    match: /prime video|primevideo|amazon video/i,
    brand: { name: "Prime Video", video: true, colors: ["#1a98ff", "#0b2a5c"] },
  },
  {
    match: /disney\+|disneyplus|hotstar/i,
    brand: { name: "Disney+", video: true, colors: ["#1f80e0", "#0b1a4a"] },
  },
  { match: /youtube music/i, brand: { name: "YouTube Music", colors: ["#ff0033", "#3d0008"] } },
  { match: /youtube/i, brand: { name: "YouTube", video: true, colors: ["#ff0000", "#420000"] } },
  { match: /apple (music|tv)|itunes/i, brand: { name: "Apple", colors: ["#fa2d48", "#4a0b15"] } },
  { match: /spotify/i, brand: { name: "Spotify", colors: ["#1ed760", "#0a3d1d"] } },
  { match: /twitch/i, brand: { name: "Twitch", video: true, colors: ["#9146ff", "#2a0f55"] } },
  { match: /soundcloud/i, brand: { name: "SoundCloud", colors: ["#ff5500", "#4a1800"] } },
  { match: /\bhulu\b/i, brand: { name: "Hulu", video: true, colors: ["#1ce783", "#073d23"] } },
  { match: /\bmax\b|hbo/i, brand: { name: "Max", video: true, colors: ["#3e2bff", "#120b4d"] } },
  { match: /crunchyroll/i, brand: { name: "Crunchyroll", video: true, colors: ["#f47521", "#4a2007"] } },
  { match: /\bvlc\b/i, brand: { name: "VLC", colors: ["#ff8800", "#4a2800"] } },
  {
    match: /jiocinema|jio cinema/i,
    brand: { name: "JioCinema", video: true, colors: ["#d4148c", "#3d0628"] },
  },
];

/** The service playing, from the app name and (for browsers) the title. */
export function brandOf(media: Pick<MediaState, "sourceApp" | "title" | "artist" | "album">): Brand | null {
  const app = media.sourceApp ?? "";
  const byApp = BRANDS.find((b) => b.match.test(app));
  if (byApp) return byApp.brand;
  // A browser tab: "Stranger Things | Netflix", "… - YouTube".
  const text = [media.title, media.artist, media.album].filter(Boolean).join(" ");
  return BRANDS.find((b) => b.match.test(text))?.brand ?? null;
}

/** CSS background for the panel when there's no artwork. */
export function brandBackground(brand: Brand | null): string {
  const [a, b] = brand?.colors ?? ["#5b6378", "#1a1d26"];
  return `radial-gradient(120% 140% at 0% 0%, ${a} 0%, transparent 60%), radial-gradient(120% 140% at 100% 100%, ${b} 0%, transparent 70%), linear-gradient(135deg, ${b}, #0b0b0d)`;
}

/** CSS background drawn over the blurred artwork, from its two main colors. */
export function paletteBackground(primary: string, accent: string): string {
  return `radial-gradient(110% 130% at 0% 0%, ${accent}cc 0%, transparent 60%), radial-gradient(120% 140% at 100% 100%, ${primary}cc 0%, transparent 65%)`;
}
