// Clock text that follows Settings → Home "24-hour clock".

/** "6:12 pm" / "18:12"; `ampm: false` drops the am/pm suffix to save room. */
export function formatClock(date: Date, h24: boolean, ampm = true): string {
  if (h24) {
    return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
  }
  const h = date.getHours() % 12 || 12;
  const text = `${h}:${String(date.getMinutes()).padStart(2, "0")}`;
  return ampm ? `${text} ${date.getHours() < 12 ? "am" : "pm"}` : text;
}

/** °C → °F when asked for. */
export function formatTemperature(celsius: number, fahrenheit: boolean): string {
  return fahrenheit ? `${Math.round((celsius * 9) / 5 + 32)}°F` : `${Math.round(celsius)}°C`;
}
